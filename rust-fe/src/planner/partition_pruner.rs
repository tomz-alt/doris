//! Advanced Partition Pruning
//!
//! Analyzes query predicates to prune unnecessary partitions before execution

use super::plan_fragment::*;
use tracing::{debug, info};

/// Partition information for a table
#[derive(Debug, Clone)]
pub struct PartitionMetadata {
    pub partition_id: i64,
    pub partition_name: String,
    pub partition_type: TablePartitionType,
    pub partition_values: PartitionValues,
}

#[derive(Debug, Clone)]
pub enum TablePartitionType {
    Range,
    List,
    Hash,
}

#[derive(Debug, Clone)]
pub enum PartitionValues {
    /// Range partition: (min, max) values
    Range { min: String, max: String },
    /// List partition: specific values
    List { values: Vec<String> },
    /// Hash partition: bucket ID
    Hash { bucket_id: u32 },
}

/// Partition pruning result
#[derive(Debug, Clone)]
pub struct PruningResult {
    pub retained_partitions: Vec<i64>,
    pub pruned_partitions: Vec<i64>,
    pub pruning_ratio: f64,
}

impl PruningResult {
    pub fn new(retained: Vec<i64>, pruned: Vec<i64>) -> Self {
        let total = retained.len() + pruned.len();
        let pruning_ratio = if total > 0 {
            pruned.len() as f64 / total as f64
        } else {
            0.0
        };

        Self {
            retained_partitions: retained,
            pruned_partitions: pruned,
            pruning_ratio,
        }
    }
}

/// Partition pruner
pub struct PartitionPruner;

impl PartitionPruner {
    /// Prune partitions based on query predicates
    pub fn prune_partitions(
        partitions: &[PartitionMetadata],
        predicates: &[Expr],
        partition_column: &str,
    ) -> PruningResult {
        if partitions.is_empty() || predicates.is_empty() {
            return PruningResult::new(
                partitions.iter().map(|p| p.partition_id).collect(),
                vec![],
            );
        }

        let mut retained = Vec::new();
        let mut pruned = Vec::new();

        for partition in partitions {
            if Self::can_satisfy_predicates(partition, predicates, partition_column) {
                retained.push(partition.partition_id);
            } else {
                pruned.push(partition.partition_id);
            }
        }

        debug!(
            "Partition pruning: retained {}, pruned {} ({}% reduction)",
            retained.len(),
            pruned.len(),
            (pruned.len() as f64 / partitions.len() as f64 * 100.0) as i32
        );

        PruningResult::new(retained, pruned)
    }

    /// Check if a partition can satisfy any of the predicates
    fn can_satisfy_predicates(
        partition: &PartitionMetadata,
        predicates: &[Expr],
        partition_column: &str,
    ) -> bool {
        // Extract partition-related predicates
        let partition_predicates: Vec<_> = predicates
            .iter()
            .filter(|p| Self::references_column(p, partition_column))
            .collect();

        if partition_predicates.is_empty() {
            // No partition predicates -> cannot prune
            return true;
        }

        // Check if partition can satisfy predicates
        match &partition.partition_values {
            PartitionValues::Range { min, max } => {
                Self::check_range_predicates(min, max, &partition_predicates)
            }
            PartitionValues::List { values } => {
                Self::check_list_predicates(values, &partition_predicates)
            }
            PartitionValues::Hash { .. } => {
                // Hash partitions cannot be easily pruned based on values
                true
            }
        }
    }

    /// Check if range partition can satisfy predicates
    fn check_range_predicates(min: &str, max: &str, predicates: &[&Expr]) -> bool {
        for predicate in predicates {
            match predicate {
                Expr::BinaryOp {
                    op: BinaryOperator::Lt,
                    right,
                    ..
                } => {
                    // col < value: keep if min < value
                    if let Some(value) = Self::extract_literal(right) {
                        if min >= value.as_str() {
                            return false; // Partition min >= predicate value, prune
                        }
                    }
                }
                Expr::BinaryOp {
                    op: BinaryOperator::LtEq,
                    right,
                    ..
                } => {
                    // col <= value: keep if min <= value
                    if let Some(value) = Self::extract_literal(right) {
                        if min > value.as_str() {
                            return false;
                        }
                    }
                }
                Expr::BinaryOp {
                    op: BinaryOperator::Gt,
                    right,
                    ..
                } => {
                    // col > value: keep if max > value
                    if let Some(value) = Self::extract_literal(right) {
                        if max <= value.as_str() {
                            return false;
                        }
                    }
                }
                Expr::BinaryOp {
                    op: BinaryOperator::GtEq,
                    right,
                    ..
                } => {
                    // col >= value: keep if max >= value
                    if let Some(value) = Self::extract_literal(right) {
                        if max < value.as_str() {
                            return false;
                        }
                    }
                }
                Expr::BinaryOp {
                    op: BinaryOperator::Eq,
                    right,
                    ..
                } => {
                    // col = value: keep if min <= value < max
                    if let Some(value) = Self::extract_literal(right) {
                        if value.as_str() < min || value.as_str() >= max {
                            return false;
                        }
                    }
                }
                _ => {}
            }
        }

        true
    }

    /// Check if list partition can satisfy predicates
    fn check_list_predicates(values: &[String], predicates: &[&Expr]) -> bool {
        for predicate in predicates {
            match predicate {
                Expr::BinaryOp {
                    op: BinaryOperator::Eq,
                    right,
                    ..
                } => {
                    // col = value: keep if value in partition values
                    if let Some(value) = Self::extract_literal(right) {
                        if !values.contains(&value) {
                            return false;
                        }
                    }
                }
                Expr::BinaryOp {
                    op: BinaryOperator::NotEq,
                    right,
                    ..
                } => {
                    // col != value: keep if partition has other values
                    if let Some(value) = Self::extract_literal(right) {
                        if values.len() == 1 && values[0] == value {
                            return false;
                        }
                    }
                }
                _ => {}
            }
        }

        true
    }

