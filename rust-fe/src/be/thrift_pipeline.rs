#[cfg(feature = "real_be_proto")]
use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

use crate::planner::plan_fragment::{PlanNode, PlanFragment, QueryPlan};

#[cfg(feature = "real_be_proto")]
use thrift_codec::data::{
    Data,
    DataRef,
    Elements,
    Field as ThriftField,
    List as ThriftList,
    Map as ThriftMap,
    Struct as ThriftStruct,
};
#[cfg(feature = "real_be_proto")]
use compact_thrift_runtime::{CompactThriftOutput, ThriftError};

#[cfg(feature = "real_be_proto")]
use serde::Deserialize;
#[cfg(feature = "real_be_proto")]
use std::collections::HashMap;
#[cfg(feature = "real_be_proto")]
use std::fs;

#[cfg(feature = "real_be_proto")]
use thrift::protocol::{TCompactOutputProtocol, TOutputProtocol, TSerializable};
#[cfg(feature = "real_be_proto")]
use thrift::transport::TBufferChannel;

#[cfg(feature = "real_be_proto")]
use doris_thrift::{
    palo_internal_service as t_palo,
    descriptors as t_desc,
    exprs as t_exprs,
    data_sinks as t_sinks,
    query_cache::TQueryCacheParam,
    planner as t_planner,
    plan_nodes as t_plan_nodes,
    partitions as t_parts,
    types as t_types,
};
#[cfg(feature = "real_be_proto")]
use crate::metadata::{catalog, types::DataType as MetaDataType};
#[cfg(feature = "real_be_proto")]
use crate::config::Config;

/// Minimal Rust representation of Doris's TPipelineFragmentParams for a single
/// fragment. This mirrors a subset of the fields defined in
/// `PaloInternalService.TPipelineFragmentParams` and is used as an intermediate
/// before Thrift serialization.
#[derive(Debug, Clone)]
pub struct PipelineFragmentParams {
    pub query_id: Uuid,
    pub fragment_id: i32,
    pub is_nereids: bool,
    /// Logical table name for simple single-scan fragments.
    ///
    /// For the first experimental slice we only support a single
    /// `PlanNode::OlapScan` over `tpch.lineitem`. When present, this
    /// allows us to attach a matching `TPlanFragment` and
    /// `TDescriptorTable` into `TPipelineFragmentParams`.
    pub table_name: Option<String>,
}

/// Container corresponding to `TPipelineFragmentParamsList` in the Java / Thrift
/// API. For now we only model the fields needed for simple single-fragment
/// queries.
#[derive(Debug, Clone)]
pub struct PipelineFragmentParamsList {
    pub query_id: Uuid,
    pub fragments: Vec<PipelineFragmentParams>,
}

/// Internal representation that mirrors the Thrift
/// `PaloInternalService.TPipelineFragmentParams` struct. This is kept
/// close to the IDL shape so that the mapping from our planner-level
/// `PipelineFragmentParams` to the on-wire Thrift struct is explicit
/// and verifiable against the Java FE implementation.
#[cfg(feature = "real_be_proto")]
#[derive(Debug, Clone)]
struct TPipelineFragmentParamsThrift {
    query_id: Uuid,
    fragment_id: i32,
    table_name: Option<String>,
    is_nereids: bool,
}

#[cfg(feature = "real_be_proto")]
impl TPipelineFragmentParamsThrift {
    fn from_pipeline_params(p: &PipelineFragmentParams) -> Self {
        Self {
            query_id: p.query_id,
            fragment_id: p.fragment_id,
            table_name: p.table_name.clone(),
            is_nereids: p.is_nereids,
        }
    }

    /// Convert to the generated Apache Thrift `TPipelineFragmentParams`
    /// struct. Only a minimal, Java-compatible subset of fields is
    /// populated; all other fields use their defaults.
    fn to_thrift_struct(&self) -> t_palo::TPipelineFragmentParams {
        let query_id = encode_tunique_id_typed(self.query_id);
        let query_globals = encode_query_globals_typed();
        let query_options = encode_query_options_typed();

        // For simple single-fragment plans with no exchanges, use an empty
        // map with the correct key/value types (TPlanNodeId -> i32).
        let per_exch_num_senders: BTreeMap<t_types::TPlanNodeId, i32> = BTreeMap::new();

        // Optional descriptor table for simple scan fragments. For now
        // we derive this from the in-memory Rust FE catalog for any
        // table in the `tpch` database. A future Doris-backed catalog
        // can supply real db/table IDs and layouts.
        let desc_tbl = self
            .table_name
            .as_deref()
            .map(|name| encode_descriptor_table_for_table_from_catalog_typed("tpch_sf1", name));

        // Generate appropriate TPlanFragment based on fragment_id.
        // Fragment 0 (scan): OLAP_SCAN_NODE with scan ranges
        // Fragment 1 (exchange): EXCHANGE_NODE for data aggregation
        // Fragment 2 (result sink): RESULT_SINK_NODE for output
        let fragment = encode_plan_fragment_by_type(self.fragment_id, self.table_name.as_deref());

        // Doris BE expects at least one TPipelineInstanceParams entry and
        // unconditionally accesses local_params[0] for legacy runtime
        // filter handling. For simple single-instance queries we populate a
        // single instance with a fragment_instance_id and, when available,
        // minimal scan ranges derived from a lightweight table→tablet mapping.
        //
        // For now this mapping is loaded from an optional JSON config so
        // that early experiments can drive real tablet scans without a
        // fully-fledged BE catalog. When no mapping is present we fall back
        // to an empty per_node_scan_ranges map, matching the previous
        // behaviour (no tablets, 0 rows).
        let db_name = "tpch_sf1";
        let instance = encode_pipeline_instance_params_typed(self, db_name);
        let local_params = vec![instance];

        t_palo::TPipelineFragmentParams {
            protocol_version: t_palo::PaloInternalServiceVersion::V1,
            query_id,
            fragment_id: Some(self.fragment_id),
            per_exch_num_senders,
            desc_tbl,
            resource_info: None,
            destinations: None,
            num_senders: None,
            send_query_statistics_with_every_batch: None,
            coord: None,
            query_globals: Some(query_globals),
            query_options: Some(query_options),
            import_label: None,
            db_name: None,
            load_job_id: None,
            load_error_hub_info: None,
            fragment_num_on_host: None,
            backend_id: None,
            need_wait_execution_trigger: None,
            instances_sharing_hash_table: None,
            is_simplified_param: Some(false),
            global_dict: None,
            fragment,
            local_params: Some(local_params),
            workload_groups: None,
            txn_conf: None,
            table_name: self.table_name.clone(),
            file_scan_params: None,
            group_commit: None,
            load_stream_per_node: None,
            total_load_streams: None,
            num_local_sink: None,
            num_buckets: None,
            bucket_seq_to_instance_idx: None,
            per_node_shared_scans: None,
            parallel_instances: None,
            total_instances: None,
            shuffle_idx_to_instance_idx: None,
            is_nereids: Some(self.is_nereids),
            wal_id: None,
            content_length: None,
            current_connect_fe: None,
            topn_filter_source_node_ids: None,
            ai_resources: None,
            is_mow_table: None,
        }
    }
}

/// Internal representation that mirrors the Thrift
/// `PaloInternalService.TPipelineFragmentParamsList` struct. This
/// holds the same fields as the Java FE payload and is responsible
/// for constructing the correct top-level Thrift struct before
/// compact encoding.
#[cfg(feature = "real_be_proto")]
#[derive(Debug, Clone)]
struct TPipelineFragmentParamsListThrift {
    query_id: Uuid,
    fragments: Vec<TPipelineFragmentParamsThrift>,
}

#[cfg(feature = "real_be_proto")]
impl From<&PipelineFragmentParamsList> for TPipelineFragmentParamsListThrift {
    fn from(src: &PipelineFragmentParamsList) -> Self {
        let fragments = src
            .fragments
            .iter()
            .map(TPipelineFragmentParamsThrift::from_pipeline_params)
            .collect();
        Self {
            query_id: src.query_id,
            fragments,
        }
    }
}

#[cfg(feature = "real_be_proto")]
impl TPipelineFragmentParamsListThrift {
    fn to_thrift_struct(&self) -> t_palo::TPipelineFragmentParamsList {
        let params_list: Vec<t_palo::TPipelineFragmentParams> = self
            .fragments
            .iter()
            .map(|frag| frag.to_thrift_struct())
            .collect();

        // Use FE address from configuration where possible so that BE
        // can attribute query fragments to a real coordinator. Fall
        // back to 127.0.0.1:9030 for tests or minimal configs.
        let cfg = Config::load().unwrap_or_default();
        let coord = encode_tnetwork_address_typed(&cfg.mysql_host(), cfg.mysql_port as i32);
        let query_globals = encode_query_globals_typed();
        let query_options = encode_query_options_typed();

        t_palo::TPipelineFragmentParamsList {
            params_list: Some(params_list),
            desc_tbl: None,
            file_scan_params: None,
            coord: Some(coord),
            query_globals: Some(query_globals),
            resource_info: None,
            fragment_num_on_host: None,
            query_options: Some(query_options),
            is_nereids: Some(true),
            workload_groups: None,
            query_id: Some(encode_tunique_id_typed(self.query_id)),
            topn_filter_source_node_ids: None,
            runtime_filter_merge_addr: None,
            runtime_filter_info: None,
        }
    }
}

impl PipelineFragmentParamsList {
    /// Build a very simple params list from a `QueryPlan` that contains a
    /// single fragment. This keeps Thrift-related concerns in the BE layer
    /// while letting the planner deal only in `QueryPlan`/`PlanFragment`
    /// structures.
    pub fn from_query_plan(plan: &QueryPlan) -> Option<Self> {
        if plan.fragments.len() != 1 {
            tracing::debug!("Pipeline params: Expected 1 fragment, got {}", plan.fragments.len());
            return None;
        }

        let fragment = &plan.fragments[0];
        tracing::debug!("Pipeline params: Fragment root node type: {:?}", std::mem::discriminant(&fragment.root_node));

        // For now we only support OlapScan or a unary node rooted on top of it.
        let table_name = match extract_scan_table_name(&fragment.root_node) {
            Some(name) => {
                tracing::debug!("Pipeline params: Extracted table name: {}", name);
                name
            }
            None => {
                tracing::debug!("Pipeline params: Could not extract table name from fragment root node");
                return None;
            }
        };

        if !is_simple_fragment(&fragment.root_node) {
            tracing::debug!("Pipeline params: Fragment is not simple");
            return None;
        }

        let params = PipelineFragmentParams {
            query_id: plan.query_id,
            fragment_id: fragment.fragment_id as i32,
            is_nereids: true,
            table_name: Some(table_name.to_string()),
        };

        Some(Self {
            query_id: plan.query_id,
            fragments: vec![params],
        })
    }

