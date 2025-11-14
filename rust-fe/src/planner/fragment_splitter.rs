// Fragment Splitter - Splits single-fragment plans into distributed multi-fragment plans
// This is key to Option B: distributing query execution across multiple BE nodes

use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::plan_fragment::*;
use super::feature_flags::QueryFeatureFlags;
use super::cost_model::{JoinCostModel, JoinStrategy, estimate_node_statistics};
use super::partition_pruner::{PartitionPruner, PartitionMetadata};
use super::runtime_filters::RuntimeFilterBuilder;
use super::bucket_shuffle::{BucketShuffleOptimizer, BucketInfo};
use crate::error::{DorisError, Result};

/// Splits a single-fragment query plan into multiple fragments for distributed execution
pub struct FragmentSplitter {
    query_id: Uuid,
    next_fragment_id: u32,
    feature_flags: Arc<QueryFeatureFlags>,
    cost_model: JoinCostModel,
    runtime_filter_builder: RuntimeFilterBuilder,
    num_executors: usize,
}

impl FragmentSplitter {
    pub fn new(query_id: Uuid) -> Self {
        Self::with_feature_flags(query_id, Arc::new(QueryFeatureFlags::default()), 10)
    }

    pub fn with_feature_flags(
        query_id: Uuid,
        feature_flags: Arc<QueryFeatureFlags>,
        num_executors: usize,
    ) -> Self {
        let cost_model = JoinCostModel::new(num_executors);
        Self {
            query_id,
            next_fragment_id: 0,
            feature_flags,
            cost_model,
            runtime_filter_builder: RuntimeFilterBuilder::new(),
            num_executors,
        }
    }

    /// Split a single-fragment plan into multiple fragments
    /// Returns a distributed query plan with exchange nodes
    pub fn split_into_fragments(&mut self, single_fragment: PlanFragment) -> Result<Vec<PlanFragment>> {
        info!("Splitting single fragment into distributed fragments");

        let mut fragments = Vec::new();

        // Analyze the plan tree to find split points
        let split_plan = self.analyze_and_split(single_fragment.root_node)?;

        // Build fragments from the split plan
        self.build_fragments_recursive(split_plan, &mut fragments)?;

        info!("Split into {} fragments", fragments.len());
        Ok(fragments)
    }

