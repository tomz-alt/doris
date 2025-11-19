//! Conversion between manual fe-planner types and auto-generated doris-thrift types
//!
//! This is a temporary bridge until fe-planner is fully migrated to use auto-generated types.

use doris_thrift;
use fe_common::Result;
use thrift::protocol::{TCompactOutputProtocol, TSerializable};
use thrift::transport::TBufferChannel;

/// Serialize pipeline params using auto-generated Thrift code
///
/// For now, this creates a minimal auto-generated structure.
/// TODO: Properly convert all fields from manual types
pub fn serialize_with_autogen(
    _manual_params: &fe_planner::TPipelineFragmentParamsList,
    query_id_bytes: [u8; 16],
) -> Result<Vec<u8>> {
    // Convert query_id bytes to TUniqueId
    let query_id = doris_thrift::TUniqueId {
        hi: i64::from_be_bytes([
            query_id_bytes[0], query_id_bytes[1], query_id_bytes[2], query_id_bytes[3],
            query_id_bytes[4], query_id_bytes[5], query_id_bytes[6], query_id_bytes[7],
        ]),
        lo: i64::from_be_bytes([
            query_id_bytes[8], query_id_bytes[9], query_id_bytes[10], query_id_bytes[11],
            query_id_bytes[12], query_id_bytes[13], query_id_bytes[14], query_id_bytes[15],
        ]),
    };

    // Create minimal TPipelineFragmentParamsList using auto-generated types
    // TODO: Convert all fields from manual_params
    let autogen_params = doris_thrift::TPipelineFragmentParamsList {
        params_list: Some(vec![]),
        desc_tbl: None,
        file_scan_params: None,
        coord: None,
        query_globals: None,
        resource_info: None,
        fragment_num_on_host: None,
        query_options: None,
        is_nereids: Some(true),
        workload_groups: None,
        query_id: Some(query_id),
        topn_filter_source_node_ids: None,
        runtime_filter_merge_addr: None,
        runtime_filter_info: None,
    };

    // Serialize using auto-generated TSerializable implementation
    let mut transport = TBufferChannel::with_capacity(0, 4096);
    let mut protocol = TCompactOutputProtocol::new(&mut transport);

    autogen_params
        .write_to_out_protocol(&mut protocol)
        .map_err(|e| fe_common::DorisError::InternalError(format!("Thrift serialization failed: {}", e)))?;

    Ok(transport.write_bytes().to_vec())
}