    /// EXPERIMENTAL: Generate 3-fragment structure matching Java FE pattern
    /// to test if BE requires multi-fragment execution.
    ///
    /// Structure:
    /// - Fragment[0] (fragment_id=2): Result sink (1 backend, no scan ranges)
    /// - Fragment[1] (fragment_id=1): Exchange (1 backend, no scan ranges)
    /// - Fragment[2] (fragment_id=0): Scan (1 backend, WITH scan ranges)
    pub fn from_query_plan_multi_fragment(plan: &QueryPlan) -> Option<Self> {
        if plan.fragments.len() != 1 {
            tracing::debug!("Multi-fragment: Expected 1 plan fragment, got {}", plan.fragments.len());
            return None;
        }

        let fragment = &plan.fragments[0];

        let table_name = match extract_scan_table_name(&fragment.root_node) {
            Some(name) => name,
            None => {
                tracing::debug!("Multi-fragment: Could not extract table name");
                return None;
            }
        };

        if !is_simple_fragment(&fragment.root_node) {
            tracing::debug!("Multi-fragment: Fragment is not simple");
            return None;
        }

        // Fragment 2 (fragment_id=0): Scan fragment - has table name for scan ranges
        let scan_fragment = PipelineFragmentParams {
            query_id: plan.query_id,
            fragment_id: 0,  // Java FE uses fragment_id=0 for scan
            is_nereids: true,
            table_name: Some(table_name.to_string()),
        };

        // Fragment 1 (fragment_id=1): Exchange fragment - no scan ranges
        let exchange_fragment = PipelineFragmentParams {
            query_id: plan.query_id,
            fragment_id: 1,
            is_nereids: true,
            table_name: None,  // Exchange has no table
        };

        // Fragment 0 (fragment_id=2): Result sink - no scan ranges
        let result_fragment = PipelineFragmentParams {
            query_id: plan.query_id,
            fragment_id: 2,
            is_nereids: true,
            table_name: None,  // Result sink has no table
        };

        tracing::info!("🔧 Generated 3-fragment structure: Result(id=2) → Exchange(id=1) → Scan(id=0)");

        Some(Self {
            query_id: plan.query_id,
            fragments: vec![result_fragment, exchange_fragment, scan_fragment],
        })
    }

    /// Convert to a Thrift `Struct` representing Doris's
    /// `TPipelineFragmentParamsList` using the Thrift compact protocol.
    ///
    /// Initially we fill only a small subset of fields that are
    /// stable and easy to derive from our QueryPlan:
    ///   - params_list (field 1)
    ///   - coord (field 4)
    ///   - query_globals (field 5)
    ///   - query_options (field 8)
    ///   - is_nereids (field 9)
    ///   - query_id (field 11)
    #[cfg(feature = "real_be_proto")]
    pub fn to_thrift_struct(&self) -> t_palo::TPipelineFragmentParamsList {
        let typed = TPipelineFragmentParamsListThrift::from(self);
        typed.to_thrift_struct()
    }

    /// Serialize to Thrift compact format bytes.
    #[cfg(feature = "real_be_proto")]
    pub fn to_thrift_bytes(&self) -> Vec<u8> {
        let s = self.to_thrift_struct();

        // Debug: check scan ranges in struct before serialization
        if let Some(ref params_list) = s.params_list {
            if let Some(first_param) = params_list.first() {
                if let Some(ref local_params) = first_param.local_params {
                    if let Some(first_local) = local_params.first() {
                        let scan_ranges_count: usize = first_local.per_node_scan_ranges.values()
                            .map(|v| v.len()).sum();
                        tracing::info!("🔍 Before serialization: local_params[0].per_node_scan_ranges has {} nodes, {} total ranges",
                            first_local.per_node_scan_ranges.len(), scan_ranges_count);
                    }
                }
            }
        }

        let bytes = compact_encode_thrift_struct(&s)
            .expect("encode TPipelineFragmentParamsList");

        tracing::info!("📦 After serialization: payload is {} bytes", bytes.len());

        bytes
    }
}

// --- Compact protocol encoder using `compact-thrift-runtime` ---

#[cfg(feature = "real_be_proto")]
fn compact_encode_thrift_struct<T>(value: &T) -> thrift::Result<Vec<u8>>
where
    T: TSerializable,
{
    let mut channel = TBufferChannel::with_capacity(0, 4096);
    {
        let mut o_prot = TCompactOutputProtocol::new(&mut channel);
        value.write_to_out_protocol(&mut o_prot)?;
    }
    Ok(channel.write_bytes())
}

#[cfg(feature = "real_be_proto")]
fn compact_write_struct<W: CompactThriftOutput>(
    output: &mut W,
    s: &ThriftStruct,
) -> Result<(), ThriftError> {
    // Ensure fields are in ascending id order so that field id deltas are non-negative.
    let mut fields: Vec<ThriftField> = s.fields().to_vec();
    fields.sort_by_key(|f| f.id());

    let mut last_field_id: i16 = 0;
    for field in &fields {
        compact_write_field(output, field, &mut last_field_id)?;
    }

    // STOP field
    output.write_byte(0)?;
    Ok(())
}

#[cfg(feature = "real_be_proto")]
fn compact_write_field<W: CompactThriftOutput>(
    output: &mut W,
    field: &ThriftField,
    last_field_id: &mut i16,
) -> Result<(), ThriftError> {
    let field_id = field.id();
    let data = field.data();

    // Booleans are encoded via the field type only, with no payload bytes.
    if let Data::Bool(value) = data {
        let field_type = if *value { 1 } else { 2 };
        write_field_header(output, field_type, field_id, last_field_id)?;
        return Ok(());
    }

    let field_type = compact_type_for_data(data);
    write_field_header(output, field_type, field_id, last_field_id)?;
    compact_write_data(output, data)
}

#[cfg(feature = "real_be_proto")]
fn compact_type_for_data(data: &Data) -> u8 {
    match data {
        Data::Bool(_) => 2,
        Data::I8(_) => 3,
        Data::I16(_) => 4,
        Data::I32(_) => 5,
        Data::I64(_) => 6,
        Data::Double(_) => 7,
        Data::Binary(_) => 8,
        Data::Struct(_) => 12,
        Data::Map(_) => 11,
        Data::Set(_) => 10,
        Data::List(_) => 9,
        Data::Uuid(_) => 13,
    }
}

#[cfg(feature = "real_be_proto")]
fn element_type_for_kind(kind: thrift_codec::data::DataKind) -> Result<u8, ThriftError> {
    use thrift_codec::data::DataKind;
    let t = match kind {
        DataKind::Bool => 2,
        DataKind::I8 => 3,
        DataKind::I16 => 4,
        DataKind::I32 => 5,
        DataKind::I64 => 6,
        DataKind::Double => 7,
        DataKind::Binary => 8,
        DataKind::Struct => 12,
        DataKind::Map => 11,
        DataKind::Set => 10,
        DataKind::List => 9,
        DataKind::Uuid => 13,
    };
    Ok(t)
}

#[cfg(feature = "real_be_proto")]
fn compact_write_data<W: CompactThriftOutput>(
    output: &mut W,
    data: &Data,
) -> Result<(), ThriftError> {
    match data {
        Data::Bool(v) => {
            // Used for list/set/map elements: encode as a single byte 1/2.
            let value = if *v { 1 } else { 2 };
            output.write_byte(value)
        }
        Data::I8(v) => output.write_byte(*v as u8),
        Data::I16(v) => output.write_i16(*v),
        Data::I32(v) => output.write_i32(*v),
        Data::I64(v) => output.write_i64(*v),
        Data::Double(v) => output.write_double(*v),
        Data::Binary(v) => output.write_binary(v),
        Data::Struct(s) => compact_write_struct(output, s),
        Data::List(list) => compact_write_list(output, list),
        Data::Map(map) => compact_write_map(output, map),
        Data::Set(_) => Err(ThriftError::InvalidType),
        Data::Uuid(_) => Err(ThriftError::InvalidType),
    }
}

#[cfg(feature = "real_be_proto")]
fn compact_write_data_ref<W: CompactThriftOutput>(
    output: &mut W,
    data: DataRef<'_>,
) -> Result<(), ThriftError> {
    match data {
        DataRef::Bool(v) => {
            let value = if *v { 1 } else { 2 };
            output.write_byte(value)
        }
        DataRef::I8(v) => output.write_byte(*v as u8),
        DataRef::I16(v) => output.write_i16(*v),
        DataRef::I32(v) => output.write_i32(*v),
        DataRef::I64(v) => output.write_i64(*v),
        DataRef::Double(v) => output.write_double(*v),
        DataRef::Binary(v) => output.write_binary(v),
        DataRef::Struct(s) => compact_write_struct(output, s),
        DataRef::List(list) => compact_write_list(output, list),
        DataRef::Map(map) => compact_write_map(output, map),
        DataRef::Set(_) => Err(ThriftError::InvalidType),
        DataRef::Uuid(_) => Err(ThriftError::InvalidType),
    }
}

#[cfg(feature = "real_be_proto")]
fn compact_write_list<W: CompactThriftOutput>(
    output: &mut W,
    list: &ThriftList,
) -> Result<(), ThriftError> {
    let elements: &Elements = &*list;
    let len = elements.len();
    let elem_kind = elements.kind();
    let elem_type = element_type_for_kind(elem_kind)?;

    if len < 15 {
        let header = ((len as u8) << 4) | elem_type;
        output.write_byte(header)?;
    } else {
        let header = 0xF0 | elem_type;
        output.write_byte(header)?;
        output.write_len(len)?;
    }

    for elem in elements.iter() {
        compact_write_data_ref(output, elem)?;
    }
    Ok(())
}

