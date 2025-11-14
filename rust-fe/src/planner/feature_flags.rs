//! Feature Flags for Distributed Query Execution Enhancements
//!
//! This module provides feature flags to enable/disable experimental optimizations
//! for distributed query execution. Each flag can be toggled independently to
//! measure performance impact and validate correctness.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Feature flags for distributed query execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFeatureFlags {
    /// Enable cost-based join strategy selection (broadcast vs. shuffle)
    ///
    /// When enabled: Analyzes table sizes and statistics to choose optimal join strategy
    /// When disabled: Always uses broadcast join (default conservative strategy)
    #[serde(default = "default_false")]
    pub cost_based_join_strategy: bool,

    /// Enable advanced partition pruning
    ///
    /// When enabled: Prunes partitions based on query predicates before execution
    /// When disabled: Scans all partitions
    #[serde(default = "default_false")]
    pub advanced_partition_pruning: bool,

    /// Enable Arrow result data parsing from BE
    ///
    /// When enabled: Parses Arrow IPC format from BE for zero-copy data transfer
    /// When disabled: Uses simplified byte array parsing
    #[serde(default = "default_false")]
    pub arrow_result_parsing: bool,

    /// Enable runtime filter propagation
    ///
    /// When enabled: Propagates filters from join build side to probe side at runtime
    /// When disabled: No runtime filter optimization
    #[serde(default = "default_false")]
    pub runtime_filter_propagation: bool,

    /// Enable bucket shuffle optimization
    ///
    /// When enabled: Uses bucket-aware shuffle to colocate data by bucket ID
    /// When disabled: Uses standard hash partition shuffle
    #[serde(default = "default_false")]
    pub bucket_shuffle_optimization: bool,

    /// Enable parallel fragment execution
    ///
    /// When enabled: Executes independent fragments concurrently
    /// When disabled: Executes fragments sequentially
    #[serde(default = "default_true")]
    pub parallel_fragment_execution: bool,

    /// Enable local aggregation pushdown
    ///
    /// When enabled: Pushes aggregation to storage layer when possible
    /// When disabled: Performs aggregation in compute layer only
    #[serde(default = "default_false")]
    pub local_aggregation_pushdown: bool,

    /// Broadcast size threshold in bytes (default 10MB)
    /// Tables smaller than this will be broadcast, larger ones shuffled
    #[serde(default = "default_broadcast_threshold")]
    pub broadcast_threshold_bytes: u64,

    /// Maximum parallelism for fragment execution
    #[serde(default = "default_max_parallelism")]
    pub max_fragment_parallelism: usize,
}

fn default_false() -> bool {
    false
}

fn default_true() -> bool {
    true
}

fn default_broadcast_threshold() -> u64 {
    10 * 1024 * 1024 // 10MB
}

fn default_max_parallelism() -> usize {
    16
}

impl Default for QueryFeatureFlags {
    fn default() -> Self {
        Self {
            cost_based_join_strategy: false,
            advanced_partition_pruning: false,
            arrow_result_parsing: false,
            runtime_filter_propagation: false,
            bucket_shuffle_optimization: false,
            parallel_fragment_execution: true,
            local_aggregation_pushdown: false,
            broadcast_threshold_bytes: default_broadcast_threshold(),
            max_fragment_parallelism: default_max_parallelism(),
        }
    }
}

impl QueryFeatureFlags {
    /// Create feature flags with all optimizations enabled (experimental mode)
    pub fn all_enabled() -> Self {
        Self {
            cost_based_join_strategy: true,
            advanced_partition_pruning: true,
            arrow_result_parsing: true,
            runtime_filter_propagation: true,
            bucket_shuffle_optimization: true,
            parallel_fragment_execution: true,
            local_aggregation_pushdown: true,
            broadcast_threshold_bytes: default_broadcast_threshold(),
            max_fragment_parallelism: default_max_parallelism(),
        }
    }

