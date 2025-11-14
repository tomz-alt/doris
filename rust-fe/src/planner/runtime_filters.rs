//! Runtime Filter Propagation
//!
//! Generates and propagates runtime filters from join build side to probe side
//! to reduce data scanned at the source

use super::plan_fragment::*;
use tracing::{debug, info};

/// Runtime filter type
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeFilterType {
    /// Bloom filter (probabilistic)
    BloomFilter {
        expected_elements: u64,
        false_positive_rate: f64,
    },
    /// Min-max filter (range)
    MinMax { column: String },
    /// IN list filter (exact values)
    InList { column: String, max_values: usize },
}

/// Runtime filter definition
#[derive(Debug, Clone)]
pub struct RuntimeFilter {
    pub filter_id: u32,
    pub filter_type: RuntimeFilterType,
    pub build_fragment_id: u32,
    pub probe_fragment_ids: Vec<u32>,
    pub join_column: String,
}

/// Runtime filter builder
pub struct RuntimeFilterBuilder {
    next_filter_id: u32,
}

impl RuntimeFilterBuilder {
    pub fn new() -> Self {
        Self { next_filter_id: 0 }
    }

    /// Analyze join node and generate runtime filters
    pub fn generate_filters_for_join(
        &mut self,
        join_node: &PlanNode,
        build_fragment_id: u32,
        probe_fragment_id: u32,
    ) -> Vec<RuntimeFilter> {
        match join_node {
            PlanNode::HashJoin {
                left,
                right,
                join_type,
                join_predicates,
            } => {
                // Only generate filters for inner and left outer joins
                if !matches!(join_type, JoinType::Inner | JoinType::Left) {
                    return vec![];
                }

                let mut filters = Vec::new();

                // Extract join columns from predicates
                for predicate in join_predicates {
                    if let Some((left_col, right_col)) = Self::extract_join_columns(predicate) {
                        // Estimate build side cardinality
                        let build_stats = super::cost_model::estimate_node_statistics(right);

                        // Create appropriate filter based on cardinality
                        let filter = if build_stats.row_count < 1000 {
                            // Small build side -> IN list filter
                            RuntimeFilter {
                                filter_id: self.next_filter_id(),
                                filter_type: RuntimeFilterType::InList {
                                    column: right_col.clone(),
                                    max_values: 1000,
                                },
                                build_fragment_id,
                                probe_fragment_ids: vec![probe_fragment_id],
                                join_column: left_col,
                            }
                        } else if build_stats.row_count < 1_000_000 {
                            // Medium build side -> Bloom filter
                            RuntimeFilter {
                                filter_id: self.next_filter_id(),
                                filter_type: RuntimeFilterType::BloomFilter {
                                    expected_elements: build_stats.row_count,
                                    false_positive_rate: 0.01,
                                },
                                build_fragment_id,
                                probe_fragment_ids: vec![probe_fragment_id],
                                join_column: left_col,
                            }
                        } else {
                            // Large build side -> Min-max filter
                            RuntimeFilter {
                                filter_id: self.next_filter_id(),
                                filter_type: RuntimeFilterType::MinMax {
                                    column: right_col,
                                },
                                build_fragment_id,
                                probe_fragment_ids: vec![probe_fragment_id],
                                join_column: left_col,
                            }
                        };

                        debug!(
                            "Generated runtime filter {} for join column: {:?}",
                            filter.filter_id, filter.filter_type
                        );

                        filters.push(filter);
                    }
                }

                filters
            }
            _ => vec![],
        }
    }

    /// Extract join column names from predicate
    fn extract_join_columns(predicate: &Expr) -> Option<(String, String)> {
        match predicate {
            Expr::BinaryOp {
                op: BinaryOperator::Eq,
                left,
                right,
            } => {
                let left_col = Self::extract_column_name(left)?;
                let right_col = Self::extract_column_name(right)?;
                Some((left_col, right_col))
            }
            _ => None,
        }
    }