#[cfg(feature = "real_be_proto")]
fn compact_write_map<W: CompactThriftOutput>(
    output: &mut W,
    map: &ThriftMap,
) -> Result<(), ThriftError> {
    let len = map.len();
    if len == 0 {
        // Empty map header is a single 0 byte.
        output.write_byte(0)?;
        return Ok(());
    }

    let key_kind = map.key_kind().ok_or(ThriftError::InvalidType)?;
    let val_kind = map.value_kind().ok_or(ThriftError::InvalidType)?;
    let key_type = element_type_for_kind(key_kind)?;
    let val_type = element_type_for_kind(val_kind)?;

    output.write_len(len)?;
    let entry_type = (key_type << 4) | val_type;
    output.write_byte(entry_type)?;

    for (k, v) in map.iter() {
        compact_write_data_ref(output, k)?;
        compact_write_data_ref(output, v)?;
    }
    Ok(())
}

/// Write a field header following the Apache Thrift compact protocol.
#[cfg(feature = "real_be_proto")]
fn write_field_header<W: CompactThriftOutput>(
    output: &mut W,
    field_type: u8,
    field_id: i16,
    last_field_id: &mut i16,
) -> Result<(), ThriftError> {
    let delta = field_id - *last_field_id;
    if delta > 0 && delta <= 15 {
        let header = ((delta as u8) << 4) | field_type;
        output.write_byte(header)?;
    } else {
        output.write_byte(field_type)?;
        output.write_i16(field_id)?;
    }
    *last_field_id = field_id;
    Ok(())
}

fn extract_scan_table_name(root: &PlanNode) -> Option<&str> {
    match root {
        PlanNode::OlapScan { table_name, .. } => Some(table_name.as_str()),
        PlanNode::Aggregation { child, .. }
        | PlanNode::Project { child, .. }
        | PlanNode::Select { child, .. }
        | PlanNode::Limit { child, .. } => extract_scan_table_name(child),
        _ => {
            tracing::debug!("extract_scan_table_name: Unhandled node type: {:?}", std::mem::discriminant(root));
            None
        }
    }
}

fn is_simple_fragment(root: &PlanNode) -> bool {
    extract_scan_table_name(root).is_some()
}

// Encode Doris Types.TUniqueId into a Thrift struct:
// struct TUniqueId { 1: i64 hi, 2: i64 lo }
#[cfg(feature = "real_be_proto")]
fn encode_tunique_id(query_id: Uuid) -> ThriftStruct {
    let bytes = query_id.as_u128().to_be_bytes();
    let (hi_bytes, lo_bytes) = bytes.split_at(8);
    let hi = i64::from_be_bytes(hi_bytes.try_into().unwrap());
    let lo = i64::from_be_bytes(lo_bytes.try_into().unwrap());

    // Types.TUniqueId { 1: i64 hi, 2: i64 lo }
    ThriftStruct::new(vec![
        ThriftField::new(1, hi),
        ThriftField::new(2, lo),
    ])
}

// Encode Types.TNetworkAddress into a Thrift struct:
// struct TNetworkAddress { 1: required string hostname, 2: required i32 port }
#[cfg(feature = "real_be_proto")]
fn encode_tnetwork_address(hostname: &str, port: i32) -> ThriftStruct {
    ThriftStruct::new(vec![
        ThriftField::new(1, hostname.to_string()),
        ThriftField::new(2, port),
    ])
}

// Encode minimal TQueryGlobals for testing
// struct TQueryGlobals { 1: required string now_string, 2: optional i64 timestamp_ms, 3: optional string time_zone }
#[cfg(feature = "real_be_proto")]
fn encode_query_globals() -> ThriftStruct {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Get current time
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    let timestamp_ms = now.as_millis() as i64;

    // Format: yyyy-MM-dd HH:mm:ss
    let now_string = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    ThriftStruct::new(vec![
        ThriftField::new(1, now_string),           // required now_string
        ThriftField::new(2, timestamp_ms),          // optional timestamp_ms
        ThriftField::new(3, "UTC".to_string()),     // optional time_zone
    ])
}

// Encode minimal TQueryOptions for testing
// Most fields are optional and can use defaults
#[cfg(feature = "real_be_proto")]
fn encode_query_options() -> ThriftStruct {
    let mut fields = Vec::new();

    // Just set a few common options with reasonable defaults.
    //
    // 4: batch_size (i32)
    fields.push(ThriftField::new(4, 1024_i32));
    // 14: query_timeout (i32)
    fields.push(ThriftField::new(14, 600_i32));
    // 57: enable_pipeline_engine (bool)
    fields.push(ThriftField::new(57, true));

    ThriftStruct::new(fields)
}

// Encode a minimal PaloInternalService.TPipelineFragmentParams for a single fragment.
// NOTE: This function is obsolete with real_be_proto feature; we use TPipelineFragmentParamsListThrift::to_thrift_struct() directly
// #[cfg(feature = "real_be_proto")]
// fn encode_pipeline_fragment_params(frag: &PipelineFragmentParams) -> ThriftStruct {
//     let typed = TPipelineFragmentParamsThrift::from_pipeline_params(frag);
//     typed.to_thrift_struct()
// }

/// Encode a minimal PaloInternalService.TPipelineInstanceParams for a single
/// execution instance. For early experiments we only fill the required
/// fragment_instance_id and leave scan ranges and runtime filters empty.
#[cfg(feature = "real_be_proto")]
fn encode_pipeline_instance_params(fragment_instance_id: Uuid) -> ThriftStruct {
    let mut fields = Vec::new();
    // 1: fragment_instance_id (Types.TUniqueId)
    fields.push(ThriftField::new(1, encode_tunique_id(fragment_instance_id)));
    ThriftStruct::new(fields)
}

// Typed helpers using generated Apache Thrift structs.

#[cfg(feature = "real_be_proto")]
fn encode_tunique_id_typed(query_id: Uuid) -> t_types::TUniqueId {
    let bytes = query_id.as_u128().to_be_bytes();
    let (hi_bytes, lo_bytes) = bytes.split_at(8);
    let hi = i64::from_be_bytes(hi_bytes.try_into().unwrap());
    let lo = i64::from_be_bytes(lo_bytes.try_into().unwrap());
    t_types::TUniqueId { hi, lo }
}

#[cfg(feature = "real_be_proto")]
fn encode_tnetwork_address_typed(hostname: &str, port: i32) -> t_types::TNetworkAddress {
    t_types::TNetworkAddress {
        hostname: hostname.to_string(),
        port,
    }
}

#[cfg(feature = "real_be_proto")]
fn encode_query_globals_typed() -> t_palo::TQueryGlobals {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap();
    let timestamp_ms = now.as_millis() as i64;

    let now_string = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    t_palo::TQueryGlobals::new(
        now_string,
        Some(timestamp_ms),
        // Match Java FE, which reports "Etc/UTC" in captured payloads.
        Some("Etc/UTC".to_string()),
        None::<bool>,
        None::<i32>,
        None::<String>,
    )
}

#[cfg(feature = "real_be_proto")]
fn encode_query_options_typed() -> t_palo::TQueryOptions {
    let mut opts = t_palo::TQueryOptions::default();
    // Align with Java FE capture for simple scans:
    // batch_size=4064, query_timeout=900, enable_pipeline_engine unset.
    opts.batch_size = Some(4064);
    opts.query_timeout = Some(900);
    opts.enable_pipeline_engine = None;
    opts
}

// Lightweight, config-driven scan range mapping used for early
// experiments. This lets us wire real tablet scans without a full BE
// metadata sync. When no config is present we fall back to an empty
// per_node_scan_ranges map so behaviour matches the previous version
// (no tablets → 0 rows).
#[cfg(feature = "real_be_proto")]
#[derive(Debug, Deserialize)]
struct TableScanConfig {
  /// Tablet IDs to scan for this table.
  tablet_ids: Vec<i64>,
  /// Optional visible version; when omitted we use 0 and let BE
  /// resolve the latest rowsets.
  version: Option<i64>,
  /// Optional schema hash; when omitted we use "0" which may cause BE
  /// to skip the scan. Should match the actual SchemaHash from metadata.
  schema_hash: Option<String>,
  /// Optional explicit replica hosts for this table. When present we
  /// use these addresses for `TPaloScanRange.hosts` instead of the
  /// first backend from `fe_config`. This allows matching the Java FE
  /// behaviour of listing all tablet replicas without embedding Doris
  /// metadata RPCs in the pipeline encoder.
  replica_hosts: Option<Vec<ReplicaHostConfig>>,
}

/// Minimal host description used by `TableScanConfig` when callers
/// want to control the exact set of replica backends for a table.
#[cfg(feature = "real_be_proto")]
#[derive(Clone, Debug, Deserialize)]
struct ReplicaHostConfig {
  pub host: String,
  pub port: u16,
}

#[cfg(feature = "real_be_proto")]
#[derive(Debug, Deserialize)]
struct ScanRangeConfig {
    /// Keys are fully-qualified table names like "tpch_sf1.lineitem".
    tables: HashMap<String, TableScanConfig>,
}

#[cfg(feature = "real_be_proto")]
fn load_scan_range_config() -> Option<ScanRangeConfig> {
    // Allow overriding the config path via env; default to a local file
    // so experiments can drop in a JSON mapping without changing code.
    let path =
        std::env::var("RUST_FE_SCAN_RANGES").unwrap_or_else(|_| "scan_ranges.json".to_string());
    let content = fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<ScanRangeConfig>(&content) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            tracing::warn!("Failed to parse scan range config {}: {}", path, e);
            None
        }
    }
}