    /// Create conservative feature flags (all optimizations disabled)
    pub fn conservative() -> Self {
        Self {
            cost_based_join_strategy: false,
            advanced_partition_pruning: false,
            arrow_result_parsing: false,
            runtime_filter_propagation: false,
            bucket_shuffle_optimization: false,
            parallel_fragment_execution: false,
            local_aggregation_pushdown: false,
            broadcast_threshold_bytes: default_broadcast_threshold(),
            max_fragment_parallelism: 1,
        }
    }

    /// Create flags from environment variables
    pub fn from_env() -> Self {
        Self {
            cost_based_join_strategy: env_bool("DORIS_COST_BASED_JOIN", false),
            advanced_partition_pruning: env_bool("DORIS_PARTITION_PRUNING", false),
            arrow_result_parsing: env_bool("DORIS_ARROW_PARSING", false),
            runtime_filter_propagation: env_bool("DORIS_RUNTIME_FILTERS", false),
            bucket_shuffle_optimization: env_bool("DORIS_BUCKET_SHUFFLE", false),
            parallel_fragment_execution: env_bool("DORIS_PARALLEL_EXECUTION", true),
            local_aggregation_pushdown: env_bool("DORIS_LOCAL_AGG_PUSHDOWN", false),
            broadcast_threshold_bytes: env_u64("DORIS_BROADCAST_THRESHOLD", default_broadcast_threshold()),
            max_fragment_parallelism: env_usize("DORIS_MAX_PARALLELISM", default_max_parallelism()),
        }
    }

    /// Load from TOML configuration file
    pub fn from_toml(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let flags: QueryFeatureFlags = toml::from_str(&content)?;
        Ok(flags)
    }

    /// Save to TOML configuration file
    pub fn to_toml(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Create a preset for specific optimization experiments
    pub fn preset(name: &str) -> Self {
        match name {
            "join-optimization" => Self {
                cost_based_join_strategy: true,
                ..Default::default()
            },
            "partition-pruning" => Self {
                advanced_partition_pruning: true,
                ..Default::default()
            },
            "arrow-format" => Self {
                arrow_result_parsing: true,
                ..Default::default()
            },
            "runtime-filters" => Self {
                runtime_filter_propagation: true,
                ..Default::default()
            },
            "bucket-shuffle" => Self {
                bucket_shuffle_optimization: true,
                ..Default::default()
            },
            "all-execution" => Self {
                cost_based_join_strategy: true,
                advanced_partition_pruning: true,
                runtime_filter_propagation: true,
                bucket_shuffle_optimization: true,
                parallel_fragment_execution: true,
                ..Default::default()
            },
            _ => Self::default(),
        }
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Shared feature flags for query execution
pub type SharedFeatureFlags = Arc<QueryFeatureFlags>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_flags() {
        let flags = QueryFeatureFlags::default();
        assert!(!flags.cost_based_join_strategy);
        assert!(!flags.advanced_partition_pruning);
        assert!(flags.parallel_fragment_execution);
    }

    #[test]
    fn test_all_enabled() {
        let flags = QueryFeatureFlags::all_enabled();
        assert!(flags.cost_based_join_strategy);
        assert!(flags.advanced_partition_pruning);
        assert!(flags.arrow_result_parsing);
        assert!(flags.runtime_filter_propagation);
        assert!(flags.bucket_shuffle_optimization);
    }

    #[test]
    fn test_conservative() {
        let flags = QueryFeatureFlags::conservative();
        assert!(!flags.cost_based_join_strategy);
        assert!(!flags.parallel_fragment_execution);
        assert_eq!(flags.max_fragment_parallelism, 1);
    }

    #[test]
    fn test_presets() {
        let join_flags = QueryFeatureFlags::preset("join-optimization");
        assert!(join_flags.cost_based_join_strategy);
        assert!(!join_flags.advanced_partition_pruning);

        let pruning_flags = QueryFeatureFlags::preset("partition-pruning");
        assert!(!pruning_flags.cost_based_join_strategy);
        assert!(pruning_flags.advanced_partition_pruning);
    }
}
