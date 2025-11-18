// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Query Execution Engine
//!
//! Executes parsed SQL statements using the catalog and backend.

pub mod executor;
pub mod executor_core; // CLAUDE.md Principle #1: Clean execute_sql interface
pub mod result;

#[cfg(test)]
mod select_tests;

#[cfg(test)]
mod comparison_tests;

pub use executor::QueryExecutor;
pub use executor_core::{SqlExecutor, ExecutionMetrics}; // Main entry point
pub use result::{QueryResult, ResultSet, Row};