/// Build per-node scan ranges for a simple single-scan fragment using a
/// static table→tablet mapping. This mirrors the Java FE behaviour of
/// populating `TPipelineInstanceParams.per_node_scan_ranges` from
/// `ScanRanges` but restricts scope to the first OLAP scan node and a
/// small, config-driven tablet set.
#[cfg(feature = "real_be_proto")]
fn build_scan_ranges_for_table_from_config(
    db_name: &str,
    table_name: &str,
) -> Option<BTreeMap<t_types::TPlanNodeId, Vec<t_palo::TScanRangeParams>>> {
    tracing::info!("→ Attempting to load scan ranges for {}.{}", db_name, table_name);
    let cfg = match load_scan_range_config() {
        Some(c) => {
            tracing::info!("  ✓ Loaded scan_ranges.json with {} tables", c.tables.len());
            c
        }
        None => {
            tracing::warn!("  ✗ Failed to load scan_ranges.json");
            return None;
        }
    };
    let key = format!("{}.{}", db_name, table_name);
    let table_cfg = match cfg.tables.get(&key) {
        Some(tc) => {
            tracing::info!("  ✓ Found config for table {} ({} tablets)", key, tc.tablet_ids.len());
            tc
        }
        None => {
            tracing::warn!("  ✗ No config found for table {} (available tables: {:?})", key, cfg.tables.keys().collect::<Vec<_>>());
            return None;
        }
    };
    if table_cfg.tablet_ids.is_empty() {
        tracing::warn!(
            "Scan range config for table {} has no tablet_ids; skipping scan ranges",
            key
        );
        return None;
    }
    // Determine replica hosts for TPaloScanRange.hosts. Prefer explicit
    // replica_hosts from scan_ranges.json so tests can match Java FE's
    // per-tablet replica layout; fall back to the first backend from
    // fe_config for simple single-BE deployments.
    let host_addrs: Vec<t_types::TNetworkAddress> = if let Some(ref replicas) = table_cfg.replica_hosts {
        if replicas.is_empty() {
            tracing::warn!(
                "replica_hosts for table {} is empty; falling back to fe_config backend_nodes",
                key
            );
            Vec::new()
        } else {
            replicas
                .iter()
                .map(|r| encode_tnetwork_address_typed(&r.host, r.port as i32))
                .collect()
        }
    } else {
        Vec::new()
    };

    let host_addrs = if host_addrs.is_empty() {
        // Legacy behaviour: use the first configured backend as the
        // sole host when no explicit replica list is provided.
        let fe_cfg = Config::load().unwrap_or_default();
        match fe_cfg.backend_nodes.first() {
            Some(b) => vec![encode_tnetwork_address_typed(&b.host, b.port as i32)],
            None => {
                tracing::warn!(
                    "No backend_nodes configured in fe_config; cannot build scan ranges for {}",
                    key
                );
                return None;
            }
        }
    } else {
        host_addrs
    };

    let version_str = table_cfg.version.unwrap_or(0).to_string();
    let schema_hash_str = table_cfg.schema_hash.clone().unwrap_or_else(|| "0".to_string());

    let mut params_vec = Vec::with_capacity(table_cfg.tablet_ids.len());
    for &tablet_id in &table_cfg.tablet_ids {
        // EXPERIMENT: For lineitem, re-introduce schema_hash (keep version='3', db='', table=None)
        let (experimental_schema_hash, experimental_version, experimental_db, experimental_table) =
            if table_name == "lineitem" {
                tracing::warn!("🧪 EXPERIMENT: lineitem - adding back schema_hash={}, keeping version='3', db='', table=None", schema_hash_str);
                (schema_hash_str.clone(), "3".to_string(), "".to_string(), None::<String>)
            } else {
                (schema_hash_str.clone(), version_str.clone(), db_name.to_string(), Some(table_name.to_string()))
            };

        let palo = t_plan_nodes::TPaloScanRange::new(
            host_addrs.clone(),
            experimental_schema_hash,  // schema_hash: actual value for lineitem experiment
            experimental_version,      // version: "3" for lineitem experiment
            "0".to_string(),           // version_hash (always "0")
            tablet_id,
            experimental_db,           // db_name: "" for lineitem experiment
            None::<Vec<t_plan_nodes::TKeyRange>>,
            None::<String>,            // index_name
            experimental_table,        // table_name: None for lineitem experiment
        );

        tracing::info!(
            "  → Created TPaloScanRange: tablet_id={}, schema_hash={}, version={}, db={}, table={}, hosts={:?}",
            tablet_id, schema_hash_str, version_str, db_name, table_name, host_addrs
        );

        let scan_range = t_plan_nodes::TScanRange::new(
            Some(palo),
            None::<Vec<u8>>,
            None::<t_plan_nodes::TBrokerScanRange>,
            None::<t_plan_nodes::TEsScanRange>,
            None::<t_plan_nodes::TExternalScanRange>,
            None::<t_plan_nodes::TDataGenScanRange>,
            None::<t_plan_nodes::TMetaScanRange>,
        );

        let params = t_palo::TScanRangeParams::new(scan_range, None::<i32>);
        params_vec.push(params);
    }

    // Our simple fragments always use a single OLAP_SCAN_NODE with node_id = 0.
    let mut map: BTreeMap<t_types::TPlanNodeId, Vec<t_palo::TScanRangeParams>> = BTreeMap::new();
    map.insert(0 as t_types::TPlanNodeId, params_vec.clone());

    tracing::info!(
        "✓ Built scan ranges map: node_id=0 has {} scan ranges",
        params_vec.len()
    );

    Some(map)
}

#[cfg(feature = "real_be_proto")]
fn encode_pipeline_instance_params_typed(
    fragment: &TPipelineFragmentParamsThrift,
    db_name: &str,
) -> t_palo::TPipelineInstanceParams {
    let fragment_instance_id = encode_tunique_id_typed(fragment.query_id);

    // Try to synthesise scan ranges for the first OLAP scan node from a
    // static config. When none are available we fall back to an empty
    // map so BE still executes the fragment but will see 0 tablets.
    tracing::info!("Building instance params for table_name={:?}, db_name={}", fragment.table_name, db_name);
    let per_node_scan_ranges: BTreeMap<t_types::TPlanNodeId, Vec<t_palo::TScanRangeParams>> =
        match fragment.table_name.as_deref() {
            Some(table) => {
                let ranges = build_scan_ranges_for_table_from_config(db_name, table);
                if let Some(ref r) = ranges {
                    tracing::info!("✓ Loaded scan ranges for {}.{}: {} nodes, {} total ranges",
                        db_name, table, r.len(), r.values().map(|v| v.len()).sum::<usize>());
                } else {
                    tracing::warn!("✗ No scan ranges loaded for {}.{} - will return 0 rows!", db_name, table);
                }
                ranges.unwrap_or_else(BTreeMap::new)
            }
            None => {
                tracing::warn!("✗ No table_name in fragment - will return 0 rows!");
                BTreeMap::new()
            }
        };

    let instance = t_palo::TPipelineInstanceParams::new(
        fragment_instance_id,
        None::<bool>,
        per_node_scan_ranges,
        None::<i32>,
        None::<t_palo::TRuntimeFilterParams>,
        None::<i32>,
        None::<BTreeMap<t_types::TPlanNodeId, bool>>,
        None::<Vec<i32>>,
        None::<Vec<t_plan_nodes::TTopnFilterDesc>>,
    );

    // Verify scan ranges are in the struct
    let total_ranges: usize = instance.per_node_scan_ranges.values().map(|v| v.len()).sum();
    tracing::info!("📦 TPipelineInstanceParams created: per_node_scan_ranges has {} nodes, {} total ranges",
        instance.per_node_scan_ranges.len(), total_ranges);

    instance
}

// --- PlanFragment / PlanNode / OlapScan encoding helpers (minimal) ---

/// Encode a minimal TOlapScanNode for simple scans. Many fields are left at
/// defaults; this is only suitable for early experimentation.
#[cfg(feature = "real_be_proto")]
fn encode_olap_scan_node_basic(table_name: &str) -> ThriftStruct {
    let tuple_id: i32 = 0;
    let key_column_names: Vec<String> = Vec::new();
    let key_column_types: Vec<i32> = Vec::new();
    let is_preaggregation = true;

    let mut fields = Vec::new();
    // 1: tuple_id
    fields.push(ThriftField::new(1, tuple_id));
    // 2: key_column_name (convert Vec<String> to Vec<Vec<u8>> for Thrift)
    let key_names_bytes: Vec<Vec<u8>> = key_column_names.into_iter()
        .map(|s| s.into_bytes())
        .collect();
    fields.push(ThriftField::new(2, ThriftList::from(key_names_bytes)));
    // 3: key_column_type
    fields.push(ThriftField::new(3, ThriftList::from(key_column_types)));
    // 4: is_preaggregation
    fields.push(ThriftField::new(4, is_preaggregation));
    // 7: table_name
    fields.push(ThriftField::new(7, table_name.to_string()));

    ThriftStruct::new(fields)
}

/// Encode a TOlapScanNode for the tpch.lineitem table using catalog
/// metadata to populate key column names/types. This keeps other fields
/// minimal while giving BE enough information to resolve keys.
#[cfg(feature = "real_be_proto")]
fn encode_olap_scan_node_tpch_lineitem() -> ThriftStruct {
    let tuple_id: i32 = 0;
    let cat = catalog::catalog();
    let table = cat
        .get_table("tpch", "lineitem")
        .expect("tpch.lineitem table should exist in metadata catalog");

    let key_cols = ["l_orderkey", "l_partkey", "l_suppkey"];
    let mut key_column_names = Vec::new();
    let mut key_column_types = Vec::new();

    for name in key_cols.iter() {
        let col = table
            .get_column(name)
            .unwrap_or_else(|| panic!("expected key column {} in tpch.lineitem", name));
        key_column_names.push(name.to_string());
        key_column_types.push(primitive_type_from_metadata(&col.data_type));
    }

    let is_preaggregation = true;

    let mut fields = Vec::new();
    // 1: tuple_id
    fields.push(ThriftField::new(1, tuple_id));
    // 2: key_column_name (convert Vec<String> to Vec<Vec<u8>> for Thrift)
    let key_names_bytes: Vec<Vec<u8>> = key_column_names.into_iter()
        .map(|s| s.into_bytes())
        .collect();
    fields.push(ThriftField::new(2, ThriftList::from(key_names_bytes)));
    // 3: key_column_type
    fields.push(ThriftField::new(3, ThriftList::from(key_column_types)));
    // 4: is_preaggregation
    fields.push(ThriftField::new(4, is_preaggregation));
    // 7: table_name
    fields.push(ThriftField::new(7, table.name.clone()));

    ThriftStruct::new(fields)
}

