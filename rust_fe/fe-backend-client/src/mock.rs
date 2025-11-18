// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Mock Backend for Testing
//!
//! This module provides a mock implementation of the C++ Backend
//! for testing purposes when protoc is not available or when
//! testing without a real BE.

use fe_common::Result;
use fe_planner::thrift_plan::TPlanFragment;
use fe_qe::result::Row;
#[cfg(test)]
use fe_qe::result::Value;

/// Mock backend that simulates C++ BE responses
pub struct MockBackend {
    /// Simulated query results
    mock_results: Vec<Vec<Row>>,
    /// Current result index
    result_index: usize,
}

impl MockBackend {
    /// Create a new mock backend
    pub fn new() -> Self {
        Self {
            mock_results: Vec::new(),
            result_index: 0,
        }
    }

    /// Set mock results for the next query
    pub fn set_mock_results(&mut self, results: Vec<Row>) {
        self.mock_results.push(results);
    }

    /// Simulate exec_plan_fragment
    pub async fn exec_plan_fragment(
        &mut self,
        _fragment: &TPlanFragment,
        _query_id: [u8; 16],
    ) -> Result<[u8; 16]> {
        // Return a mock fragment instance ID
        Ok([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
    }

    /// Simulate fetch_data
    pub async fn fetch_data(
        &mut self,
        _finst_id: [u8; 16],
    ) -> Result<Vec<Row>> {
        if self.result_index < self.mock_results.len() {
            let results = self.mock_results[self.result_index].clone();
            self.result_index += 1;
            Ok(results)
        } else {
            // Return empty result set
            Ok(Vec::new())
        }
    }

    /// Create mock TPC-H Q1 results (0 rows for now)
    pub fn create_tpch_q1_results() -> Vec<Row> {
        // Empty result set with correct schema
        // In a real scenario, this would contain aggregated data
        Vec::new()
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_backend_creation() {
        let backend = MockBackend::new();
        assert_eq!(backend.result_index, 0);
        assert_eq!(backend.mock_results.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_exec_plan_fragment() {
        let mut backend = MockBackend::new();

        // Create a dummy fragment (we don't need real data for mock)
        let fragment = TPlanFragment {
            plan: fe_planner::thrift_plan::TPlan {
                nodes: Vec::new(),
            },
        };

        let query_id = [0u8; 16];
        let finst_id = backend.exec_plan_fragment(&fragment, query_id).await.unwrap();

        // Should return a mock instance ID
        assert_eq!(finst_id.len(), 16);
    }

    #[tokio::test]
    async fn test_mock_fetch_data_empty() {
        let mut backend = MockBackend::new();

        let finst_id = [0u8; 16];
        let results = backend.fetch_data(finst_id).await.unwrap();

        // Should return empty results
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_mock_fetch_data_with_results() {
        let mut backend = MockBackend::new();

        // Set mock results
        let mock_row = Row {
            values: vec![
                Value::Int(1),
                Value::String("test".to_string()),
            ],
        };
        backend.set_mock_results(vec![mock_row.clone()]);

        let finst_id = [0u8; 16];
        let results = backend.fetch_data(finst_id).await.unwrap();

        // Should return the mock results
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].values.len(), 2);
    }

    #[tokio::test]
    async fn test_tpch_q1_mock_results() {
        let results = MockBackend::create_tpch_q1_results();

        // Should return empty for now (no data)
        assert_eq!(results.len(), 0);
    }
}
