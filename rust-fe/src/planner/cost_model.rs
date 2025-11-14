//! Cost-based Query Optimization
//!
//! Provides cost estimation for different execution strategies

use super::plan_fragment::*;
use tracing::debug;

/// Statistics for a table or intermediate result
#[derive(Debug, Clone)]
pub struct TableStatistics {
    pub row_count: u64,
    pub total_bytes: u64,
    pub column_stats: Vec<ColumnStatistics>,
}

#[derive(Debug, Clone)]
pub struct ColumnStatistics {
    pub column_name: String,
    pub distinct_count: u64,
    pub null_count: u64,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
}

impl TableStatistics {
    /// Create statistics with estimated values
    pub fn estimated(row_count: u64, avg_row_bytes: u64) -> Self {
        Self {
            row_count,
            total_bytes: row_count * avg_row_bytes,
            column_stats: vec![],
        }
    }

    /// Check if table is small enough for broadcast
    pub fn is_broadcast_eligible(&self, threshold_bytes: u64) -> bool {
        self.total_bytes < threshold_bytes
    }
}

/// Cost model for join strategies
pub struct JoinCostModel {
    /// Cost of broadcasting data (network + serialization)
    broadcast_cost_per_byte: f64,
    /// Cost of shuffling data (hash + network)
    shuffle_cost_per_byte: f64,
    /// Number of executor nodes
    num_executors: usize,
}

impl JoinCostModel {
    pub fn new(num_executors: usize) -> Self {
        Self {
            broadcast_cost_per_byte: 2.0,  // Higher cost due to replication
            shuffle_cost_per_byte: 1.5,     // Lower cost, no replication
            num_executors,
        }
    }

    /// Estimate cost of broadcast join
    pub fn estimate_broadcast_join(
        &self,
        build_side_stats: &TableStatistics,
        probe_side_stats: &TableStatistics,
    ) -> f64 {
        // Broadcast build side to all executors
        let broadcast_cost = build_side_stats.total_bytes as f64
            * self.broadcast_cost_per_byte
            * self.num_executors as f64;

        // Process probe side locally (no shuffle)
        let probe_cost = probe_side_stats.total_bytes as f64 * 0.1;

        // Hash build cost (creating hash table on each executor)
        let hash_build_cost = build_side_stats.row_count as f64 * self.num_executors as f64;

        broadcast_cost + probe_cost + hash_build_cost
    }

    /// Estimate cost of shuffle (partitioned) join
    pub fn estimate_shuffle_join(
        &self,
        build_side_stats: &TableStatistics,
        probe_side_stats: &TableStatistics,
    ) -> f64 {
        // Shuffle both sides
        let build_shuffle_cost = build_side_stats.total_bytes as f64 * self.shuffle_cost_per_byte;
        let probe_shuffle_cost = probe_side_stats.total_bytes as f64 * self.shuffle_cost_per_byte;

        // Hash build cost (distributed across executors)
        let hash_build_cost = build_side_stats.row_count as f64;

        // Probe cost (distributed)
        let probe_cost = probe_side_stats.row_count as f64 * 0.5;

        build_shuffle_cost + probe_shuffle_cost + hash_build_cost + probe_cost
    }

    /// Choose optimal join strategy based on cost
    pub fn choose_join_strategy(
        &self,
        left_stats: &TableStatistics,
        right_stats: &TableStatistics,
        broadcast_threshold: u64,
    ) -> JoinStrategy {
        // If right side is small, always broadcast
        if right_stats.is_broadcast_eligible(broadcast_threshold) {
            debug!(
                "Choosing BROADCAST join: right side {} bytes < threshold {} bytes",
                right_stats.total_bytes, broadcast_threshold
            );
            return JoinStrategy::Broadcast {
                broadcast_side: JoinSide::Right,
            };
        }

        // If left side is small, broadcast left
        if left_stats.is_broadcast_eligible(broadcast_threshold) {
            debug!(
                "Choosing BROADCAST join: left side {} bytes < threshold {} bytes",
                left_stats.total_bytes, broadcast_threshold
            );
            return JoinStrategy::Broadcast {
                broadcast_side: JoinSide::Left,
            };
        }

        // Both sides are large - compare costs
        let broadcast_right_cost = self.estimate_broadcast_join(right_stats, left_stats);
        let broadcast_left_cost = self.estimate_broadcast_join(left_stats, right_stats);
        let shuffle_cost = self.estimate_shuffle_join(left_stats, right_stats);

        debug!(
            "Cost comparison: broadcast_right={:.2}, broadcast_left={:.2}, shuffle={:.2}",
            broadcast_right_cost, broadcast_left_cost, shuffle_cost
        );

        // Choose minimum cost strategy
        if broadcast_right_cost < broadcast_left_cost && broadcast_right_cost < shuffle_cost {
            JoinStrategy::Broadcast {
                broadcast_side: JoinSide::Right,
            }
        } else if broadcast_left_cost < shuffle_cost {
            JoinStrategy::Broadcast {
                broadcast_side: JoinSide::Left,
            }
        } else {
            JoinStrategy::Shuffle {
                partition_keys: vec![], // Will be filled in by planner
            }
        }
    }
}