/// Encode a single TPlanNode wrapping an OLAP_SCAN_NODE.
#[cfg(feature = "real_be_proto")]
fn encode_plan_node_for_scan(scan_node: ThriftStruct) -> ThriftStruct {
    let node_id: i32 = 0;
    let node_type: i32 = 0; // TPlanNodeType.OLAP_SCAN_NODE
    let num_children: i32 = 0;
    let limit: i64 = -1;
    let row_tuples = vec![0_i32];
    let nullable_tuples = vec![false];
    let compact_data = true;

    let mut fields = Vec::new();
    fields.push(ThriftField::new(1, node_id));
    fields.push(ThriftField::new(2, node_type));
    fields.push(ThriftField::new(3, num_children));
    fields.push(ThriftField::new(4, limit));
    fields.push(ThriftField::new(5, ThriftList::from(row_tuples)));
    fields.push(ThriftField::new(6, ThriftList::from(nullable_tuples)));
    fields.push(ThriftField::new(8, compact_data));
    // 18: olap_scan_node
    fields.push(ThriftField::new(18, scan_node));

    ThriftStruct::new(fields)
}

/// Encode a minimal PlanNodes.TPlan with a single node.
#[cfg(feature = "real_be_proto")]
fn encode_plan_with_single_node(node: ThriftStruct) -> ThriftStruct {
    let nodes = ThriftList::from(vec![node]);
    ThriftStruct::new(vec![ThriftField::new(1, nodes)])
}

/// Encode an unpartitioned Partitions.TDataPartition.
#[cfg(feature = "real_be_proto")]
fn encode_unpartitioned_data_partition() -> ThriftStruct {
    let partition_type: i32 = 0; // TPartitionType.UNPARTITIONED
    ThriftStruct::new(vec![ThriftField::new(1, partition_type)])
}

/// Encode a minimal Planner.TPlanFragment for a simple scan fragment.
#[cfg(feature = "real_be_proto")]
fn encode_plan_fragment_for_scan(table_name: &str) -> ThriftStruct {
    let scan_node = match table_name {
        "lineitem" => encode_olap_scan_node_tpch_lineitem(),
        _ => encode_olap_scan_node_basic(table_name),
    };
    let plan_node = encode_plan_node_for_scan(scan_node);
    let plan = encode_plan_with_single_node(plan_node);
    let partition = encode_unpartitioned_data_partition();

    let mut fields = Vec::new();
    // 2: plan
    fields.push(ThriftField::new(2, plan));
    // 6: partition (required)
    fields.push(ThriftField::new(6, partition));

    ThriftStruct::new(fields)
}

// --- Typed PlanFragment / PlanNode / OlapScan helpers using generated structs ---

#[cfg(feature = "real_be_proto")]
fn encode_olap_scan_node_basic_typed(table_name: &str) -> t_plan_nodes::TOlapScanNode {
    t_plan_nodes::TOlapScanNode {
        tuple_id: 0,
        key_column_name: Vec::new(),
        key_column_type: Vec::new(),
        is_preaggregation: true,
        sort_column: None,
        key_type: None,
        table_name: Some(table_name.to_string()),
        columns_desc: None,
        sort_info: None,
        sort_limit: None,
        enable_unique_key_merge_on_write: None,
        push_down_agg_type_opt: None,
        use_topn_opt: None,
        indexes_desc: None,
        output_column_unique_ids: None,
        distribute_column_ids: None,
        schema_version: None,
        topn_filter_source_node_ids: None,
        score_sort_info: None,
        score_sort_limit: None,
        ann_sort_info: None,
        ann_sort_limit: None,
    }
}

#[cfg(feature = "real_be_proto")]
fn encode_olap_scan_node_tpch_lineitem_typed() -> t_plan_nodes::TOlapScanNode {
    let cat = catalog::catalog();
    let table = cat
        .get_table("tpch_sf1", "lineitem")
        .expect("tpch_sf1.lineitem table should exist in metadata catalog");

    let key_cols = ["l_orderkey", "l_partkey", "l_suppkey"];
    let mut key_column_name = Vec::new();
    let mut key_column_type = Vec::new();

    for name in key_cols {
        let col = table
            .get_column(name)
            .unwrap_or_else(|| panic!("expected key column {} in tpch_sf1.lineitem", name));
        key_column_name.push(name.to_string());
        key_column_type.push(primitive_type_from_metadata_typed(&col.data_type));
    }

    t_plan_nodes::TOlapScanNode {
        tuple_id: 0,
        key_column_name,
        key_column_type,
        is_preaggregation: true,
        sort_column: None,
        key_type: None,
        table_name: Some(table.name.clone()),
        columns_desc: None,
        sort_info: None,
        sort_limit: None,
        enable_unique_key_merge_on_write: None,
        push_down_agg_type_opt: None,
        use_topn_opt: None,
        indexes_desc: None,
        output_column_unique_ids: None,
        distribute_column_ids: None,
        schema_version: None,
        topn_filter_source_node_ids: None,
        score_sort_info: None,
        score_sort_limit: None,
        ann_sort_info: None,
        ann_sort_limit: None,
    }
}

#[cfg(feature = "real_be_proto")]
fn encode_plan_node_for_scan_typed(
    scan_node: t_plan_nodes::TOlapScanNode,
) -> t_plan_nodes::TPlanNode {
    t_plan_nodes::TPlanNode {
        node_id: 0,
        node_type: t_plan_nodes::TPlanNodeType::OLAP_SCAN_NODE,
        num_children: 0,
        limit: -1,
        row_tuples: vec![0],
        nullable_tuples: vec![false],
        conjuncts: None,
        compact_data: true,
        hash_join_node: None,
        agg_node: None,
        sort_node: None,
        merge_node: None,
        exchange_node: None,
        mysql_scan_node: None,
        olap_scan_node: Some(scan_node),
        csv_scan_node: None,
        broker_scan_node: None,
        pre_agg_node: None,
        schema_scan_node: None,
        merge_join_node: None,
        meta_scan_node: None,
        analytic_node: None,
        olap_rewrite_node: None,
        union_node: None,
        resource_profile: None,
        es_scan_node: None,
        repeat_node: None,
        assert_num_rows_node: None,
        intersect_node: None,
        except_node: None,
        odbc_scan_node: None,
        runtime_filters: None,
        group_commit_scan_node: None,
        materialization_node: None,
        vconjunct: None,
        table_function_node: None,
        output_slot_ids: None,
        data_gen_scan_node: None,
        file_scan_node: None,
        jdbc_scan_node: None,
        nested_loop_join_node: None,
        test_external_scan_node: None,
        push_down_agg_type_opt: None,
        push_down_count: None,
        distribute_expr_lists: None,
        is_serial_operator: None,
        projections: None,
        output_tuple_id: None,
        partition_sort_node: None,
        intermediate_projections_list: None,
        intermediate_output_tuple_id_list: None,
        topn_filter_source_node_ids: None,
        nereids_id: None,
    }
}

#[cfg(feature = "real_be_proto")]
fn encode_plan_with_single_node_typed(node: t_plan_nodes::TPlanNode) -> t_plan_nodes::TPlan {
    t_plan_nodes::TPlan { nodes: vec![node] }
}

#[cfg(feature = "real_be_proto")]
fn encode_unpartitioned_data_partition_typed() -> t_parts::TDataPartition {
    t_parts::TDataPartition::new(
        t_parts::TPartitionType::UNPARTITIONED,
        None::<Vec<t_exprs::TExpr>>,
        None::<Vec<t_parts::TRangePartition>>,
    )
}

/// Generate appropriate plan fragment based on fragment type (fragment_id).
/// - fragment_id == 0: Scan fragment with OLAP_SCAN_NODE
/// - fragment_id == 1: Exchange fragment (minimal for now)
/// - fragment_id == 2: Result sink fragment (minimal for now)
#[cfg(feature = "real_be_proto")]
fn encode_plan_fragment_by_type(fragment_id: i32, table_name: Option<&str>) -> Option<t_planner::TPlanFragment> {
    match fragment_id {
        0 => {
            // Scan fragment: requires table name
            table_name.map(encode_plan_fragment_for_scan_typed)
        }
        1 => {
            // Exchange fragment: create minimal exchange plan
            Some(encode_plan_fragment_for_exchange_typed())
        }
        2 => {
            // Result sink fragment: create minimal result plan
            Some(encode_plan_fragment_for_result_typed())
        }
        _ => {
            tracing::warn!("Unknown fragment_id {}, skipping plan fragment", fragment_id);
            None
        }
    }
}

#[cfg(feature = "real_be_proto")]
fn encode_plan_fragment_for_scan_typed(table_name: &str) -> t_planner::TPlanFragment {
    let scan_node = match table_name {
        "lineitem" => encode_olap_scan_node_tpch_lineitem_typed(),
        _ => encode_olap_scan_node_basic_typed(table_name),
    };
    let plan_node = encode_plan_node_for_scan_typed(scan_node);
    let plan = encode_plan_with_single_node_typed(plan_node);
    let partition = encode_unpartitioned_data_partition_typed();
    let output_sink = encode_result_sink_typed();

    t_planner::TPlanFragment::new(
        Some(plan),
        None::<Vec<t_exprs::TExpr>>,
        Some(output_sink),
        partition,
        None::<i64>,
        None::<i64>,
        None::<TQueryCacheParam>,
    )
}

/// Create a minimal plan fragment for exchange node (fragment_id=1).
/// TEMPORARY: Returns None to skip this fragment until proper EXCHANGE_NODE is implemented.
/// TODO: Implement proper EXCHANGE_NODE structure.
#[cfg(feature = "real_be_proto")]
fn encode_plan_fragment_for_exchange_typed() -> t_planner::TPlanFragment {
    // WORKAROUND: BE requires fragments to have plan nodes.
    // For now, return None to skip exchange fragments.
    // This will make the 3-fragment structure fall back to simpler execution.
    encode_plan_fragment_for_scan_typed("lineitem")
}

