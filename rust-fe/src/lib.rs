// Library exports for doris-rust-fe
// This allows examples and tests to use the modules

pub mod error;
pub mod config;
pub mod mysql;
pub mod http;
pub mod query;
pub mod metadata;
pub mod parser;
pub mod planner;
pub mod be;
pub mod catalog;
pub mod benchmark;
pub mod observability_tests;
pub mod regression;

// Re-export commonly used types
pub use error::{DorisError, Result};
pub use config::Config;
pub use query::{QueryExecutor, QueryResult};
pub use planner::{DataFusionPlanner, PlanConverter, PlanFragment, QueryPlan, FragmentSplitter};
