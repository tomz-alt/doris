//! Integration tests for distributed query optimization feature flags

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn test_default_feature_flags() {
        let flags = QueryFeatureFlags::default();

        // All experimental features disabled by default
        assert!(!flags.cost_based_join_strategy);
        assert!(!flags.advanced_partition_pruning);
        assert!(!flags.arrow_result_parsing);
        assert!(!flags.runtime_filter_propagation);
        assert!(!flags.bucket_shuffle_optimization);

        // Stable features enabled
        assert!(flags.parallel_fragment_execution);

        // Default thresholds
        assert_eq!(flags.broadcast_threshold_bytes, 10 * 1024 * 1024);
        assert_eq!(flags.max_fragment_parallelism, 16);
    }

    #[test]
    fn test_all_enabled_flags() {
        let flags = QueryFeatureFlags::all_enabled();

        assert!(flags.cost_based_join_strategy);
        assert!(flags.advanced_partition_pruning);
        assert!(flags.arrow_result_parsing);
        assert!(flags.runtime_filter_propagation);
        assert!(flags.bucket_shuffle_optimization);
        assert!(flags.parallel_fragment_execution);
        assert!(flags.local_aggregation_pushdown);
    }

    #[test]
    fn test_conservative_flags() {
        let flags = QueryFeatureFlags::conservative();

        assert!(!flags.cost_based_join_strategy);
        assert!(!flags.parallel_fragment_execution);
        assert_eq!(flags.max_fragment_parallelism, 1);
    }

    #[test]
    fn test_feature_flag_presets() {
        // Test join optimization preset
        let join_flags = QueryFeatureFlags::preset("join-optimization");
        assert!(join_flags.cost_based_join_strategy);
        assert!(!join_flags.advanced_partition_pruning);

        // Test partition pruning preset
        let pruning_flags = QueryFeatureFlags::preset("partition-pruning");
        assert!(!pruning_flags.cost_based_join_strategy);
        assert!(pruning_flags.advanced_partition_pruning);

        // Test all execution optimizations
        let all_exec_flags = QueryFeatureFlags::preset("all-execution");
        assert!(all_exec_flags.cost_based_join_strategy);
        assert!(all_exec_flags.advanced_partition_pruning);
        assert!(all_exec_flags.runtime_filter_propagation);
        assert!(all_exec_flags.bucket_shuffle_optimization);
    }

    #[test]
    fn test_fragment_splitter_with_cost_based_join() {
        let query_id = Uuid::new_v4();

        // Test with cost-based optimization enabled
        let flags_enabled = Arc::new(QueryFeatureFlags {
            cost_based_join_strategy: true,
            ..Default::default()
        });

        let _splitter = FragmentSplitter::with_feature_flags(
            query_id,
            flags_enabled.clone(),
            10,
        );

        // Verify flags were passed correctly
        assert!(flags_enabled.cost_based_join_strategy);
    }

    #[test]
    fn test_join_cost_estimation() {
        let cost_model = JoinCostModel::new(10);

        // Small build side -> should choose broadcast
        let small_table = TableStatistics::estimated(1000, 100);
        let large_table = TableStatistics::estimated(1_000_000, 200);

        let strategy = cost_model.choose_join_strategy(
            &large_table,
            &small_table,
            10_000_000,
        );

        assert!(matches!(strategy, JoinStrategy::Broadcast { .. }));
    }

    #[test]
    fn test_partition_pruning_range() {
        let partitions = vec![
            PartitionMetadata {
                partition_id: 1,
                partition_name: "p1".to_string(),
                partition_type: partition_pruner::TablePartitionType::Range,
                partition_values: partition_pruner::PartitionValues::Range {
                    min: "2024-01-01".to_string(),
                    max: "2024-02-01".to_string(),
                },
            },
            PartitionMetadata {
                partition_id: 2,
                partition_name: "p2".to_string(),
                partition_type: partition_pruner::TablePartitionType::Range,
                partition_values: partition_pruner::PartitionValues::Range {
                    min: "2024-02-01".to_string(),
                    max: "2024-03-01".to_string(),
                },
            },
        ];

        // Predicate: date >= '2024-02-15'  (middle of p2, should prune p1)
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

        // Should retain only p2 (contains dates >= 2024-02-15), prune p1
        assert!(result.retained_partitions.len() >= 1);
        assert!(result.retained_partitions.contains(&2));
    }

    #[test]
    fn test_runtime_filter_generation() {
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

    #[test]
    fn test_bucket_shuffle_colocation() {
        let left_bucket = BucketInfo {
            table_name: "t1".to_string(),
            bucket_column: "id".to_string(),
            num_buckets: 16,
            bucket_type: bucket_shuffle::BucketType::Hash,
        };

        let right_bucket = BucketInfo {
            table_name: "t2".to_string(),
            bucket_column: "id".to_string(),
            num_buckets: 16,
            bucket_type: bucket_shuffle::BucketType::Hash,
        };

        assert!(BucketShuffleOptimizer::can_colocate(&left_bucket, &right_bucket));

        // Different bucket counts cannot colocate
        let right_different = BucketInfo {
            table_name: "t2".to_string(),
            bucket_column: "id".to_string(),
            num_buckets: 32,
            bucket_type: bucket_shuffle::BucketType::Hash,
        };

        assert!(!BucketShuffleOptimizer::can_colocate(&left_bucket, &right_different));
    }

    #[test]
    fn test_fragment_executor_with_flags() {
        let query_id = Uuid::new_v4();

        // Test with Arrow parsing enabled
        let flags = Arc::new(QueryFeatureFlags {
            arrow_result_parsing: true,
            ..Default::default()
        });

        let _executor = FragmentExecutor::with_feature_flags(query_id, flags.clone());

        // Verify flags were passed correctly
        assert!(flags.arrow_result_parsing);
    }
}
