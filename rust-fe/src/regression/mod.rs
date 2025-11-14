//! Comprehensive Regression Test Framework
//!
//! This module implements a test framework similar to Doris's TeamCity CI
//! with 7,484+ regression tests. Tests SQL execution, result correctness,
//! error handling, and performance characteristics.
//!
//! Test Coverage:
//! - Core SQL Features: JOIN, aggregations, subqueries, functions, datatypes
//! - DDL Operations: CREATE, ALTER, DROP for tables, databases, indexes
//! - DML Operations: INSERT, UPDATE, DELETE with various patterns
//! - Advanced Features: Partitioning, materialized views, views, CTEs
//! - Indexing: B-Tree indexes, inverted indexes, bloom filters
//! - Edge Cases: NULL handling, empty sets, extreme values

pub mod test_runner;

// Core SQL Tests (273 tests)
pub mod join_tests;           // 40 tests
pub mod aggregate_tests;      // 50 tests
pub mod subquery_tests;       // 45 tests
pub mod function_tests;       // 80 tests
pub mod datatype_tests;       // 30 tests
pub mod edge_case_tests;      // 28 tests

// DDL/DML Tests (130 tests)
pub mod ddl_tests;            // 60 tests
pub mod dml_tests;            // 70 tests

// Advanced Features (200 tests)
pub mod partition_tests;            // 50 tests
pub mod materialized_view_tests;    // 50 tests
pub mod view_tests;                 // 50 tests
pub mod index_tests;                // 50 tests

// Total regression tests: 603 tests
// Combined with existing tests: 603 + 529 = 1,132 comprehensive tests