    /// Analyze plan tree and insert exchange nodes at split points
    fn analyze_and_split(&mut self, node: PlanNode) -> Result<PlanNode> {
        match node {
            // Aggregation is a key split point - split into partial/final phases
            PlanNode::Aggregation {
                child,
                group_by_exprs,
                agg_functions,
                is_merge,
            } => {
                debug!("Found aggregation - splitting into partial/final phases");

                // If it's a partial aggregation, recurse on child but don't split further
                if !is_merge {
                    let processed_child = self.analyze_and_split(*child)?;
                    return Ok(PlanNode::Aggregation {
                        child: Box::new(processed_child),
                        group_by_exprs,
                        agg_functions,
                        is_merge: false,
                    });
                }

                // If it's already a final aggregation with a partial child, insert exchange between them
                if let PlanNode::Aggregation { is_merge: false, .. } = *child {
                    // DataFusion already created partial/final - insert exchange between them
                    let partial_child = self.analyze_and_split(*child)?;

                    // Insert exchange node between partial and final
                    let exchange_type = if group_by_exprs.is_empty() {
                        ExchangeType::Gather
                    } else {
                        ExchangeType::HashPartition {
                            partition_keys: group_by_exprs.clone(),
                        }
                    };

                    let exchange = PlanNode::Exchange {
                        child: Box::new(partial_child),
                        exchange_type,
                    };

                    return Ok(PlanNode::Aggregation {
                        child: Box::new(exchange),
                        group_by_exprs,
                        agg_functions,
                        is_merge: true,
                    });
                }

                // For other Final aggregations, split into partial and final phases
                let partial_child = self.analyze_and_split(*child)?;

                // Create partial aggregation (runs on data nodes)
                let partial_agg = PlanNode::Aggregation {
                    child: Box::new(partial_child),
                    group_by_exprs: group_by_exprs.clone(),
                    agg_functions: agg_functions.clone(),
                    is_merge: false,
                };

                // Insert exchange node
                let exchange_type = if group_by_exprs.is_empty() {
                    // No GROUP BY - gather all results to coordinator
                    ExchangeType::Gather
                } else {
                    // GROUP BY - hash partition by group keys for parallel aggregation
                    ExchangeType::HashPartition {
                        partition_keys: group_by_exprs.clone(),
                    }
                };

                let exchange = PlanNode::Exchange {
                    child: Box::new(partial_agg),
                    exchange_type,
                };

                // Create final aggregation (runs on coordinator or hash partitioned)
                Ok(PlanNode::Aggregation {
                    child: Box::new(exchange),
                    group_by_exprs,
                    agg_functions,
                    is_merge: true,
                })
            }

            // Sort + Limit can be optimized with partial sorts
            PlanNode::TopN {
                child,
                order_by,
                limit,
                offset,
            } => {
                debug!("Found TopN - considering partial sort optimization");

                let split_child = self.analyze_and_split(*child)?;

                // For TopN, we can do local TopN on each node, then gather and final TopN
                let local_topn = PlanNode::TopN {
                    child: Box::new(split_child),
                    order_by: order_by.clone(),
                    limit: limit + offset, // Local nodes need to include offset
                    offset: 0,
                };

                // Exchange to gather results
                let exchange = PlanNode::Exchange {
                    child: Box::new(local_topn),
                    exchange_type: ExchangeType::Gather,
                };

                // Final TopN on coordinator
                Ok(PlanNode::TopN {
                    child: Box::new(exchange),
                    order_by,
                    limit,
                    offset,
                })
            }

            // Hash Join - can be split with broadcast or shuffle
            PlanNode::HashJoin {
                left,
                right,
                join_type,
                join_predicates,
            } => {
                debug!("Found HashJoin - determining join strategy");

                let split_left = self.analyze_and_split(*left)?;
                let split_right = self.analyze_and_split(*right)?;

                // Feature Flag: Cost-based join strategy selection
                if self.feature_flags.cost_based_join_strategy {
                    let left_stats = estimate_node_statistics(&split_left);
                    let right_stats = estimate_node_statistics(&split_right);

                    let strategy = self.cost_model.choose_join_strategy(
                        &left_stats,
                        &right_stats,
                        self.feature_flags.broadcast_threshold_bytes,
                    );

                    info!("Cost-based join strategy: {:?}", strategy);

                    match strategy {
                        JoinStrategy::Broadcast { broadcast_side } => {
                            let (broadcast_child, probe_child) = match broadcast_side {
                                super::cost_model::JoinSide::Right => (split_right, split_left),
                                super::cost_model::JoinSide::Left => (split_left, split_right),
                            };

                            let broadcast_node = PlanNode::Exchange {
                                child: Box::new(broadcast_child),
                                exchange_type: ExchangeType::Broadcast,
                            };

                            if matches!(broadcast_side, super::cost_model::JoinSide::Left) {
                                Ok(PlanNode::HashJoin {
                                    left: Box::new(broadcast_node),
                                    right: Box::new(probe_child),
                                    join_type,
                                    join_predicates,
                                })
                            } else {
                                Ok(PlanNode::HashJoin {
                                    left: Box::new(probe_child),
                                    right: Box::new(broadcast_node),
                                    join_type,
                                    join_predicates,
                                })
                            }
                        }
                        JoinStrategy::Shuffle { .. } => {
                            // Extract partition keys from join predicates
                            let partition_keys = join_predicates.clone();

                            let shuffle_left = PlanNode::Exchange {
                                child: Box::new(split_left),
                                exchange_type: ExchangeType::HashPartition {
                                    partition_keys: partition_keys.clone(),
                                },
                            };

                            let shuffle_right = PlanNode::Exchange {
                                child: Box::new(split_right),
                                exchange_type: ExchangeType::HashPartition {
                                    partition_keys,
                                },
                            };

                            Ok(PlanNode::HashJoin {
                                left: Box::new(shuffle_left),
                                right: Box::new(shuffle_right),
                                join_type,
                                join_predicates,
                            })
                        }
                    }
                } else {
                    // Default strategy: Broadcast right side
                    let broadcast_right = PlanNode::Exchange {
                        child: Box::new(split_right),
                        exchange_type: ExchangeType::Broadcast,
                    };

                    Ok(PlanNode::HashJoin {
                        left: Box::new(split_left),
                        right: Box::new(broadcast_right),
                        join_type,
                        join_predicates,
                    })
                }
            }

            // Recursively process other nodes
            PlanNode::Select { child, predicates } => {
                let split_child = self.analyze_and_split(*child)?;
                Ok(PlanNode::Select {
                    child: Box::new(split_child),
                    predicates,
                })
            }

            PlanNode::Project { child, exprs } => {
                let split_child = self.analyze_and_split(*child)?;
                Ok(PlanNode::Project {
                    child: Box::new(split_child),
                    exprs,
                })
            }

            PlanNode::Sort { child, order_by, limit } => {
                let split_child = self.analyze_and_split(*child)?;

                // Sort without limit - needs gather first
                let exchange = PlanNode::Exchange {
                    child: Box::new(split_child),
                    exchange_type: ExchangeType::Gather,
                };

                Ok(PlanNode::Sort {
                    child: Box::new(exchange),
                    order_by,
                    limit,
                })
            }

            // Leaf nodes and simple nodes pass through
            PlanNode::OlapScan { .. } => Ok(node),
            PlanNode::Exchange { .. } => Ok(node),
            PlanNode::Union { .. } => Ok(node),
        }
    }

