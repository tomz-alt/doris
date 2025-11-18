// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Thrift Plan Structures
//!
//! Manual Thrift bindings for Doris plan structures.
//! These match the .thrift definitions exactly.

use serde::{Deserialize, Serialize};

/// Plan node types (from PlanNodes.thrift)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum TPlanNodeType {
    OlapScanNode = 0,
    MysqlScanNode = 1,
    CsvScanNode = 2,
    SchemaScanNode = 3,
    HashJoinNode = 4,
    MergeJoinNode = 5,
    AggregationNode = 6,
    PreAggregationNode = 7,
    SortNode = 8,
    ExchangeNode = 9,
    MergeNode = 10,
    SelectNode = 11,
    CrossJoinNode = 12,
    MetaScanNode = 13,
    AnalyticEvalNode = 14,
    OlapRewriteNode = 15,
    KuduScanNode = 16,
    BrokerScanNode = 17,
    EmptySetNode = 18,
    UnionNode = 19,
    EsScanNode = 20,
    EsHttpScanNode = 21,
    RepeatNode = 22,
    AssertNumRowsNode = 23,
    IntersectNode = 24,
    ExceptNode = 25,
    OdbcScanNode = 26,
    TableFunctionNode = 27,
    DataGenScanNode = 28,
    FileScanNode = 29,
    JdbcScanNode = 30,
    TestExternalScanNode = 31,
    PartitionSortNode = 32,
    GroupCommitScanNode = 33,
    MaterializationNode = 34,
}

/// Network address
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TNetworkAddress {
    pub hostname: String,
    pub port: i32,
}

/// Tablet ID
pub type TTabletId = i64;

/// Tuple ID
pub type TTupleId = i32;

/// Plan node ID
pub type TPlanNodeId = i32;

/// OLAP scan range (from PlanNodes.thrift TPaloScanRange)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPaloScanRange {
    pub hosts: Vec<TNetworkAddress>,
    pub schema_hash: String,
    pub version: String,
    pub version_hash: String,
    pub tablet_id: TTabletId,
    pub db_name: String,
    pub index_name: Option<String>,
    pub table_name: Option<String>,
}

/// OLAP scan node (from PlanNodes.thrift)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TOlapScanNode {
    pub tuple_id: TTupleId,
    pub key_column_name: Vec<String>,
    pub is_preaggregation: bool,
}

/// Plan node (from PlanNodes.thrift)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPlanNode {
    pub node_id: TPlanNodeId,
    pub node_type: TPlanNodeType,
    pub num_children: i32,
    pub limit: i64,
    pub row_tuples: Vec<TTupleId>,
    pub nullable_tuples: Vec<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub olap_scan_node: Option<TOlapScanNode>,
}

/// Plan (collection of nodes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPlan {
    pub nodes: Vec<TPlanNode>,
}

/// Plan fragment (from Planner.thrift)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPlanFragment {
    pub plan: TPlan,
}

impl TPlanFragment {
    /// Serialize to JSON for comparison with Java FE
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize to compact JSON
    pub fn to_json_compact(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_node_serialization() {
        let node = TPlanNode {
            node_id: 0,
            node_type: TPlanNodeType::OlapScanNode,
            num_children: 0,
            limit: -1,
            row_tuples: vec![0],
            nullable_tuples: vec![false],
            olap_scan_node: Some(TOlapScanNode {
                tuple_id: 0,
                key_column_name: vec!["l_orderkey".to_string(), "l_partkey".to_string()],
                is_preaggregation: true,
            }),
        };

        let json = serde_json::to_string_pretty(&node).unwrap();
        println!("TPlanNode JSON:\n{}", json);

        // Verify it deserializes back
        let decoded: TPlanNode = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.node_id, 0);
        assert_eq!(decoded.node_type, TPlanNodeType::OlapScanNode);
    }

    #[test]
    fn test_plan_fragment_serialization() {
        let fragment = TPlanFragment {
            plan: TPlan {
                nodes: vec![
                    TPlanNode {
                        node_id: 0,
                        node_type: TPlanNodeType::OlapScanNode,
                        num_children: 0,
                        limit: -1,
                        row_tuples: vec![0],
                        nullable_tuples: vec![false],
                        olap_scan_node: Some(TOlapScanNode {
                            tuple_id: 0,
                            key_column_name: vec!["l_orderkey".to_string()],
                            is_preaggregation: true,
                        }),
                    }
                ],
            },
        };

        let json = fragment.to_json().unwrap();
        println!("TPlanFragment JSON:\n{}", json);

        // Verify structure
        assert!(json.contains("\"node_type\": \"OlapScanNode\""));
        assert!(json.contains("\"tuple_id\": 0"));
    }
}
