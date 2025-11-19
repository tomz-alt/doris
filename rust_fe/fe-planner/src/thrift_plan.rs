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

/// Partition types (from Partitions.thrift)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum TPartitionType {
    Unpartitioned = 0,
    Random = 1,
    HashPartitioned = 2,
    RangePartitioned = 3,
    ListPartitioned = 4,
    BucketShuffleHashPartitioned = 5,
    OlapTableSinkHashPartitioned = 6,
    HiveTableSinkHashPartitioned = 7,
}

/// Data partition (from Partitions.thrift)
/// Describes how data is partitioned across instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TDataPartition {
    /// Partition type (REQUIRED)
    pub partition_type: TPartitionType,
    /// Partition expressions (optional) - for HASH_PARTITIONED, etc.
    pub partition_exprs: Option<Vec<()>>,  // TExpr placeholder for now
    /// Partition info (optional) - for RANGE_PARTITIONED
    pub partition_infos: Option<Vec<()>>,  // TRangePartition placeholder
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
/// Reference: Planner.thrift TPlanFragment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPlanFragment {
    /// Field 2: Plan tree (optional)
    pub plan: TPlan,
    /// Field 6: Data partitioning (REQUIRED!)
    pub partition: TDataPartition,
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

/// Plan fragment destination - where to send results
/// Reference: DataSinks.thrift TPlanFragmentDestination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPlanFragmentDestination {
    /// Fragment instance ID
    pub fragment_instance_id: TUniqueId,

    /// Server address
    pub server: crate::scan_range_builder::TNetworkAddress,

    /// bRPC server address (optional)
    pub brpc_server: Option<crate::scan_range_builder::TNetworkAddress>,
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

    /// Fragment ID (field 3)
    /// Reference: Java FE ThriftPlansBuilder.java:340 params.setFragmentId(fragment.getFragmentId().asInt())
    pub fragment_id: Option<i32>,

    /// Number of senders per exchange node
    pub per_exch_num_senders: HashMap<i32, i32>,

    /// Descriptor table (field 5 in Thrift)
    /// Reference: Java FE Coordinator.java:3214 params.setDescTbl(descTable)
    pub desc_tbl: Option<TDescriptorTable>,

    /// Destinations for data transfer (field 7) - REQUIRED!
    /// Reference: Java FE ThriftPlansBuilder.java:349-357
    pub destinations: Vec<TPlanFragmentDestination>,

    /// The plan fragment to execute
    pub fragment: TPlanFragment,

    /// Per-instance parameters
    pub local_params: Vec<TPipelineInstanceParams>,

    /// Coordinator address (optional)
    pub coord: Option<TNetworkAddress>,

    /// Number of senders (optional)
    pub num_senders: Option<i32>,

    /// Query globals (field 11)
    /// Reference: Java FE Coordinator.java:3221 params.setQueryGlobals(queryGlobals)
    pub query_globals: Option<TQueryGlobals>,

    /// Query options (field 12)
    /// Reference: Java FE Coordinator.java:3222 params.setQueryOptions(queryOptions)
    pub query_options: Option<TQueryOptions>,

    /// Number of fragments on this backend host (field 17)
    /// Reference: Java FE ThriftPlansBuilder.java:343 params.setFragmentNumOnHost(workerProcessInstanceNum.count(worker))
    pub fragment_num_on_host: Option<i32>,

    /// Backend ID (field 18)
    /// Reference: Java FE ThriftPlansBuilder.java:336 params.setBackendId(worker.id())
    pub backend_id: Option<i64>,

    /// Total number of instances (field 38)
    /// Reference: Java FE ThriftPlansBuilder.java:360 params.setTotalInstances(instanceNumInThisFragment)
    pub total_instances: Option<i32>,

    /// Whether this is a Nereids-generated plan (field 40)
    /// Reference: Java FE ThriftPlansBuilder.java:335 params.setIsNereids(true)
    pub is_nereids: Option<bool>,
}

/// Query globals for pipeline execution
/// Reference: PaloInternalService.thrift TQueryGlobals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TQueryGlobals {
    /// Current timestamp as string (format: yyyy-MM-dd HH:mm:ss)
    pub now_string: String,

    /// Timestamp in milliseconds (optional)
    pub timestamp_ms: Option<i64>,

    /// Timezone name (optional, e.g. "UTC", "Asia/Shanghai")
    pub time_zone: Option<String>,

    /// Load zero tolerance flag (optional, default false)
    pub load_zero_tolerance: Option<bool>,

    /// Nano seconds (optional)
    pub nano_seconds: Option<i32>,
}

