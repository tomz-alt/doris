// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Query Execution Engine
//!
//! Executes parsed SQL statements using the catalog and backend.

pub mod executor;
pub mod result;

pub use executor::QueryExecutor;
pub use result::{QueryResult, ResultSet, Row};