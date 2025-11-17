//! Decode Java FE `TPipelineFragmentParamsList` payloads using the
//! `compact-thrift-runtime` crate (Apache Thrift compact protocol).
//!
//! This is intended to validate that `compact-thrift-runtime` can
//! correctly decode Doris Java FE BE payloads, as a compatibility
//! check before switching the Rust FE encoder.
//!
//! Usage:
//!   cargo run --no-default-features --features real_be_proto \
//!     --example decode_java_compact -- /path/to/java-fe-thrift.bin

use std::env;
use std::fs::File;
use std::io::Read;

use compact_thrift_runtime::{CompactThriftInputSlice, CompactThriftProtocol, thrift};

// Minimal Thrift model for the fields we care about in
// `PaloInternalService.TPipelineFragmentParamsList` and nested types.
//
// Unknown fields are skipped by the generated decoder, so this can
// safely be a subset of the full Doris Thrift schema.
thrift! {
    /// Mirror of Types.TUniqueId { 1: i64 hi, 2: i64 lo }.
    struct Types_TUniqueId {
        1: required i64 hi;
        2: required i64 lo;
    }

    /// Mirror of Types.TNetworkAddress { 1: string hostname, 2: i32 port }.
    struct Types_TNetworkAddress {
        1: required string hostname;
        2: required i32 port;
    }

    /// Minimal TQueryGlobals { 1: string now_string, 2: i64 timestamp_ms, 3: string time_zone }.
    struct TQueryGlobals {
        1: required string now_string;
        2: optional i64 timestamp_ms;
        3: optional string time_zone;
    }

    /// Minimal TQueryOptions subset.
    struct TQueryOptions {
        4: optional i32 batch_size;
        14: optional i32 query_timeout;
        57: optional bool enable_pipeline_engine;
    }

    /// Minimal TPipelineInstanceParams with only fragment_instance_id.
    struct TPipelineInstanceParams {
        1: required Types_TUniqueId fragment_instance_id;
    }

    /// Minimal PlanNodes.TDataPartition { 1: i32 partition_type }.
    struct TDataPartition {
        1: required i32 partition_type;
    }

    /// Minimal TOlapScanNode for simple scans.
    struct TOlapScanNode {
        1: required i32 tuple_id;
        2: optional list<binary> key_column_name;
        3: optional list<i32> key_column_type;
        4: optional bool is_preaggregation;
        7: optional string table_name;
    }

    /// Minimal TPlanNode with a single OLAP_SCAN_NODE.
    struct TPlanNode {
        1: required i32 node_id;
        2: required i32 node_type;
        3: required i32 num_children;
        4: required i64 limit;
        5: optional list<i32> row_tuples;
        6: optional list<bool> nullable_tuples;
        8: optional bool compact_data;
        18: optional TOlapScanNode olap_scan_node;
    }

    /// Minimal Planner.TPlan { 1: list<TPlanNode> nodes }.
    struct TPlan {
        1: optional list<TPlanNode> nodes;
    }

    /// Minimal Planner.TPlanFragment { 2: TPlan plan, 6: TDataPartition partition }.
    struct TPlanFragment {
        2: optional TPlan plan;
        6: optional TDataPartition partition;
    }

    /// Minimal Types.TScalarType / TTypeNode / TTypeDesc for BIGINT/DECIMAL/DATE strings.
    struct TScalarType {
        1: required i32 type;
        2: optional i32 len;
        3: optional i32 precision;
        4: optional i32 scale;
    }

    struct TTypeNode {
        1: required i32 type;
        2: optional TScalarType scalar_type;
    }

    struct TTypeDesc {
        1: optional list<TTypeNode> types;
        2: optional bool is_nullable;
    }

    /// Minimal TSlotDescriptor / TTupleDescriptor / TDescriptorTable.
    struct TSlotDescriptor {
        1: required i32 id;
        2: required i32 parent;
        3: required TTypeDesc slotType;
        4: required i32 columnPos;
        5: required i32 byteOffset;
        6: required i32 nullIndicatorByte;
        7: required i32 nullIndicatorBit;
        8: required string colName;
        9: required i32 slotIdx;
        10: required bool isMaterialized;
    }

    struct TTupleDescriptor {
        1: required i32 id;
        2: required i32 byteSize;
        3: required i32 numNullBytes;
        4: optional i64 tableId;
        5: optional i32 numNullSlots;
    }

    struct TDescriptorTable {
        1: optional list<TSlotDescriptor> slotDescriptors;
        2: optional list<TTupleDescriptor> tupleDescriptors;
    }

    /// Minimal subset of PaloInternalService.TPipelineFragmentParams.
    struct TPipelineFragmentParams {
        1: required i32 protocol_version;
        2: required Types_TUniqueId query_id;
        3: optional i32 fragment_id;
        5: optional TDescriptorTable desc_tbl;
        10: optional Types_TNetworkAddress coord;
        11: optional TQueryGlobals query_globals;
        12: optional TQueryOptions query_options;
        21: optional bool is_simplified_param;
        23: optional TPlanFragment fragment;
        24: optional list<TPipelineInstanceParams> local_params;
        28: optional string table_name;
        40: optional bool is_nereids;
    }

    /// Minimal subset of PaloInternalService.TPipelineFragmentParamsList.
    struct TPipelineFragmentParamsList {
        1: optional list<TPipelineFragmentParams> params_list;
        4: optional Types_TNetworkAddress coord;
        5: optional TQueryGlobals query_globals;
        8: optional TQueryOptions query_options;
        9: optional bool is_nereids;
        11: optional Types_TUniqueId query_id;
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .expect("usage: decode_java_compact <file>");

    let mut buf = Vec::new();
    File::open(&path)?.read_to_end(&mut buf)?;

    let mut input = CompactThriftInputSlice::new(&buf);
    let value = TPipelineFragmentParamsList::read_thrift(&mut input)
        .map_err(|e| format!("failed to decode compact Thrift payload: {e}"))?;

    let num_params = value
        .params_list
        .as_ref()
        .map(|v| v.len())
        .unwrap_or(0);

    println!("Successfully decoded '{}'", path);
    println!("  params_list.len = {num_params}");

    if let Some(params) = value.params_list.as_ref().and_then(|v| v.get(0)) {
        println!("  [0].protocol_version = {}", params.protocol_version);
        println!("  [0].fragment_id      = {:?}", params.fragment_id);
        println!("  [0].table_name       = {:?}", params.table_name);
        println!("  [0].is_nereids       = {:?}", params.is_nereids);

        if let Some(ref qg) = params.query_globals {
            println!("  [0].query_globals.now_string = {}", qg.now_string);
            println!("  [0].query_globals.time_zone  = {:?}", qg.time_zone);
        } else {
            println!("  [0].query_globals = <none>");
        }

        if let Some(ref qo) = params.query_options {
            println!(
                "  [0].query_options.batch_size / timeout / enable_pipeline_engine = {:?} / {:?} / {:?}",
                qo.batch_size, qo.query_timeout, qo.enable_pipeline_engine
            );
        } else {
            println!("  [0].query_options = <none>");
        }

        if let Some(ref desc) = params.desc_tbl {
            let slots = desc.slotDescriptors.as_ref().map(|v| v.len()).unwrap_or(0);
            let tuples = desc.tupleDescriptors.as_ref().map(|v| v.len()).unwrap_or(0);
            println!("  [0].desc_tbl.slotDescriptors.len   = {slots}");
            println!("  [0].desc_tbl.tupleDescriptors.len  = {tuples}");
        } else {
            println!("  [0].desc_tbl = <none>");
        }

        if let Some(ref frag) = params.fragment {
            let nodes = frag.plan.as_ref()
                .and_then(|p| p.nodes.as_ref())
                .map(|v| v.len())
                .unwrap_or(0);
            println!("  [0].fragment.plan.nodes.len        = {nodes}");
        } else {
            println!("  [0].fragment = <none>");
        }

        let local_params_len = params.local_params.as_ref().map(|v| v.len()).unwrap_or(0);
        println!("  [0].local_params.len               = {local_params_len}");
    }

    Ok(())
}