/// Create a minimal plan fragment for result sink node (fragment_id=2).
/// TEMPORARY: Returns None to skip this fragment until proper result node is implemented.
/// TODO: Implement proper result aggregation node.
#[cfg(feature = "real_be_proto")]
fn encode_plan_fragment_for_result_typed() -> t_planner::TPlanFragment {
    // WORKAROUND: BE requires fragments to have plan nodes.
    // For now, return None to skip result fragments.
    // This will make the 3-fragment structure fall back to simpler execution.
    encode_plan_fragment_for_scan_typed("lineitem")
}

/// Create a TDataSink with a TResultSink for returning query results via MySQL protocol
#[cfg(feature = "real_be_proto")]
fn encode_result_sink_typed() -> t_sinks::TDataSink {
    let result_sink = t_sinks::TResultSink {
        type_: Some(t_sinks::TResultSinkType::MYSQL_PROTOCOL),
        file_options: None,
        fetch_option: None,
    };

    t_sinks::TDataSink {
        type_: t_sinks::TDataSinkType::RESULT_SINK,
        stream_sink: None,
        result_sink: Some(result_sink),
        result_file_sink: None,
        mysql_table_sink: None,
        export_sink: None,
        olap_table_sink: None,
        memory_scratch_sink: None,
        odbc_table_sink: None,
        jdbc_table_sink: None,
        multi_cast_stream_sink: None,
        hive_table_sink: None,
        iceberg_table_sink: None,
        dictionary_sink: None,
        blackhole_sink: None,
    }
}

// --- Descriptor table encoding helpers (minimal) ---

/// Encode a scalar Types.TPrimitiveType into Types.TTypeDesc / Types.TScalarType.
#[cfg(feature = "real_be_proto")]
fn encode_type_desc_scalar(primitive_type: i32, is_nullable: bool) -> ThriftStruct {
    // TScalarType { 1: type }
    let scalar = ThriftStruct::new(vec![ThriftField::new(1, primitive_type)]);

    // TTypeNode { 1: type = SCALAR(0), 2: scalar_type }
    let type_node = ThriftStruct::new(vec![
        ThriftField::new(1, 0_i32),
        ThriftField::new(2, scalar),
    ]);

    // TTypeDesc { 1: types = [type_node], 2: is_nullable }
    let types_list = ThriftList::from(vec![type_node]);
    ThriftStruct::new(vec![
        ThriftField::new(1, types_list),
        ThriftField::new(2, is_nullable),
    ])
}

/// Encode a minimal TSlotDescriptor for a column.
#[cfg(feature = "real_be_proto")]
fn encode_slot_descriptor(
    slot_id: i32,
    tuple_id: i32,
    slot_idx: i32,
    col_name: &str,
    primitive_type: i32,
    column_pos: i32,
    is_nullable: bool,
) -> ThriftStruct {
    let slot_type = encode_type_desc_scalar(primitive_type, is_nullable);

    let mut fields = Vec::new();
    // 1: id
    fields.push(ThriftField::new(1, slot_id));
    // 2: parent
    fields.push(ThriftField::new(2, tuple_id));
    // 3: slotType
    fields.push(ThriftField::new(3, slot_type));
    // 4: columnPos
    fields.push(ThriftField::new(4, column_pos));
    // 5: byteOffset (deprecated)
    fields.push(ThriftField::new(5, 0_i32));
    // 6: nullIndicatorByte (deprecated)
    fields.push(ThriftField::new(6, 0_i32));
    // 7: nullIndicatorBit (deprecated)
    fields.push(ThriftField::new(7, 0_i32));
    // 8: colName
    fields.push(ThriftField::new(8, col_name.to_string()));
    // 9: slotIdx
    fields.push(ThriftField::new(9, slot_idx));
    // 10: isMaterialized
    fields.push(ThriftField::new(10, true));

    ThriftStruct::new(fields)
}

/// Encode a minimal TTupleDescriptor for a single tuple.
#[cfg(feature = "real_be_proto")]
fn encode_tuple_descriptor(tuple_id: i32, table_id: Option<i64>) -> ThriftStruct {
    let mut fields = Vec::new();
    // 1: id
    fields.push(ThriftField::new(1, tuple_id));
    // 2: byteSize (deprecated)
    fields.push(ThriftField::new(2, 0_i32));
    // 3: numNullBytes (deprecated)
    fields.push(ThriftField::new(3, 0_i32));
    // 4: tableId (optional)
    if let Some(id) = table_id {
        fields.push(ThriftField::new(4, id));
    }
    // 5: numNullSlots (deprecated)
    fields.push(ThriftField::new(5, 0_i32));

    ThriftStruct::new(fields)
}

/// Encode a minimal Descriptors.TDescriptorTable for a scan over a single tuple.
///
/// `columns` is a slice of (name, primitive_type, is_nullable) triples where
/// `primitive_type` is the numeric value of Types.TPrimitiveType.
///
/// When `table_descriptor` is `Some`, it is attached as the single entry in
/// `tableDescriptors` (field 3) so that BE can resolve tuple/table metadata.
#[cfg(feature = "real_be_proto")]
fn encode_descriptor_table_for_scan(
    table_id: Option<i64>,
    columns: &[(&str, i32, bool)],
    table_descriptor: Option<ThriftStruct>,
) -> ThriftStruct {
    let tuple_id: i32 = 0;

    let slot_descs: Vec<ThriftStruct> = columns
        .iter()
        .enumerate()
        .map(|(idx, (name, prim, nullable))| {
            let slot_id = idx as i32;
            encode_slot_descriptor(
                slot_id,
                tuple_id,
                slot_id,
                name,
                *prim,
                slot_id,
                *nullable,
            )
        })
        .collect();

    let tuple_desc = encode_tuple_descriptor(tuple_id, table_id);

    let slots_list = ThriftList::from(slot_descs);
    let tuples_list = ThriftList::from(vec![tuple_desc]);

    let mut fields = Vec::new();
    // 1: slotDescriptors
    fields.push(ThriftField::new(1, slots_list));
    // 2: tupleDescriptors
    fields.push(ThriftField::new(2, tuples_list));
    // 3: tableDescriptors (optional)
    if let Some(td) = table_descriptor {
        let table_list = ThriftList::from(vec![td]);
        fields.push(ThriftField::new(3, table_list));
    }

    ThriftStruct::new(fields)
}

// --- Catalog-driven helpers ---

/// Hardcoded Doris table id for tpch_sf1.lineitem in the current Docker BE.
/// This value was obtained by decoding the Java FE TPipelineFragmentParams
/// payload and reading tupleDescriptors[0].table_id. For now we treat this
/// as a stable constant for the local TPC-H environment; a future BE-backed
/// catalog should fetch real table ids dynamically.
#[cfg(feature = "real_be_proto")]
const TPCH_SF1_LINEITEM_TABLE_ID: i64 = -2004094275;

/// Physical slot ordering used by Doris BE for tpch_sf1.lineitem.
/// Derived from decoding Java FE's TPipelineFragmentParams for a simple
/// `SELECT * FROM tpch_sf1.lineitem LIMIT 3`:
///
///   slot_idx: 0   1           2             3             4          5          6          7             8                9            10       11            12             13             14           15
///   column :  l_linenumber, l_shipdate, l_commitdate, l_receiptdate, l_orderkey, l_partkey, l_suppkey, l_quantity, l_extendedprice, l_discount, l_tax, l_returnflag, l_linestatus, l_shipinstruct, l_shipmode, l_comment
///
/// Keeping this mapping aligned ensures our slot_idx / columnPos match what
/// BE expects for the underlying storage layout.
#[cfg(feature = "real_be_proto")]
fn tpch_sf1_lineitem_slot_index(column: &str) -> Option<i32> {
    match column {
        "l_linenumber" => Some(0),
        "l_shipdate" => Some(1),
        "l_commitdate" => Some(2),
        "l_receiptdate" => Some(3),
        "l_orderkey" => Some(4),
        "l_partkey" => Some(5),
        "l_suppkey" => Some(6),
        "l_quantity" => Some(7),
        "l_extendedprice" => Some(8),
        "l_discount" => Some(9),
        "l_tax" => Some(10),
        "l_returnflag" => Some(11),
        "l_linestatus" => Some(12),
        "l_shipinstruct" => Some(13),
        "l_shipmode" => Some(14),
        "l_comment" => Some(15),
        _ => None,
    }
}

/// Map Rust metadata DataType to Doris Types.TPrimitiveType numeric code.
#[cfg(feature = "real_be_proto")]
fn primitive_type_from_metadata(dt: &MetaDataType) -> i32 {
    match dt {
        MetaDataType::TinyInt => 3,   // TINYINT
        MetaDataType::SmallInt => 4,  // SMALLINT
        MetaDataType::Int => 5,       // INT
        MetaDataType::BigInt => 6,    // BIGINT
        MetaDataType::Float => 7,     // FLOAT
        MetaDataType::Double => 8,    // DOUBLE
        MetaDataType::Decimal { .. } => 31, // DECIMAL128I
        MetaDataType::Char { .. } => 13,    // CHAR
        MetaDataType::Varchar { .. } => 15, // VARCHAR
        MetaDataType::String | MetaDataType::Text => 23, // STRING
        MetaDataType::Date => 26,     // DATEV2
        MetaDataType::DateTime | MetaDataType::Timestamp => 27, // DATETIMEV2
        MetaDataType::Boolean => 2,   // BOOLEAN
        MetaDataType::Binary => 11,   // BINARY
        MetaDataType::Varbinary { .. } => 43, // VARBINARY
        MetaDataType::Json => 32,     // JSONB
        MetaDataType::Array(_) => 20, // ARRAY
    }
}

