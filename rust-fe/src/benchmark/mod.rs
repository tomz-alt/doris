//! Benchmark infrastructure for distributed query optimization validation

pub mod query_profiler;

pub use query_profiler::{QueryProfiler, QueryProfile, ExecutionMetrics, OptimizationInfo, ProfileComparison};