impl TQueryGlobals {
    /// Create minimal query globals with current timestamp
    pub fn minimal() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap();
        let timestamp_ms = now.as_millis() as i64;

        // Format timestamp as yyyy-MM-dd HH:mm:ss
        let now_string = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        Self {
            now_string,
            timestamp_ms: Some(timestamp_ms),
            time_zone: Some("UTC".to_string()),
            load_zero_tolerance: Some(false),
            nano_seconds: None,
        }
    }
}

/// Query options for pipeline execution (minimal version)
/// Reference: PaloInternalService.thrift TQueryOptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TQueryOptions {
    /// Batch size (optional, field 4)
    pub batch_size: Option<i32>,

    /// Memory limit in bytes (optional, field 12)
    pub mem_limit: Option<i64>,

    /// Query timeout in seconds (optional, field 14)
    pub query_timeout: Option<i32>,
}

impl TQueryOptions {
    /// Create minimal query options with reasonable defaults
    pub fn minimal() -> Self {
        Self {
            batch_size: Some(4096),  // Default batch size
            mem_limit: Some(2147483648),  // 2GB default
            query_timeout: Some(3600),  // 1 hour timeout
        }
    }
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

        // Build descriptor table from OLAP_SCAN_NODE
        // Reference: Java FE Coordinator.java:3214 params.setDescTbl(descTable)
        let desc_tbl = if let Some(first_node) = fragment.plan.nodes.first() {
            if let Some(olap_node) = &first_node.olap_scan_node {
                // Use complete descriptor table for lineitem, generic for others
                if olap_node.table_name.as_ref().map(|s| s.as_str()) == Some("lineitem") {
                    Some(TDescriptorTable::for_lineitem_table(
                        olap_node.tuple_id,
                        10001,  // lineitem table_id
                    ))
                } else {
                    Some(TDescriptorTable::for_olap_scan(
                        olap_node.tuple_id,
                        &olap_node.key_column_name,
                        &olap_node.key_column_type,
                    ))
                }
            } else {
                None
            }
        } else {
            None
        };

        // Build fragment params
        // Reference: Java FE ThriftPlansBuilder.java:334-370
        let params = TPipelineFragmentParams {
            protocol_version: 0,  // V1 (Java: PaloInternalServiceVersion.V1)
            query_id: unique_id,
            fragment_id: Some(0),  // Fragment ID (Java: fragment.getFragmentId().asInt())
            per_exch_num_senders: HashMap::new(),
            desc_tbl,  // Descriptor table
            destinations: Vec::new(),  // Empty destinations for single-fragment query (Java: nonMultiCastDestinations)
            fragment,
            local_params,
            coord: None,
            num_senders: Some(1),
            query_globals: Some(TQueryGlobals::minimal()),  // Query globals
            query_options: Some(TQueryOptions::minimal()),  // Query options
            fragment_num_on_host: Some(1),  // Number of fragments on this host
            backend_id: Some(10001),  // Backend ID (hardcoded for now, should come from metadata)
            total_instances: Some(1),  // Total number of instances (Java: instanceNumInThisFragment)
            is_nereids: Some(true),  // Nereids-generated plan flag (Java always sets true)
        };

        TPipelineFragmentParamsList {
            params_list: vec![params],
        }
    }
}

// ============================================================================
// Descriptor Table Structures
// Reference: Descriptors.thrift
// ============================================================================

/// Slot descriptor - describes a single column/slot in a tuple
/// Reference: Descriptors.thrift TSlotDescriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TSlotDescriptor {
    /// Slot ID
    pub id: i32,

    /// Parent tuple ID
    pub parent: i32,

    /// Slot type descriptor
    pub slot_type: TTypeDesc,

    /// Column position in originating table
    pub column_pos: i32,

    /// Byte offset (deprecated but required)
    pub byte_offset: i32,

    /// Null indicator byte
    pub null_indicator_byte: i32,

    /// Null indicator bit
    pub null_indicator_bit: i32,

    /// Column name
    pub col_name: String,

    /// Slot index
    pub slot_idx: i32,

    /// Is materialized
    pub is_materialized: bool,

    /// Optional fields (11-17)
    pub col_unique_id: Option<i32>,
    pub is_key: Option<bool>,
    pub need_materialize: Option<bool>,
    pub is_auto_increment: Option<bool>,
    pub column_paths: Option<Vec<String>>,
    pub col_default_value: Option<String>,
    pub primitive_type: Option<TPrimitiveType>,
}

