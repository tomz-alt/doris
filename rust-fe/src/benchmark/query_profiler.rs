//! Query Profiler - Captures BE logs, execution metrics, and query profiles
//!
//! Used to validate that optimizations are actually being applied

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use uuid::Uuid;
use tracing::{info, debug};

use crate::planner::QueryFeatureFlags;

/// Query execution profile with detailed metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryProfile {
    pub query_id: Uuid,
    pub query_sql: String,
    pub feature_flags: QueryFeatureFlags,
    pub execution_metrics: ExecutionMetrics,
    pub optimization_info: OptimizationInfo,
    pub be_metrics: Vec<BackendMetrics>,
}

/// Execution timing and resource metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Total end-to-end query time
    pub total_time_ms: u64,

    /// Planning time (FE)
    pub planning_time_ms: u64,

    /// Fragment splitting time
    pub fragment_splitting_time_ms: u64,

    /// BE execution time (sum across all fragments)
    pub be_execution_time_ms: u64,

    /// Result transfer time
    pub result_transfer_time_ms: u64,

    /// Number of fragments generated
    pub num_fragments: usize,

    /// Number of BEs involved
    pub num_backends: usize,

    /// Total rows scanned
    pub rows_scanned: u64,

    /// Total rows returned
    pub rows_returned: u64,

    /// Bytes scanned
    pub bytes_scanned: u64,

    /// Bytes returned
    pub bytes_returned: u64,
}

