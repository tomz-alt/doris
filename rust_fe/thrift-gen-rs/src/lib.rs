//! Auto-generated Thrift bindings for Apache Doris
//!
//! This module contains Thrift structures generated from Doris .thrift files
//! using the Apache Thrift compiler (v0.19.0).
//!
//! DO NOT MODIFY - regenerate with: thrift --gen rs <file>.thrift

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::all)]

// Re-export all generated modules
pub mod types;
pub mod data;
pub mod descriptors;
pub mod exprs;
pub mod plan_nodes;
pub mod data_sinks;
pub mod planner;
pub mod runtime_profile;
pub mod status;
pub mod palo_internal_service;
pub mod partitions;
pub mod opcodes;
pub mod external_table_schema;
pub mod query_cache;
pub mod metrics;

// Re-export commonly used types for convenience
pub use palo_internal_service::{
    TPipelineFragmentParamsList,
    TPipelineFragmentParams,
    TPipelineInstanceParams,
    TQueryOptions,
    TQueryGlobals,
};

pub use types::{
    TUniqueId,
    TNetworkAddress,
};

pub use descriptors::TDescriptorTable;
pub use planner::TPlanFragment;
