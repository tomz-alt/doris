//! Bucket Shuffle Optimization
//!
//! Optimizes shuffle operations for bucketed tables by colocating data
//! based on bucket IDs, reducing data movement

use super::plan_fragment::*;
use tracing::{debug, info};

/// Bucket information for a table
#[derive(Debug, Clone)]
pub struct BucketInfo {
    pub table_name: String,
    pub bucket_column: String,
    pub num_buckets: u32,
    pub bucket_type: BucketType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BucketType {
    Hash,
    Random,
}

/// Bucket shuffle optimizer
pub struct BucketShuffleOptimizer;

impl BucketShuffleOptimizer {
    /// Check if two tables can be colocated based on bucket information
    pub fn can_colocate(left: &BucketInfo, right: &BucketInfo) -> bool {
        // Tables can be colocated if:
        // 1. Same number of buckets
        // 2. Both use hash bucketing
        // 3. Join on bucket columns
        left.num_buckets == right.num_buckets
            && left.bucket_type == BucketType::Hash
            && right.bucket_type == BucketType::Hash
    }

    /// Optimize join by using bucket colocation
    pub fn optimize_join(
        join_node: &PlanNode,
        left_bucket_info: Option<&BucketInfo>,
        right_bucket_info: Option<&BucketInfo>,
    ) -> (PlanNode, bool) {
        match join_node {
            PlanNode::HashJoin {
                left,
                right,
                join_type,
                join_predicates,
            } => {
                // Check if tables can be colocated
                if let (Some(left_info), Some(right_info)) =
                    (left_bucket_info, right_bucket_info)
                {
                    if Self::can_colocate(left_info, right_info)
                        && Self::join_on_bucket_columns(join_predicates, left_info, right_info)
                    {
                        info!(
                            "Bucket shuffle optimization: colocating {} and {}",
                            left_info.table_name, right_info.table_name
                        );

                        // No exchange needed - data is already colocated
                        return (join_node.clone(), true);
                    }
                }

                // Cannot colocate - use standard shuffle
                (join_node.clone(), false)
            }
            _ => (join_node.clone(), false),
        }
    }

    /// Check if join predicates use bucket columns
    fn join_on_bucket_columns(
        predicates: &[Expr],
        left_info: &BucketInfo,
        right_info: &BucketInfo,
    ) -> bool {
        for predicate in predicates {
            if let Expr::BinaryOp {
                op: BinaryOperator::Eq,
                left,
                right,
            } = predicate
            {
                if let (Some(left_col), Some(right_col)) =
                    (Self::extract_column(left), Self::extract_column(right))
                {
                    if (left_col.contains(&left_info.bucket_column)
                        && right_col.contains(&right_info.bucket_column))
                        || (left_col.contains(&right_info.bucket_column)
                            && right_col.contains(&left_info.bucket_column))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn extract_column(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Column { name, .. } => Some(name.clone()),
            Expr::Cast { expr, .. } => Self::extract_column(expr),
            _ => None,
        }
    }

    /// Create bucket-aware exchange node
    pub fn create_bucket_exchange(
        child: Box<PlanNode>,
        bucket_info: &BucketInfo,
        target_buckets: u32,
    ) -> PlanNode {
        debug!(
            "Creating bucket exchange for {} ({} buckets)",
            bucket_info.table_name, bucket_info.num_buckets
        );

        // If target buckets match source buckets, can avoid shuffle
        if target_buckets == bucket_info.num_buckets {
            return *child;
        }

        // Otherwise, create hash partition exchange on bucket column
        PlanNode::Exchange {
            child,
            exchange_type: ExchangeType::HashPartition {
                partition_keys: vec![Expr::Column {
                    name: bucket_info.bucket_column.clone(),
                    data_type: DataType::Int,
                }],
            },
        }
    }

    /// Analyze scan node to extract bucket information
    pub fn get_bucket_info(scan: &PlanNode) -> Option<BucketInfo> {
        match scan {
            PlanNode::OlapScan { table_name, .. } => {
                // TODO: Query metadata catalog for actual bucket info
                // For now, use heuristics based on table name
                if table_name.contains("bucketed") || table_name.contains("distributed") {
                    Some(BucketInfo {
                        table_name: table_name.clone(),
                        bucket_column: "id".to_string(), // Simplified
                        num_buckets: 16,
                        bucket_type: BucketType::Hash,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Calculate optimal bucket count for aggregation
    pub fn optimal_bucket_count(
        input_buckets: u32,
        num_executors: usize,
        parallelism_factor: usize,
    ) -> u32 {
        // Target: num_executors * parallelism_factor buckets
        let target = (num_executors * parallelism_factor) as u32;

        // If input is already well-distributed, keep it
        if input_buckets >= target {
            input_buckets
        } else {
            // Round up to power of 2 for better distribution
            target.next_power_of_two()
        }
    }

    /// Check if aggregation can use bucket shuffle
    pub fn can_use_bucket_agg(
        agg_node: &PlanNode,
        child_bucket_info: Option<&BucketInfo>,
    ) -> bool {
        match agg_node {
            PlanNode::Aggregation {
                group_by_exprs, ..
            } => {
                if let Some(bucket_info) = child_bucket_info {
                    // Check if grouping on bucket column
                    for expr in group_by_exprs {
                        if let Expr::Column { name, .. } = expr {
                            if name.contains(&bucket_info.bucket_column) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_colocate() {
        let left = BucketInfo {
            table_name: "table1".to_string(),
            bucket_column: "id".to_string(),
            num_buckets: 16,
            bucket_type: BucketType::Hash,
        };

        let right = BucketInfo {
            table_name: "table2".to_string(),
            bucket_column: "id".to_string(),
            num_buckets: 16,
            bucket_type: BucketType::Hash,
        };

        assert!(BucketShuffleOptimizer::can_colocate(&left, &right));

        let right_different = BucketInfo {
            table_name: "table2".to_string(),
            bucket_column: "id".to_string(),
            num_buckets: 32,
            bucket_type: BucketType::Hash,
        };

        assert!(!BucketShuffleOptimizer::can_colocate(&left, &right_different));
    }

    #[test]
    fn test_optimal_bucket_count() {
        assert_eq!(BucketShuffleOptimizer::optimal_bucket_count(8, 10, 2), 32);
        assert_eq!(BucketShuffleOptimizer::optimal_bucket_count(32, 10, 2), 32);
        assert_eq!(BucketShuffleOptimizer::optimal_bucket_count(64, 10, 2), 64);
    }

    #[test]
    fn test_join_on_bucket_columns() {
        let left_info = BucketInfo {
            table_name: "t1".to_string(),
            bucket_column: "id".to_string(),
            num_buckets: 16,
            bucket_type: BucketType::Hash,
        };

        let right_info = BucketInfo {
            table_name: "t2".to_string(),
            bucket_column: "user_id".to_string(),
            num_buckets: 16,
            bucket_type: BucketType::Hash,
        };

        let predicates = vec![Expr::BinaryOp {
            op: BinaryOperator::Eq,
            left: Box::new(Expr::Column {
                name: "t1.id".to_string(),
                data_type: DataType::Int,
            }),
            right: Box::new(Expr::Column {
                name: "t2.user_id".to_string(),
                data_type: DataType::Int,
            }),
        }];

        assert!(BucketShuffleOptimizer::join_on_bucket_columns(
            &predicates,
            &left_info,
            &right_info
        ));
    }
}