/// Information about which optimizations were applied
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationInfo {
    /// Cost-based join decisions made
    pub join_strategies: Vec<JoinStrategyDecision>,

    /// Partitions pruned
    pub partition_pruning: Option<PartitionPruningInfo>,

    /// Runtime filters generated
    pub runtime_filters: Vec<RuntimeFilterInfo>,

    /// Bucket shuffle optimizations applied
    pub bucket_shuffle: Vec<BucketShuffleInfo>,

    /// Whether Arrow format was used
    pub arrow_format_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinStrategyDecision {
    pub join_node_id: usize,
    pub strategy_chosen: String, // "Broadcast" or "Shuffle"
    pub left_table: String,
    pub right_table: String,
    pub left_size_bytes: u64,
    pub right_size_bytes: u64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionPruningInfo {
    pub table_name: String,
    pub total_partitions: usize,
    pub pruned_partitions: usize,
    pub pruning_ratio: f64,
    pub partitions_scanned: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeFilterInfo {
    pub filter_id: u32,
    pub filter_type: String, // "BloomFilter", "MinMax", "InList"
    pub join_column: String,
    pub build_side_rows: u64,
    pub probe_side_rows_before: u64,
    pub probe_side_rows_after: u64,
    pub filter_effectiveness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketShuffleInfo {
    pub left_table: String,
    pub right_table: String,
    pub bucket_column: String,
    pub num_buckets: u32,
    pub colocated: bool,
    pub network_bytes_saved: u64,
}

/// Backend execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendMetrics {
    pub backend_id: String,
    pub fragment_id: u32,
    pub execution_time_ms: u64,
    pub rows_scanned: u64,
    pub rows_returned: u64,
    pub bytes_scanned: u64,
    pub cpu_time_ms: u64,
    pub io_time_ms: u64,
}

/// Query profiler
pub struct QueryProfiler {
    profiles: HashMap<Uuid, QueryProfile>,
}

impl QueryProfiler {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Start profiling a query
    pub fn start_query(
        &mut self,
        query_id: Uuid,
        query_sql: String,
        feature_flags: QueryFeatureFlags,
    ) -> QueryProfileHandle {
        let profile = QueryProfile {
            query_id,
            query_sql,
            feature_flags,
            execution_metrics: ExecutionMetrics {
                total_time_ms: 0,
                planning_time_ms: 0,
                fragment_splitting_time_ms: 0,
                be_execution_time_ms: 0,
                result_transfer_time_ms: 0,
                num_fragments: 0,
                num_backends: 0,
                rows_scanned: 0,
                rows_returned: 0,
                bytes_scanned: 0,
                bytes_returned: 0,
            },
            optimization_info: OptimizationInfo {
                join_strategies: vec![],
                partition_pruning: None,
                runtime_filters: vec![],
                bucket_shuffle: vec![],
                arrow_format_used: false,
            },
            be_metrics: vec![],
        };

        self.profiles.insert(query_id, profile);

        QueryProfileHandle {
            query_id,
            start_time: Instant::now(),
        }
    }

    /// Record join strategy decision
    pub fn record_join_strategy(
        &mut self,
        query_id: Uuid,
        join_node_id: usize,
        strategy: &str,
        left_table: String,
        right_table: String,
        left_size: u64,
        right_size: u64,
        reason: String,
    ) {
        if let Some(profile) = self.profiles.get_mut(&query_id) {
            info!(
                "Query {}: Join strategy chosen for node {}: {} (left: {} bytes, right: {} bytes) - {}",
                query_id, join_node_id, strategy, left_size, right_size, reason
            );

            profile.optimization_info.join_strategies.push(JoinStrategyDecision {
                join_node_id,
                strategy_chosen: strategy.to_string(),
                left_table,
                right_table,
                left_size_bytes: left_size,
                right_size_bytes: right_size,
                reason,
            });
        }
    }

    /// Record partition pruning
    pub fn record_partition_pruning(
        &mut self,
        query_id: Uuid,
        table_name: String,
        total: usize,
        pruned: usize,
        scanned_partitions: Vec<String>,
    ) {
        if let Some(profile) = self.profiles.get_mut(&query_id) {
            let pruning_ratio = pruned as f64 / total as f64;

            profile.optimization_info.partition_pruning = Some(PartitionPruningInfo {
                table_name: table_name.clone(),
                total_partitions: total,
                pruned_partitions: pruned,
                pruning_ratio,
                partitions_scanned: scanned_partitions,
            });

            info!(
                "Query {}: Partition pruning on {}: {}/{} partitions pruned ({:.1}% reduction)",
                query_id, table_name, pruned, total, pruning_ratio * 100.0
            );
        }
    }

    /// Record runtime filter
    pub fn record_runtime_filter(
        &mut self,
        query_id: Uuid,
        filter_id: u32,
        filter_type: &str,
        join_column: String,
        build_rows: u64,
        probe_before: u64,
        probe_after: u64,
    ) {
        if let Some(profile) = self.profiles.get_mut(&query_id) {
            let effectiveness = 1.0 - (probe_after as f64 / probe_before as f64);

            profile.optimization_info.runtime_filters.push(RuntimeFilterInfo {
                filter_id,
                filter_type: filter_type.to_string(),
                join_column: join_column.clone(),
                build_side_rows: build_rows,
                probe_side_rows_before: probe_before,
                probe_side_rows_after: probe_after,
                filter_effectiveness: effectiveness,
            });

            info!(
                "Query {}: Runtime filter {} ({}) on {}: filtered {:.1}% of probe rows",
                query_id, filter_id, filter_type, join_column, effectiveness * 100.0
            );
        }
    }

    /// Record bucket shuffle optimization
    pub fn record_bucket_shuffle(
        &mut self,
        query_id: Uuid,
        left_table: String,
        right_table: String,
        bucket_column: String,
        num_buckets: u32,
        colocated: bool,
        bytes_saved: u64,
    ) {
        if let Some(profile) = self.profiles.get_mut(&query_id) {
            profile.optimization_info.bucket_shuffle.push(BucketShuffleInfo {
                left_table: left_table.clone(),
                right_table: right_table.clone(),
                bucket_column: bucket_column.clone(),
                num_buckets,
                colocated,
                network_bytes_saved: bytes_saved,
            });

            if colocated {
                info!(
                    "Query {}: Bucket shuffle optimization: {} JOIN {} on {} (colocated, saved {} MB network)",
                    query_id, left_table, right_table, bucket_column, bytes_saved / 1024 / 1024
                );
            }
        }
    }

    /// Complete query profiling
    pub fn complete_query(
        &mut self,
        handle: QueryProfileHandle,
        num_fragments: usize,
        num_backends: usize,
        rows_returned: u64,
    ) {
        let total_time = handle.start_time.elapsed();

        if let Some(profile) = self.profiles.get_mut(&handle.query_id) {
            profile.execution_metrics.total_time_ms = total_time.as_millis() as u64;
            profile.execution_metrics.num_fragments = num_fragments;
            profile.execution_metrics.num_backends = num_backends;
            profile.execution_metrics.rows_returned = rows_returned;

            info!(
                "Query {} completed in {:.2}s ({} fragments, {} backends, {} rows)",
                handle.query_id,
                total_time.as_secs_f64(),
                num_fragments,
                num_backends,
                rows_returned
            );
        }
    }

    /// Get profile for a query
    pub fn get_profile(&self, query_id: &Uuid) -> Option<&QueryProfile> {
        self.profiles.get(query_id)
    }

    /// Generate comparison report between two configurations
    pub fn compare_profiles(
        &self,
        baseline_id: &Uuid,
        optimized_id: &Uuid,
    ) -> Option<ProfileComparison> {
        let baseline = self.profiles.get(baseline_id)?;
        let optimized = self.profiles.get(optimized_id)?;

        Some(ProfileComparison {
            baseline: baseline.clone(),
            optimized: optimized.clone(),
            speedup: baseline.execution_metrics.total_time_ms as f64
                / optimized.execution_metrics.total_time_ms as f64,
            io_reduction: 1.0
                - (optimized.execution_metrics.bytes_scanned as f64
                    / baseline.execution_metrics.bytes_scanned as f64),
        })
    }

    /// Export all profiles as JSON
    pub fn export_json(&self) -> String {
        serde_json::to_string_pretty(&self.profiles).unwrap_or_default()
    }
}

impl Default for QueryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle for an in-progress query profile
pub struct QueryProfileHandle {
    pub query_id: Uuid,
    start_time: Instant,
}

/// Comparison between baseline and optimized query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileComparison {
    pub baseline: QueryProfile,
    pub optimized: QueryProfile,
    pub speedup: f64,
    pub io_reduction: f64,
}

impl ProfileComparison {
    /// Generate markdown report
    pub fn to_markdown(&self) -> String {
        format!(
            r#"# Query Performance Comparison

## Query
```sql
{}
```

## Results Summary
- **Speedup**: {:.2}x faster
- **I/O Reduction**: {:.1}% less data scanned

## Baseline Configuration
- Total Time: {:.2}s
- Fragments: {}
- Backends: {}
- Rows Scanned: {}
- Bytes Scanned: {} MB

## Optimized Configuration
- Total Time: {:.2}s
- Fragments: {}
- Backends: {}
- Rows Scanned: {}
- Bytes Scanned: {} MB

## Optimizations Applied
{}

## Join Strategy Decisions
{}

## Partition Pruning
{}

## Runtime Filters
{}
"#,
            self.baseline.query_sql,
            self.speedup,
            self.io_reduction * 100.0,
            self.baseline.execution_metrics.total_time_ms as f64 / 1000.0,
            self.baseline.execution_metrics.num_fragments,
            self.baseline.execution_metrics.num_backends,
            self.baseline.execution_metrics.rows_scanned,
            self.baseline.execution_metrics.bytes_scanned / 1024 / 1024,
            self.optimized.execution_metrics.total_time_ms as f64 / 1000.0,
            self.optimized.execution_metrics.num_fragments,
            self.optimized.execution_metrics.num_backends,
            self.optimized.execution_metrics.rows_scanned,
            self.optimized.execution_metrics.bytes_scanned / 1024 / 1024,
            self.format_optimizations(),
            self.format_join_strategies(),
            self.format_partition_pruning(),
            self.format_runtime_filters(),
        )
    }

    fn format_optimizations(&self) -> String {
        let opts = &self.optimized.optimization_info;
        let mut lines = vec![];

        if !opts.join_strategies.is_empty() {
            lines.push(format!("- Cost-based join selection: {} decisions", opts.join_strategies.len()));
        }

        if let Some(ref pruning) = opts.partition_pruning {
            lines.push(format!(
                "- Partition pruning: {}/{} partitions ({:.1}%)",
                pruning.pruned_partitions,
                pruning.total_partitions,
                pruning.pruning_ratio * 100.0
            ));
        }

        if !opts.runtime_filters.is_empty() {
            lines.push(format!("- Runtime filters: {} generated", opts.runtime_filters.len()));
        }

        if !opts.bucket_shuffle.is_empty() {
            lines.push(format!("- Bucket shuffle: {} colocations", opts.bucket_shuffle.len()));
        }

        if opts.arrow_format_used {
            lines.push("- Arrow format: enabled".to_string());
        }

        if lines.is_empty() {
            "None".to_string()
        } else {
            lines.join("\n")
        }
    }

    fn format_join_strategies(&self) -> String {
        let strategies = &self.optimized.optimization_info.join_strategies;
        if strategies.is_empty() {
            return "No join strategy decisions recorded".to_string();
        }

        strategies
            .iter()
            .map(|s| {
                format!(
                    "- Node {}: {} - {} ⨝ {} ({} / {} bytes) - {}",
                    s.join_node_id,
                    s.strategy_chosen,
                    s.left_table,
                    s.right_table,
                    s.left_size_bytes,
                    s.right_size_bytes,
                    s.reason
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_partition_pruning(&self) -> String {
        if let Some(ref pruning) = self.optimized.optimization_info.partition_pruning {
            format!(
                "Table: {}\nTotal Partitions: {}\nPruned: {}\nScanned: {}\nPartitions: {:?}",
                pruning.table_name,
                pruning.total_partitions,
                pruning.pruned_partitions,
                pruning.partitions_scanned.len(),
                pruning.partitions_scanned
            )
        } else {
            "No partition pruning applied".to_string()
        }
    }

    fn format_runtime_filters(&self) -> String {
        let filters = &self.optimized.optimization_info.runtime_filters;
        if filters.is_empty() {
            return "No runtime filters generated".to_string();
        }

        filters
            .iter()
            .map(|f| {
                format!(
                    "- Filter {}: {} on {} - {:.1}% filtered ({} -> {} rows)",
                    f.filter_id,
                    f.filter_type,
                    f.join_column,
                    f.filter_effectiveness * 100.0,
                    f.probe_side_rows_before,
                    f.probe_side_rows_after
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
