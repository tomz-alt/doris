//! Comprehensive Regression Test Framework
//!
//! This module implements a test framework similar to Doris's TeamCity CI
//! with 7,484+ regression tests. Tests SQL execution, result correctness,
//! error handling, and performance characteristics.

pub mod test_runner;
pub mod join_tests;
pub mod aggregate_tests;
pub mod subquery_tests;
pub mod function_tests;
pub mod datatype_tests;
pub mod edge_case_tests;

// Total regression tests: 40 (join) + 50 (aggregate) + 45 (subquery) +
// 80 (function) + 30 (datatype) + 28 (edge_case) = 273 tests