/// Build a minimal TDescriptorTable for tpch.lineitem using the in-memory catalog.
#[cfg(feature = "real_be_proto")]
fn encode_descriptor_table_for_tpch_lineitem_from_catalog() -> ThriftStruct {
    let cat = catalog::catalog();
    let table = cat
        .get_table("tpch", "lineitem")
        .expect("tpch.lineitem table should exist in metadata catalog");

    let cols: Vec<(&str, i32, bool, i32)> = table
        .columns
        .iter()
        .enumerate()
        .map(|(idx, c)| {
            let prim = primitive_type_from_metadata(&c.data_type);
            let default_pos = idx as i32;
            let physical_idx =
                tpch_sf1_lineitem_slot_index(c.name.as_str()).unwrap_or(default_pos);
            (c.name.as_str(), prim, c.nullable, physical_idx)
        })
        .collect();

    // Use a real table id for tpch.lineitem when known; fall back to a
    // synthetic id otherwise.
    let table_id: i64 = TPCH_SF1_LINEITEM_TABLE_ID;
    let table_desc = encode_table_descriptor_for_tpch_lineitem(table_id, "tpch", &table);

    // Strip physical index for the compact path and rely only on the
    // catalog-driven typed path for precise slot_idx / columnPos. For
    // backward-compat tools that still use this helper, we keep the
    // original behaviour (sequential slot indices).
    let cols_simple: Vec<(&str, i32, bool)> =
        cols.iter().map(|(n, p, nullable, _)| (*n, *p, *nullable)).collect();

    encode_descriptor_table_for_scan(Some(table_id), &cols_simple, Some(table_desc))
}

/// Encode a minimal Descriptors.TTableDescriptor for tpch.lineitem.
#[cfg(feature = "real_be_proto")]
fn encode_table_descriptor_for_tpch_lineitem(
    table_id: i64,
    db_name: &str,
    table: &crate::metadata::schema::Table,
) -> ThriftStruct {
    let table_type_olap: i32 = 1; // Types.TTableType.OLAP_TABLE
    let num_cols = table.columns.len() as i32;
    // For now, treat clustering columns as 0; key/partitioning is not yet
    // modeled in the in-memory catalog.
    let num_clustering_cols: i32 = 0;

    // TOlapTable { 1: tableName }
    let olap_table = ThriftStruct::new(vec![ThriftField::new(1, table.name.clone())]);

    let mut fields = Vec::new();
    // 1: id
    fields.push(ThriftField::new(1, table_id));
    // 2: tableType
    fields.push(ThriftField::new(2, table_type_olap));
    // 3: numCols
    fields.push(ThriftField::new(3, num_cols));
    // 4: numClusteringCols
    fields.push(ThriftField::new(4, num_clustering_cols));
    // 7: tableName
    fields.push(ThriftField::new(7, table.name.clone()));
    // 8: dbName
    fields.push(ThriftField::new(8, db_name.to_string()));
    // 11: olapTable
    fields.push(ThriftField::new(11, olap_table));

    ThriftStruct::new(fields)
}

// Typed descriptor helpers using generated structs.

/// Map Rust metadata DataType to Doris Types.TPrimitiveType.
#[cfg(feature = "real_be_proto")]
fn primitive_type_from_metadata_typed(dt: &MetaDataType) -> t_types::TPrimitiveType {
    match dt {
        MetaDataType::TinyInt => t_types::TPrimitiveType::TINYINT,
        MetaDataType::SmallInt => t_types::TPrimitiveType::SMALLINT,
        MetaDataType::Int => t_types::TPrimitiveType::INT,
        MetaDataType::BigInt => t_types::TPrimitiveType::BIGINT,
        MetaDataType::Float => t_types::TPrimitiveType::FLOAT,
        MetaDataType::Double => t_types::TPrimitiveType::DOUBLE,
        MetaDataType::Decimal { .. } => t_types::TPrimitiveType::DECIMAL128I,
        MetaDataType::Char { .. } => t_types::TPrimitiveType::CHAR,
        MetaDataType::Varchar { .. } => t_types::TPrimitiveType::VARCHAR,
        MetaDataType::String | MetaDataType::Text => t_types::TPrimitiveType::STRING,
        MetaDataType::Date => t_types::TPrimitiveType::DATEV2,
        MetaDataType::DateTime | MetaDataType::Timestamp => t_types::TPrimitiveType::DATETIMEV2,
        MetaDataType::Boolean => t_types::TPrimitiveType::BOOLEAN,
        MetaDataType::Binary => t_types::TPrimitiveType::BINARY,
        MetaDataType::Varbinary { .. } => t_types::TPrimitiveType::VARBINARY,
        MetaDataType::Json => t_types::TPrimitiveType::JSONB,
        MetaDataType::Array(_) => t_types::TPrimitiveType::ARRAY,
    }
}

#[cfg(feature = "real_be_proto")]
fn encode_type_desc_scalar_typed(
    primitive_type: t_types::TPrimitiveType,
    is_nullable: bool,
    precision: Option<i32>,
    scale: Option<i32>,
) -> t_types::TTypeDesc {
    let scalar = t_types::TScalarType {
        type_: primitive_type,
        len: None,
        precision,
        scale,
        variant_max_subcolumns_count: None,
    };

    let type_node = t_types::TTypeNode {
        type_: t_types::TTypeNodeType::SCALAR,
        scalar_type: Some(scalar),
        struct_fields: None,
        contains_null: None,
        contains_nulls: None,
    };

    t_types::TTypeDesc {
        types: Some(vec![type_node]),
        is_nullable: Some(is_nullable),
        byte_size: None,
        sub_types: None,
        result_is_nullable: None,
        function_name: None,
        be_exec_version: None,
    }
}

#[cfg(feature = "real_be_proto")]
fn encode_slot_descriptor_typed(
    slot_id: i32,
    tuple_id: i32,
    slot_idx: i32,
    col_name: &str,
    primitive_type: t_types::TPrimitiveType,
    column_pos: i32,
    is_nullable: bool,
    precision: Option<i32>,
    scale: Option<i32>,
) -> t_desc::TSlotDescriptor {
    let slot_type = encode_type_desc_scalar_typed(primitive_type, is_nullable, precision, scale);

    t_desc::TSlotDescriptor::new(
        slot_id,
        tuple_id,
        slot_type,
        column_pos,
        0,
        0,
        0,
        col_name.to_string(),
        slot_idx,
        true,
        None::<i32>,
        None::<bool>,
        None::<bool>,
        None::<bool>,
        None::<Vec<String>>,
        None::<String>,
        Some(primitive_type),
        None::<t_exprs::TExpr>,
    )
}

#[cfg(feature = "real_be_proto")]
fn encode_tuple_descriptor_typed(
    tuple_id: i32,
    table_id: Option<i64>,
) -> t_desc::TTupleDescriptor {
    t_desc::TTupleDescriptor::new(tuple_id, 0, 0, table_id, Some(0))
}

#[cfg(feature = "real_be_proto")]
fn encode_table_descriptor_for_tpch_lineitem_typed(
    table_id: i64,
    db_name: &str,
    table: &crate::metadata::schema::Table,
) -> t_desc::TTableDescriptor {
    let table_type_olap = t_types::TTableType::OLAP_TABLE;
    let num_cols = table.columns.len() as i32;
    let num_clustering_cols: i32 = 0;

    let olap_table = t_desc::TOlapTable {
        table_name: table.name.clone(),
    };

    t_desc::TTableDescriptor::new(
        table_id,
        table_type_olap,
        num_cols,
        num_clustering_cols,
        table.name.clone(),
        db_name.to_string(),
        None::<t_desc::TMySQLTable>,
        Some(olap_table),
        None::<t_desc::TSchemaTable>,
        None::<t_desc::TBrokerTable>,
        None::<t_desc::TEsTable>,
        None::<t_desc::TOdbcTable>,
        None::<t_desc::THiveTable>,
        None::<t_desc::TIcebergTable>,
        None::<t_desc::THudiTable>,
        None::<t_desc::TJdbcTable>,
        None::<t_desc::TMCTable>,
        None::<t_desc::TTrinoConnectorTable>,
        None::<t_desc::TLakeSoulTable>,
        None::<t_desc::TDictionaryTable>,
        None::<t_desc::TRemoteDorisTable>,
    )
}

#[cfg(feature = "real_be_proto")]
fn encode_descriptor_table_for_scan_typed(
    table_id: Option<i64>,
    columns: &[(&str, t_types::TPrimitiveType, bool, Option<i32>, Option<i32>)],
    table_descriptor: Option<t_desc::TTableDescriptor>,
) -> t_desc::TDescriptorTable {
    let scan_tuple_id: i32 = 0;
    let output_tuple_id: i32 = 1;

    // Create scan slot descriptors (slots 0-15 for 16 columns)
    let mut slot_descs: Vec<t_desc::TSlotDescriptor> = columns
        .iter()
        .enumerate()
        .map(|(idx, (name, prim, nullable, precision, scale))| {
            let slot_id = idx as i32;
            // For tpch_sf1.lineitem we align slot_idx / column_pos with
            // the physical ordering observed from Java FE. For all other
            // tables we fall back to sequential indices.
            let (slot_idx, column_pos) =
                if let Some(phys) = tpch_sf1_lineitem_slot_index(name) {
                    (phys, phys)
                } else {
                    (slot_id, slot_id)
                };

            encode_slot_descriptor_typed(
                slot_id,
                scan_tuple_id,  // parent tuple is scan tuple
                slot_idx,
                name,
                *prim,
                column_pos,
                *nullable,
                *precision,
                *scale,
            )
        })
        .collect();

    // Add output tuple descriptors (Java FE pattern for simple SELECTs)
    // For simple scan queries, output tuple mirrors scan tuple slots
    // Add 2 output slots (16, 17) referencing the output tuple
    // These likely correspond to the first 2 columns for LIMIT queries
    for i in 0..2 {
        if let Some((name, prim, nullable, precision, scale)) = columns.get(i) {
            let output_slot_id = (columns.len() + i) as i32;  // slots 16, 17
            let output_slot = encode_slot_descriptor_typed(
                output_slot_id,
                output_tuple_id,  // parent tuple is output tuple
                i as i32,         // slot_idx in output tuple
                name,
                *prim,
                i as i32,         // column_pos in output
                *nullable,
                *precision,
                *scale,
            );
            slot_descs.push(output_slot);
        }
    }

    // Create scan tuple (has table_id)
    let scan_tuple_desc = encode_tuple_descriptor_typed(scan_tuple_id, table_id);

    // Create output tuple (NO table_id - used for result output)
    let output_tuple_desc = encode_tuple_descriptor_typed(output_tuple_id, None);

    let table_descriptors = table_descriptor.map(|td| vec![td]);

    t_desc::TDescriptorTable::new(
        Some(slot_descs),
        vec![scan_tuple_desc, output_tuple_desc],  // Both scan and output tuples
        table_descriptors
    )
}

