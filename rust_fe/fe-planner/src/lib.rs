// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Query Planner
//!
//! Converts parsed SQL AST into execution plans and Thrift payloads for BE.

pub mod planner;
pub mod thrift_plan;
pub mod thrift_serialize;
pub mod scan_range_builder;

pub use planner::QueryPlanner;
pub use thrift_serialize::{serialize_plan_fragment, serialize_pipeline_params, write_descriptor_table};
pub use scan_range_builder::{ScanRangeBuilder, TScanRangeLocations, TPaloScanRange, TScanRangeLocation};

// Export pipeline execution structures (VERSION_3)
pub use thrift_plan::{
    TPipelineFragmentParamsList,
    TPipelineFragmentParams,
    TPipelineInstanceParams,
    TScanRangeParams,
    TUniqueId,
};