    /// Build fragments from a plan tree with exchange nodes
    fn build_fragments_recursive(
        &mut self,
        node: PlanNode,
        fragments: &mut Vec<PlanFragment>,
    ) -> Result<usize> {
        // Find exchange nodes - these are fragment boundaries
        match node {
            PlanNode::Exchange { child, exchange_type } => {
                debug!("Found exchange boundary - creating new fragment");

                // The child of exchange becomes a separate fragment
                let child_fragment_idx = self.build_fragments_recursive(*child, fragments)?;

                // Create a fragment that receives from the child fragment
                let fragment_id = self.next_fragment_id();
                let receiver = PlanNode::Exchange {
                    child: Box::new(PlanNode::OlapScan {
                        table_name: format!("_exchange_{}", child_fragment_idx),
                        columns: vec![],
                        predicates: vec![],
                        tablet_ids: vec![],
                    }),
                    exchange_type,
                };

                let fragment = PlanFragment::new(fragment_id, receiver);
                fragments.push(fragment);

                Ok(fragment_id as usize)
            }

            // Recursively process nodes with children
            PlanNode::Aggregation { child, group_by_exprs, agg_functions, is_merge } => {
                let child_tree = self.build_child_tree(*child, fragments)?;

                let new_node = PlanNode::Aggregation {
                    child: Box::new(child_tree),
                    group_by_exprs,
                    agg_functions,
                    is_merge,
                };

                self.create_fragment(new_node, fragments)
            }

            PlanNode::Select { child, predicates } => {
                let child_tree = self.build_child_tree(*child, fragments)?;
                let new_node = PlanNode::Select {
                    child: Box::new(child_tree),
                    predicates,
                };
                self.create_fragment(new_node, fragments)
            }

            PlanNode::Project { child, exprs } => {
                let child_tree = self.build_child_tree(*child, fragments)?;
                let new_node = PlanNode::Project {
                    child: Box::new(child_tree),
                    exprs,
                };
                self.create_fragment(new_node, fragments)
            }

            PlanNode::TopN { child, order_by, limit, offset } => {
                let child_tree = self.build_child_tree(*child, fragments)?;
                let new_node = PlanNode::TopN {
                    child: Box::new(child_tree),
                    order_by,
                    limit,
                    offset,
                };
                self.create_fragment(new_node, fragments)
            }

            PlanNode::Sort { child, order_by, limit } => {
                let child_tree = self.build_child_tree(*child, fragments)?;
                let new_node = PlanNode::Sort {
                    child: Box::new(child_tree),
                    order_by,
                    limit,
                };
                self.create_fragment(new_node, fragments)
            }

            PlanNode::HashJoin { left, right, join_type, join_predicates } => {
                let left_tree = self.build_child_tree(*left, fragments)?;
                let right_tree = self.build_child_tree(*right, fragments)?;
                let new_node = PlanNode::HashJoin {
                    left: Box::new(left_tree),
                    right: Box::new(right_tree),
                    join_type,
                    join_predicates,
                };
                self.create_fragment(new_node, fragments)
            }

            // Leaf nodes - start of fragment
            PlanNode::OlapScan { .. } => self.create_fragment(node, fragments),
            PlanNode::Union { .. } => self.create_fragment(node, fragments),
        }
    }