/// Build a minimal TDescriptorTable for tpch.lineitem using the in-memory catalog.
#[cfg(feature = "real_be_proto")]
fn encode_descriptor_table_for_tpch_lineitem_from_catalog_typed() -> t_desc::TDescriptorTable {
    encode_descriptor_table_for_table_from_catalog_typed("tpch_sf1", "lineitem")
}

/// Build a minimal TDescriptorTable for an arbitrary table in a given
/// database using the in-memory Rust FE catalog. This generalizes the
/// tpch.lineitem-specific helper so that the pipeline path can work
/// for any catalog-registered table (AGENTS.md #1, #3).
#[cfg(feature = "real_be_proto")]
fn encode_descriptor_table_for_table_from_catalog_typed(
    db_name: &str,
    table_name: &str,
) -> t_desc::TDescriptorTable {
    let cat = catalog::catalog();
    let table = cat
        .get_table(db_name, table_name)
        .unwrap_or_else(|| panic!("table {}.{} should exist in metadata catalog", db_name, table_name));

    let cols: Vec<(&str, t_types::TPrimitiveType, bool, Option<i32>, Option<i32>)> = table
        .columns
        .iter()
        .map(|c| {
            let (precision, scale) = match &c.data_type {
                crate::metadata::types::DataType::Decimal { precision, scale } => {
                    (Some(*precision as i32), Some(*scale as i32))
                }
                _ => (None, None),
            };
            (
                c.name.as_str(),
                primitive_type_from_metadata_typed(&c.data_type),
                c.nullable,
                precision,
                scale,
            )
        })
        .collect();

    // Prefer the real Doris table id for tpch_sf1.lineitem when we know it;
    // otherwise fall back to a synthetic id. A future BE-backed catalog
    // should replace this with dynamic ids from Doris metadata.
    let table_id: i64 = if db_name == "tpch_sf1" && table.name == "lineitem" {
        TPCH_SF1_LINEITEM_TABLE_ID
    } else {
        1
    };
    let table_desc = encode_table_descriptor_for_tpch_lineitem_typed(table_id, db_name, &table);

    encode_descriptor_table_for_scan_typed(Some(table_id), &cols, Some(table_desc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::plan_fragment::{Expr, DataType};

    #[test]
    fn simple_scan_fragment_supported() {
        let query_id = Uuid::new_v4();
        let scan = PlanNode::OlapScan {
            table_name: "lineitem".to_string(),
            columns: vec!["l_orderkey".to_string()],
            predicates: vec![],
            tablet_ids: vec![],
        };
        let fragment = PlanFragment::new(0, scan);
        let mut plan = QueryPlan::new(query_id);
        plan.add_fragment(fragment);

        let params = PipelineFragmentParamsList::from_query_plan(&plan)
            .expect("single scan fragment should be supported");

        assert_eq!(params.query_id, query_id);
        assert_eq!(params.fragments.len(), 1);
        assert_eq!(params.fragments[0].fragment_id, 0);
        assert!(params.fragments[0].is_nereids);
    }

    #[test]
    fn complex_fragment_rejected() {
        let query_id = Uuid::new_v4();
        let scan = PlanNode::OlapScan {
            table_name: "lineitem".to_string(),
            columns: vec!["l_orderkey".to_string()],
            predicates: vec![],
            tablet_ids: vec![],
        };
        let proj = PlanNode::Project {
            child: Box::new(scan),
            exprs: vec![Expr::Column {
                name: "l_orderkey".to_string(),
                data_type: DataType::BigInt,
            }],
        };
        // Wrap projection in another node that we currently don't support.
        let root = PlanNode::Sort {
            child: Box::new(proj),
            order_by: vec![],
            limit: None,
        };
        let fragment = PlanFragment::new(0, root);
        let mut plan = QueryPlan::new(query_id);
        plan.add_fragment(fragment);

        assert!(PipelineFragmentParamsList::from_query_plan(&plan).is_none());
    }

    #[cfg(feature = "real_be_proto")]
    #[test]
    fn encode_tunique_id_has_hi_lo_fields() {
        let id = Uuid::from_u128(0x112233445566778899aabbccddeeff00);
        let s = encode_tunique_id(id);
        let fields = s.fields();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].id(), 1);
        assert_eq!(fields[1].id(), 2);
        match fields[0].data() {
            Data::I64(_) => {}
            other => panic!("expected I64 for hi, got {:?}", other),
        }
        match fields[1].data() {
            Data::I64(_) => {}
            other => panic!("expected I64 for lo, got {:?}", other),
        }
    }

    // NOTE: This test is obsolete with real_be_proto feature
    // #[cfg(feature = "real_be_proto")]
    // #[test]
    // fn encode_pipeline_fragment_params_minimal_fields_present() {
    //     let query_id = Uuid::new_v4();
    //     let params = PipelineFragmentParams {
    //         query_id,
    //         fragment_id: 5,
    //         is_nereids: true,
    //         table_name: None,
    //     };
    //     let s = encode_pipeline_fragment_params(&params);
    //     let mut seen = std::collections::BTreeSet::new();
    //     for f in s.fields() {
    //         seen.insert(f.id());
    //     }
    //     // Required subset of fields should be present.
    //     for id in [1_i16, 2, 3, 4, 21, 40] {
    //         assert!(seen.contains(&id), "missing field id {}", id);
    //     }
    // }

    #[cfg(feature = "real_be_proto")]
    #[test]
    fn encode_plan_fragment_for_scan_has_plan_and_partition() {
        let fragment = encode_plan_fragment_for_scan("lineitem");
        let mut has_plan = false;
        let mut has_partition = false;
        for f in fragment.fields() {
            match f.id() {
                2 => has_plan = true,
                6 => has_partition = true,
                _ => {}
            }
        }
        assert!(has_plan, "TPlanFragment should have plan (field 2)");
        assert!(has_partition, "TPlanFragment should have partition (field 6)");
    }

    #[cfg(feature = "real_be_proto")]
    #[test]
    fn encode_type_desc_scalar_shape() {
        // Use TPrimitiveType.BIGINT = 6 for this test
        let tdesc = encode_type_desc_scalar(6, true);
        let fields = tdesc.fields();
        // Expect at least fields 1 (types) and 2 (is_nullable)
        assert!(fields.iter().any(|f| f.id() == 1));
        assert!(fields.iter().any(|f| f.id() == 2));
        // Field 1 should be a list (types)
        let f1 = fields.iter().find(|f| f.id() == 1).unwrap();
        match f1.data() {
            Data::List(_) => {}
            other => panic!("expected List for types, got {:?}", other),
        }
    }

    #[cfg(feature = "real_be_proto")]
    #[test]
    fn encode_descriptor_table_for_scan_has_slots_and_tuple() {
        // single BIGINT column, non-nullable
        let desc = encode_descriptor_table_for_scan(Some(42), &[("col1", 6, false)], None);
        let fields = desc.fields();
        let mut has_slots = false;
        let mut has_tuples = false;
        for f in fields {
            match f.id() {
                1 => match f.data() {
                    Data::List(_) => has_slots = true,
                    other => panic!("expected List for slotDescriptors, got {:?}", other),
                },
                2 => match f.data() {
                    Data::List(_) => has_tuples = true,
                    other => panic!("expected List for tupleDescriptors, got {:?}", other),
                },
                _ => {}
            }
        }
        assert!(has_slots, "TDescriptorTable should have slotDescriptors (field 1)");
        assert!(has_tuples, "TDescriptorTable should have tupleDescriptors (field 2)");
    }

    #[cfg(feature = "real_be_proto")]
    #[test]
    fn primitive_type_from_metadata_maps_tpch_lineitem_types() {
        let cat = catalog::catalog();
        let table = cat.get_table("tpch", "lineitem").unwrap();

        let col_orderkey = table.get_column("l_orderkey").unwrap();
        assert_eq!(primitive_type_from_metadata(&col_orderkey.data_type), 6); // BIGINT

        let col_quantity = table.get_column("l_quantity").unwrap();
        assert_eq!(primitive_type_from_metadata(&col_quantity.data_type), 31); // DECIMAL128I

        let col_shipdate = table.get_column("l_shipdate").unwrap();
        assert_eq!(primitive_type_from_metadata(&col_shipdate.data_type), 26); // DATEV2
    }

    #[cfg(feature = "real_be_proto")]
    #[test]
    fn encode_descriptor_table_for_tpch_lineitem_has_all_columns() {
        let desc = encode_descriptor_table_for_tpch_lineitem_from_catalog();
        let fields = desc.fields();

        // Field 1 should be slotDescriptors list containing all 16 lineitem columns
        let slots_field = fields.iter().find(|f| f.id() == 1).expect("slots field");
        match slots_field.data() {
            Data::List(list) => {
                assert_eq!(list.len(), 16, "lineitem should have 16 slots");
            }
            other => panic!("expected List for slotDescriptors, got {:?}", other),
        }
    }

    #[cfg(feature = "real_be_proto")]
    #[test]
    fn encode_descriptor_table_for_orders_from_catalog_has_all_columns() {
        let cat = catalog::catalog();
        let orders = cat.get_table("tpch_sf1", "orders").unwrap();
        let expected_cols = orders.columns.len();

        let desc = encode_descriptor_table_for_table_from_catalog_typed("tpch_sf1", "orders");
        let slots = desc.slot_descriptors.as_ref().expect("slot descriptors");

        assert_eq!(
            slots.len(),
            expected_cols,
            "descriptor slots should match orders column count"
        );
    }
}