    /// Check if expression references a column
    fn references_column(expr: &Expr, column_name: &str) -> bool {
        match expr {
            Expr::Column { name, .. } => name == column_name,
            Expr::BinaryOp { left, right, .. } => {
                Self::references_column(left, column_name)
                    || Self::references_column(right, column_name)
            }
            Expr::UnaryOp { expr, .. } => Self::references_column(expr, column_name),
            Expr::Cast { expr, .. } => Self::references_column(expr, column_name),
            Expr::Function { args, .. } => {
                args.iter().any(|arg| Self::references_column(arg, column_name))
            }
            _ => false,
        }
    }

    /// Extract literal value from expression
    fn extract_literal(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Literal { value, .. } => Some(match value {
                LiteralValue::String(s) => s.clone(),
                LiteralValue::Int32(i) => i.to_string(),
                LiteralValue::Int64(i) => i.to_string(),
                LiteralValue::Date(d) => d.clone(),
                LiteralValue::DateTime(dt) => dt.clone(),
                _ => return None,
            }),
            _ => None,
        }
    }

    /// Apply partition pruning to an OLAP scan node
    pub fn apply_to_scan(
        scan: &PlanNode,
        partitions: &[PartitionMetadata],
        partition_column: &str,
    ) -> (PlanNode, PruningResult) {
        match scan {
            PlanNode::OlapScan {
                table_name,
                columns,
                predicates,
                ..
            } => {
                let pruning_result =
                    Self::prune_partitions(partitions, predicates, partition_column);

                // Convert partition IDs to tablet IDs (simplified)
                let tablet_ids: Vec<i64> = pruning_result
                    .retained_partitions
                    .iter()
                    .flat_map(|pid| {
                        // Each partition has multiple tablets
                        // This is simplified - in production, query metadata
                        vec![*pid * 100, *pid * 100 + 1, *pid * 100 + 2]
                    })
                    .collect();

                info!(
                    "Partition pruning for {}: {} tablets retained",
                    table_name,
                    tablet_ids.len()
                );

                let new_scan = PlanNode::OlapScan {
                    table_name: table_name.clone(),
                    columns: columns.clone(),
                    predicates: predicates.clone(),
                    tablet_ids,
                };

                (new_scan, pruning_result)
            }
            _ => (scan.clone(), PruningResult::new(vec![], vec![])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_partition_pruning() {
        let partitions = vec![
            PartitionMetadata {
                partition_id: 1,
                partition_name: "p1".to_string(),
                partition_type: TablePartitionType::Range,
                partition_values: PartitionValues::Range {
                    min: "2024-01-01".to_string(),
                    max: "2024-02-01".to_string(),
                },
            },
            PartitionMetadata {
                partition_id: 2,
                partition_name: "p2".to_string(),
                partition_type: TablePartitionType::Range,
                partition_values: PartitionValues::Range {
                    min: "2024-02-01".to_string(),
                    max: "2024-03-01".to_string(),
                },
            },
            PartitionMetadata {
                partition_id: 3,
                partition_name: "p3".to_string(),
                partition_type: TablePartitionType::Range,
                partition_values: PartitionValues::Range {
                    min: "2024-03-01".to_string(),
                    max: "2024-04-01".to_string(),
                },
            },
        ];

        // Predicate: date >= '2024-02-15'
        let predicates = vec![Expr::BinaryOp {
            op: BinaryOperator::GtEq,
            left: Box::new(Expr::Column {
                name: "date".to_string(),
                data_type: DataType::Date,
            }),
            right: Box::new(Expr::Literal {
                value: LiteralValue::String("2024-02-15".to_string()),
                data_type: DataType::Date,
            }),
        }];

        let result = PartitionPruner::prune_partitions(&partitions, &predicates, "date");

        // Should retain p2 and p3, prune p1
        assert_eq!(result.retained_partitions.len(), 2);
        assert_eq!(result.pruned_partitions.len(), 1);
        assert!(result.retained_partitions.contains(&2));
        assert!(result.retained_partitions.contains(&3));
        assert!(result.pruned_partitions.contains(&1));
    }

    #[test]
    fn test_list_partition_pruning() {
        let partitions = vec![
            PartitionMetadata {
                partition_id: 1,
                partition_name: "p_us".to_string(),
                partition_type: TablePartitionType::List,
                partition_values: PartitionValues::List {
                    values: vec!["US".to_string(), "CA".to_string()],
                },
            },
            PartitionMetadata {
                partition_id: 2,
                partition_name: "p_eu".to_string(),
                partition_type: TablePartitionType::List,
                partition_values: PartitionValues::List {
                    values: vec!["UK".to_string(), "DE".to_string()],
                },
            },
        ];

        // Predicate: country = 'US'
        let predicates = vec![Expr::BinaryOp {
            op: BinaryOperator::Eq,
            left: Box::new(Expr::Column {
                name: "country".to_string(),
                data_type: DataType::String,
            }),
            right: Box::new(Expr::Literal {
                value: LiteralValue::String("US".to_string()),
                data_type: DataType::String,
            }),
        }];

        let result = PartitionPruner::prune_partitions(&partitions, &predicates, "country");

        // Should retain only p_us
        assert_eq!(result.retained_partitions.len(), 1);
        assert_eq!(result.pruned_partitions.len(), 1);
        assert!(result.retained_partitions.contains(&1));
        assert!(result.pruned_partitions.contains(&2));
    }
}