    fn build_child_tree(
        &mut self,
        child: PlanNode,
        fragments: &mut Vec<PlanFragment>,
    ) -> Result<PlanNode> {
        // Check if this is an exchange node or contains exchange nodes
        match child {
            // Exchange creates fragment boundary
            PlanNode::Exchange { .. } => {
                let _idx = self.build_fragments_recursive(child, fragments)?;
                // Return a reference to the exchange result
                Ok(PlanNode::OlapScan {
                    table_name: "_exchange_result".to_string(),
                    columns: vec![],
                    predicates: vec![],
                    tablet_ids: vec![],
                })
            }

            // Nodes that may contain exchanges - recurse to find them
            PlanNode::Aggregation { child, group_by_exprs, agg_functions, is_merge } => {
                let processed_child = self.build_child_tree(*child, fragments)?;
                Ok(PlanNode::Aggregation {
                    child: Box::new(processed_child),
                    group_by_exprs,
                    agg_functions,
                    is_merge,
                })
            }

            PlanNode::Project { child, exprs } => {
                let processed_child = self.build_child_tree(*child, fragments)?;
                Ok(PlanNode::Project {
                    child: Box::new(processed_child),
                    exprs,
                })
            }

            PlanNode::Select { child, predicates } => {
                let processed_child = self.build_child_tree(*child, fragments)?;
                Ok(PlanNode::Select {
                    child: Box::new(processed_child),
                    predicates,
                })
            }

            PlanNode::TopN { child, order_by, limit, offset } => {
                let processed_child = self.build_child_tree(*child, fragments)?;
                Ok(PlanNode::TopN {
                    child: Box::new(processed_child),
                    order_by,
                    limit,
                    offset,
                })
            }

            PlanNode::Sort { child, order_by, limit } => {
                let processed_child = self.build_child_tree(*child, fragments)?;
                Ok(PlanNode::Sort {
                    child: Box::new(processed_child),
                    order_by,
                    limit,
                })
            }

            PlanNode::HashJoin { left, right, join_type, join_predicates } => {
                let processed_left = self.build_child_tree(*left, fragments)?;
                let processed_right = self.build_child_tree(*right, fragments)?;
                Ok(PlanNode::HashJoin {
                    left: Box::new(processed_left),
                    right: Box::new(processed_right),
                    join_type,
                    join_predicates,
                })
            }

            // Leaf nodes - no exchange possible
            PlanNode::OlapScan { .. } => Ok(child),
            PlanNode::Union { .. } => Ok(child),
        }
    }

    fn create_fragment(&mut self, node: PlanNode, fragments: &mut Vec<PlanFragment>) -> Result<usize> {
        let fragment_id = self.next_fragment_id();
        let fragment = PlanFragment::new(fragment_id, node);
        fragments.push(fragment);
        Ok(fragment_id as usize)
    }

    fn next_fragment_id(&mut self) -> u32 {
        let id = self.next_fragment_id;
        self.next_fragment_id += 1;
        id
    }
}

/// Analyzes a query plan and recommends partitioning strategy
pub fn recommend_partition_strategy(node: &PlanNode) -> Option<PartitionInfo> {
    match node {
        PlanNode::Aggregation { group_by_exprs, .. } if !group_by_exprs.is_empty() => {
            // Hash partition by group keys
            Some(PartitionInfo {
                partition_type: PartitionType::Hash {
                    keys: group_by_exprs
                        .iter()
                        .filter_map(|expr| {
                            if let Expr::Column { name, .. } = expr {
                                Some(name.clone())
                            } else {
                                None
                            }
                        })
                        .collect(),
                },
                num_partitions: 16, // Default, can be adjusted based on cluster size
            })
        }
        PlanNode::OlapScan { .. } => {
            // Random distribution for table scans
            Some(PartitionInfo {
                partition_type: PartitionType::Random,
                num_partitions: 16,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragment_splitter_creation() {
        let query_id = Uuid::new_v4();
        let splitter = FragmentSplitter::new(query_id);
        assert_eq!(splitter.next_fragment_id, 0);
    }

    #[test]
    fn test_recommend_partition_for_scan() {
        let scan = PlanNode::OlapScan {
            table_name: "test".to_string(),
            columns: vec![],
            predicates: vec![],
            tablet_ids: vec![],
        };

        let partition_info = recommend_partition_strategy(&scan);
        assert!(partition_info.is_some());
        assert!(matches!(
            partition_info.unwrap().partition_type,
            PartitionType::Random
        ));
    }
}