    /// Extract column name from expression
    fn extract_column_name(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Column { name, .. } => Some(name.clone()),
            Expr::Cast { expr, .. } => Self::extract_column_name(expr),
            _ => None,
        }
    }

    fn next_filter_id(&mut self) -> u32 {
        let id = self.next_filter_id;
        self.next_filter_id += 1;
        id
    }

    /// Apply runtime filter to scan node
    pub fn apply_filter_to_scan(
        scan: &PlanNode,
        runtime_filter: &RuntimeFilter,
    ) -> PlanNode {
        match scan {
            PlanNode::OlapScan {
                table_name,
                columns,
                predicates,
                tablet_ids,
            } => {
                let mut new_predicates = predicates.clone();

                // Add runtime filter predicate
                match &runtime_filter.filter_type {
                    RuntimeFilterType::BloomFilter { .. } => {
                        // Add bloom filter check (represented as function call)
                        let filter_predicate = Expr::Function {
                            name: format!("runtime_bloom_filter_{}", runtime_filter.filter_id),
                            args: vec![Expr::Column {
                                name: runtime_filter.join_column.clone(),
                                data_type: DataType::Int, // Simplified
                            }],
                        };
                        new_predicates.push(filter_predicate);
                    }
                    RuntimeFilterType::MinMax { .. } => {
                        // Min-max filter will be applied by storage layer
                        // Add marker predicate
                        let filter_predicate = Expr::Function {
                            name: format!("runtime_minmax_filter_{}", runtime_filter.filter_id),
                            args: vec![Expr::Column {
                                name: runtime_filter.join_column.clone(),
                                data_type: DataType::Int,
                            }],
                        };
                        new_predicates.push(filter_predicate);
                    }
                    RuntimeFilterType::InList { max_values, .. } => {
                        // IN list filter
                        let filter_predicate = Expr::Function {
                            name: format!(
                                "runtime_in_filter_{}_max_{}",
                                runtime_filter.filter_id, max_values
                            ),
                            args: vec![Expr::Column {
                                name: runtime_filter.join_column.clone(),
                                data_type: DataType::Int,
                            }],
                        };
                        new_predicates.push(filter_predicate);
                    }
                }

                info!(
                    "Applied runtime filter {} to scan of {}",
                    runtime_filter.filter_id, table_name
                );

                PlanNode::OlapScan {
                    table_name: table_name.clone(),
                    columns: columns.clone(),
                    predicates: new_predicates,
                    tablet_ids: tablet_ids.clone(),
                }
            }
            _ => scan.clone(),
        }
    }
}

impl Default for RuntimeFilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_join_columns() {
        let predicate = Expr::BinaryOp {
            op: BinaryOperator::Eq,
            left: Box::new(Expr::Column {
                name: "t1.id".to_string(),
                data_type: DataType::Int,
            }),
            right: Box::new(Expr::Column {
                name: "t2.id".to_string(),
                data_type: DataType::Int,
            }),
        };

        let result = RuntimeFilterBuilder::extract_join_columns(&predicate);
        assert_eq!(result, Some(("t1.id".to_string(), "t2.id".to_string())));
    }

    #[test]
    fn test_generate_filters_for_join() {
        let mut builder = RuntimeFilterBuilder::new();

        let join_node = PlanNode::HashJoin {
            left: Box::new(PlanNode::OlapScan {
                table_name: "fact_table".to_string(),
                columns: vec![],
                predicates: vec![],
                tablet_ids: vec![],
            }),
            right: Box::new(PlanNode::OlapScan {
                table_name: "dim_small".to_string(),
                columns: vec![],
                predicates: vec![],
                tablet_ids: vec![],
            }),
            join_type: JoinType::Inner,
            join_predicates: vec![Expr::BinaryOp {
                op: BinaryOperator::Eq,
                left: Box::new(Expr::Column {
                    name: "fact_id".to_string(),
                    data_type: DataType::Int,
                }),
                right: Box::new(Expr::Column {
                    name: "dim_id".to_string(),
                    data_type: DataType::Int,
                }),
            }],
        };

        let filters = builder.generate_filters_for_join(&join_node, 1, 2);
        assert!(!filters.is_empty());
        assert_eq!(filters[0].build_fragment_id, 1);
        assert!(filters[0].probe_fragment_ids.contains(&2));
    }
}