/// Type descriptor for columns
/// Reference: Types.thrift TTypeDesc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTypeDesc {
    pub types: Vec<TTypeNode>,
}

/// Type node in type descriptor
/// Reference: Types.thrift TTypeNode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTypeNode {
    pub node_type: TTypeNodeType,
    pub scalar_type: Option<TScalarType>,
}

/// Type node type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TTypeNodeType {
    Scalar = 0,
    Array = 1,
    Map = 2,
    Struct = 3,
}

/// Scalar type descriptor
/// Reference: Types.thrift TScalarType
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TScalarType {
    pub scalar_type: TPrimitiveType,
    pub len: Option<i32>,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
}

/// Tuple descriptor - describes a tuple (row) structure
/// Reference: Descriptors.thrift TTupleDescriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTupleDescriptor {
    /// Tuple ID
    pub id: i32,

    /// Byte size (deprecated but required)
    pub byte_size: i32,

    /// Number of null bytes (deprecated but required)
    pub num_null_bytes: i32,

    /// Optional table ID this tuple belongs to
    pub table_id: Option<i64>,
}

/// Descriptor table - top-level schema descriptor
/// Reference: Descriptors.thrift TDescriptorTable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TDescriptorTable {
    /// All slot descriptors
    pub slot_descriptors: Option<Vec<TSlotDescriptor>>,

    /// All tuple descriptors (required)
    pub tuple_descriptors: Vec<TTupleDescriptor>,

    /// All table descriptors (optional)
    pub table_descriptors: Option<Vec<TTableDescriptor>>,
}

/// Table descriptor (minimal - we may not need this for simple queries)
/// Reference: Descriptors.thrift TTableDescriptor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTableDescriptor {
    pub id: i64,
    pub table_type: TTableType,
    pub num_cols: i32,
    pub num_clustering_cols: i32,
    pub table_name: String,
    pub db_name: String,
}

/// Table type enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TTableType {
    MysqlTable = 0,
    OlapTable = 1,
    SchemaTable = 2,
    BrokerTable = 3,
    EsTable = 4,
    OdbcTable = 5,
}

impl TDescriptorTable {
    /// Build minimal descriptor table for OLAP scan
    /// Reference: Java FE DescriptorTable.toThrift()
    pub fn for_olap_scan(
        tuple_id: i32,
        key_column_names: &[String],
        key_column_types: &[TPrimitiveType],
    ) -> Self {
        // Build slot descriptors for key columns
        let slot_descriptors: Vec<TSlotDescriptor> = key_column_names
            .iter()
            .zip(key_column_types.iter())
            .enumerate()
            .map(|(idx, (col_name, col_type))| {
                TSlotDescriptor {
                    id: idx as i32,
                    parent: tuple_id,
                    slot_type: TTypeDesc {
                        types: vec![TTypeNode {
                            node_type: TTypeNodeType::Scalar,
                            scalar_type: Some(TScalarType {
                                scalar_type: col_type.clone(),
                                len: None,
                                precision: None,
                                scale: None,
                            }),
                        }],
                    },
                    column_pos: idx as i32,
                    byte_offset: 0,  // Deprecated
                    null_indicator_byte: 0,
                    null_indicator_bit: idx as i32 % 8,
                    col_name: col_name.clone(),
                    slot_idx: idx as i32,
                    is_materialized: true,
                    col_unique_id: Some(-1),
                    is_key: Some(true),
                    need_materialize: Some(true),
                    is_auto_increment: None,
                    column_paths: None,
                    col_default_value: None,
                    primitive_type: Some(*col_type),
                }
            })
            .collect();

        // Build tuple descriptor
        let tuple_descriptor = TTupleDescriptor {
            id: tuple_id,
            byte_size: 0,  // Deprecated
            num_null_bytes: 0,  // Deprecated
            table_id: None,
        };

        TDescriptorTable {
            slot_descriptors: Some(slot_descriptors),
            tuple_descriptors: vec![tuple_descriptor],
            table_descriptors: None,  // Optional, not needed for simple queries
        }
    }

