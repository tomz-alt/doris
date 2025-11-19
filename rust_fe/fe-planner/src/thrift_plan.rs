// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Thrift Plan Structures
//!
//! Manual Thrift bindings for Doris plan structures.
//! These match the .thrift definitions exactly.

use serde::{Deserialize, Serialize};

/// Primitive types (from Types.thrift)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum TPrimitiveType {
    InvalidType = 0,
    NullType = 1,
    Boolean = 2,
    TinyInt = 3,
    SmallInt = 4,
    Int = 5,
    BigInt = 6,
    Float = 7,
    Double = 8,
    Date = 9,
    DateTime = 10,
    Binary = 11,
    DecimalDeprecated = 12,
    Char = 13,
    LargeInt = 14,
    Varchar = 15,
    Hll = 16,
    DecimalV2 = 17,
    Bitmap = 19,
    Array = 20,
    Map = 21,
    Struct = 22,
    String = 23,
    All = 24,
    QuantileState = 25,
    DateV2 = 26,
    DateTimeV2 = 27,
    TimeV2 = 28,
    Decimal32 = 29,
    Decimal64 = 30,
    Decimal128I = 31,
    Decimal256 = 32,
    JsonB = 33,
    Variant = 34,
    AggState = 35,
    LambdaFunction = 36,
    Ipv4 = 37,
    Ipv6 = 38,
}

// Import TNetworkAddress from scan_range_builder to avoid duplication
pub use crate::scan_range_builder::TNetworkAddress;

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

/// Tablet ID
pub type TTabletId = i64;

/// Tuple ID
pub type TTupleId = i32;

/// Plan node ID
pub type TPlanNodeId = i32;

/// OLAP scan node (from PlanNodes.thrift)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TOlapScanNode {
    pub tuple_id: TTupleId,
    pub key_column_name: Vec<String>,
    pub key_column_type: Vec<TPrimitiveType>,
    pub is_preaggregation: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
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
    pub compact_data: bool,
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

// ============================================================================
// Pipeline Execution Structures (VERSION_3)
// Reference: PaloInternalService.thrift, Coordinator.java:3185-3350
// ============================================================================

use std::collections::HashMap;

/// Unique ID for queries and fragments (16 bytes total)
/// Reference: Types.thrift TUniqueId
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TUniqueId {
    pub hi: i64,
    pub lo: i64,
}

impl TUniqueId {
    /// Create from 16-byte array
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self {
            hi: i64::from_be_bytes(bytes[0..8].try_into().unwrap()),
            lo: i64::from_be_bytes(bytes[8..16].try_into().unwrap()),
        }
    }
}

/// Scan range parameters wrapper
/// Reference: PlanNodes.thrift TScanRangeParams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TScanRangeParams {
    pub scan_range: crate::scan_range_builder::TScanRange,
    pub volume_id: Option<i32>,
}

/// Pipeline instance parameters (per execution instance)
/// Reference: PaloInternalService.thrift TPipelineInstanceParams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPipelineInstanceParams {
    /// Unique fragment instance ID
    pub fragment_instance_id: TUniqueId,

    /// Scan ranges per plan node (node_id -> scan_ranges)
    pub per_node_scan_ranges: HashMap<i32, Vec<TScanRangeParams>>,

    /// Sender ID for this instance
    pub sender_id: Option<i32>,

    /// Backend number
    pub backend_num: Option<i32>,
}

/// Pipeline fragment parameters (per backend)
/// Reference: PaloInternalService.thrift TPipelineFragmentParams
/// Java FE: Coordinator.java:3209
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPipelineFragmentParams {
    /// Protocol version (0 = V1)
    pub protocol_version: i32,

    /// Query ID
    pub query_id: TUniqueId,

    /// Number of senders per exchange node
    pub per_exch_num_senders: HashMap<i32, i32>,

    /// The plan fragment to execute
    pub fragment: TPlanFragment,

    /// Per-instance parameters
    pub local_params: Vec<TPipelineInstanceParams>,

    /// Coordinator address (optional)
    pub coord: Option<TNetworkAddress>,

    /// Number of senders (optional)
    pub num_senders: Option<i32>,
}

/// Pipeline fragment parameters list (VERSION_3 top-level)
/// Reference: PaloInternalService.thrift TPipelineFragmentParamsList
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPipelineFragmentParamsList {
    /// List of pipeline parameters (one per backend usually)
    pub params_list: Vec<TPipelineFragmentParams>,
}

impl TPipelineFragmentParamsList {
    /// Create minimal pipeline params from fragment and scan ranges
    /// Reference: Java FE Coordinator.java:3185-3350 (toThrift method)
    pub fn from_fragment_and_ranges(
        fragment: TPlanFragment,
        query_id: [u8; 16],
        node_id: i32,
        scan_ranges: Vec<crate::scan_range_builder::TScanRangeLocations>,
    ) -> Self {
        let unique_id = TUniqueId::from_bytes(query_id);

        // Build scan range params (wrap TScanRange in TScanRangeParams)
        let scan_params: Vec<TScanRangeParams> = scan_ranges
            .into_iter()
            .map(|loc| TScanRangeParams {
                scan_range: loc.scan_range,
                volume_id: None,
            })
            .collect();

        // Map node_id to scan ranges
        let mut per_node_scan_ranges = HashMap::new();
        per_node_scan_ranges.insert(node_id, scan_params);

        // Build instance params (one instance for simple case)
        let local_params = vec![TPipelineInstanceParams {
            fragment_instance_id: unique_id.clone(),
            per_node_scan_ranges,
            sender_id: Some(0),
            backend_num: Some(0),
        }];

        // Build fragment params
        let params = TPipelineFragmentParams {
            protocol_version: 0,  // V1
            query_id: unique_id,
            per_exch_num_senders: HashMap::new(),
            fragment,
            local_params,
            coord: None,
            num_senders: Some(1),
        };

        TPipelineFragmentParamsList {
            params_list: vec![params],
        }
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
            compact_data: true,
            olap_scan_node: Some(TOlapScanNode {
                tuple_id: 0,
                key_column_name: vec!["l_orderkey".to_string(), "l_partkey".to_string()],
                key_column_type: vec![TPrimitiveType::Int, TPrimitiveType::Int],
                is_preaggregation: true,
                table_name: Some("lineitem".to_string()),
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
                        compact_data: true,
                        olap_scan_node: Some(TOlapScanNode {
                            tuple_id: 0,
                            key_column_name: vec!["l_orderkey".to_string()],
                            key_column_type: vec![TPrimitiveType::Int],
                            is_preaggregation: true,
                            table_name: Some("lineitem".to_string()),
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