/// Join execution strategy
#[derive(Debug, Clone, PartialEq)]
pub enum JoinStrategy {
    /// Broadcast one side to all nodes
    Broadcast { broadcast_side: JoinSide },
    /// Shuffle (partition) both sides
    Shuffle { partition_keys: Vec<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum JoinSide {
    Left,
    Right,
}

/// Estimate statistics from plan node (simplified)
pub fn estimate_node_statistics(node: &PlanNode) -> TableStatistics {
    match node {
        PlanNode::OlapScan { table_name, .. } => {
            // TODO: Get real statistics from metadata catalog
            // For now, use heuristics based on table name
            if table_name.contains("dim_") || table_name.contains("small") {
                TableStatistics::estimated(1000, 100) // Small dimension table
            } else if table_name.contains("fact_") || table_name.contains("large") {
                TableStatistics::estimated(10_000_000, 200) // Large fact table
            } else {
                TableStatistics::estimated(100_000, 150) // Medium table
            }
        }
        PlanNode::Aggregation { child, .. } => {
            let child_stats = estimate_node_statistics(child);
            // Aggregation typically reduces rows
            TableStatistics::estimated(
                child_stats.row_count / 10,
                child_stats.total_bytes / child_stats.row_count.max(1),
            )
        }
        PlanNode::Select { child, .. } => {
            let child_stats = estimate_node_statistics(child);
            // Selection reduces rows by ~50% (heuristic)
            TableStatistics::estimated(
                child_stats.row_count / 2,
                child_stats.total_bytes / child_stats.row_count.max(1),
            )
        }
        PlanNode::Project { child, .. } => {
            let child_stats = estimate_node_statistics(child);
            // Projection may reduce bytes per row
            TableStatistics::estimated(child_stats.row_count, 100)
        }
        _ => TableStatistics::estimated(10_000, 100),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcast_eligibility() {
        let small_table = TableStatistics::estimated(1000, 100);
        assert!(small_table.is_broadcast_eligible(1_000_000));
        assert_eq!(small_table.total_bytes, 100_000);

        let large_table = TableStatistics::estimated(1_000_000, 200);
        assert!(!large_table.is_broadcast_eligible(1_000_000));
    }

    #[test]
    fn test_join_strategy_selection() {
        let cost_model = JoinCostModel::new(10);

        // Small right side -> broadcast
        let left = TableStatistics::estimated(1_000_000, 200);
        let right = TableStatistics::estimated(100, 100);
        let strategy = cost_model.choose_join_strategy(&left, &right, 10_000_000);
        assert_eq!(
            strategy,
            JoinStrategy::Broadcast {
                broadcast_side: JoinSide::Right
            }
        );

        // Small left side -> broadcast left
        let left = TableStatistics::estimated(100, 100);
        let right = TableStatistics::estimated(1_000_000, 200);
        let strategy = cost_model.choose_join_strategy(&left, &right, 10_000_000);
        assert_eq!(
            strategy,
            JoinStrategy::Broadcast {
                broadcast_side: JoinSide::Left
            }
        );

        // Both large -> shuffle
        let left = TableStatistics::estimated(10_000_000, 200);
        let right = TableStatistics::estimated(10_000_000, 200);
        let strategy = cost_model.choose_join_strategy(&left, &right, 1_000_000);
        assert!(matches!(strategy, JoinStrategy::Shuffle { .. }));
    }
}