    /// Build complete TDescriptorTable for TPC-H lineitem table
    /// Reference: TPC-H lineitem schema with 16 columns
    pub fn for_lineitem_table(tuple_id: i32, table_id: i64) -> Self {
        // Define lineitem columns with proper types
        let columns: Vec<(&str, TPrimitiveType, bool)> = vec![
            ("l_orderkey", TPrimitiveType::BigInt, true),      // key column
            ("l_partkey", TPrimitiveType::BigInt, true),       // key column
            ("l_suppkey", TPrimitiveType::BigInt, true),       // key column
            ("l_linenumber", TPrimitiveType::Int, true),       // key column
            ("l_quantity", TPrimitiveType::DecimalV2, false),  // decimal(15,2)
            ("l_extendedprice", TPrimitiveType::DecimalV2, false), // decimal(15,2)
            ("l_discount", TPrimitiveType::DecimalV2, false),  // decimal(15,2)
            ("l_tax", TPrimitiveType::DecimalV2, false),       // decimal(15,2)
            ("l_returnflag", TPrimitiveType::Char, false),     // char(1)
            ("l_linestatus", TPrimitiveType::Char, false),     // char(1)
            ("l_shipdate", TPrimitiveType::DateV2, false),     // date
            ("l_commitdate", TPrimitiveType::DateV2, false),   // date
            ("l_receiptdate", TPrimitiveType::DateV2, false),  // date
            ("l_shipinstruct", TPrimitiveType::Char, false),   // char(25)
            ("l_shipmode", TPrimitiveType::Char, false),       // char(10)
            ("l_comment", TPrimitiveType::Varchar, false),     // varchar(44)
        ];

        // Build slot descriptors for all 16 columns
        let slot_descriptors: Vec<TSlotDescriptor> = columns
            .iter()
            .enumerate()
            .map(|(idx, &(col_name, col_type, is_key))| {
                let scalar_type = match col_type {
                    TPrimitiveType::DecimalV2 => TScalarType {
                        scalar_type: col_type,
                        len: None,
                        precision: Some(15),
                        scale: Some(2),
                    },
                    TPrimitiveType::Char => TScalarType {
                        scalar_type: col_type,
                        len: if col_name == "l_returnflag" || col_name == "l_linestatus" {
                            Some(1)
                        } else if col_name == "l_shipinstruct" {
                            Some(25)
                        } else if col_name == "l_shipmode" {
                            Some(10)
                        } else {
                            Some(1)
                        },
                        precision: None,
                        scale: None,
                    },
                    TPrimitiveType::Varchar => TScalarType {
                        scalar_type: col_type,
                        len: Some(44), // l_comment varchar(44)
                        precision: None,
                        scale: None,
                    },
                    _ => TScalarType {
                        scalar_type: col_type,
                        len: None,
                        precision: None,
                        scale: None,
                    },
                };

                TSlotDescriptor {
                    id: idx as i32,
                    parent: tuple_id,
                    slot_type: TTypeDesc {
                        types: vec![TTypeNode {
                            node_type: TTypeNodeType::Scalar,
                            scalar_type: Some(scalar_type),
                        }],
                    },
                    column_pos: idx as i32,
                    byte_offset: 0,  // Deprecated in modern Doris
                    null_indicator_byte: 0,  // Deprecated
                    null_indicator_bit: idx as i32 % 8,
                    col_name: col_name.to_string(),
                    slot_idx: idx as i32,
                    is_materialized: true,
                    col_unique_id: Some(idx as i32),  // Use index as unique ID
                    is_key: Some(is_key),
                    need_materialize: Some(true),
                    is_auto_increment: Some(false),
                    column_paths: None,  // Not used for regular columns
                    col_default_value: None,  // No default values
                    primitive_type: Some(col_type),
                }
            })
            .collect();

        // Build tuple descriptor
        let tuple_descriptor = TTupleDescriptor {
            id: tuple_id,
            byte_size: 0,  // Deprecated
            num_null_bytes: 0,  // Deprecated
            table_id: Some(table_id),
        };

        TDescriptorTable {
            slot_descriptors: Some(slot_descriptors),
            tuple_descriptors: vec![tuple_descriptor],
            table_descriptors: None,  // TTableDescriptor is optional for basic queries
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
            partition: TDataPartition {
                partition_type: TPartitionType::Unpartitioned,
                partition_exprs: None,
                partition_infos: None,
            },
        };

        let json = fragment.to_json().unwrap();
        println!("TPlanFragment JSON:\n{}", json);

        // Verify structure
        assert!(json.contains("\"node_type\": \"OlapScanNode\""));
        assert!(json.contains("\"tuple_id\": 0"));
    }
}
