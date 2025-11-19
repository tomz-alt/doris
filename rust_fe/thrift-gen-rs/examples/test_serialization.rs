//! Test auto-generated Thrift serialization

use doris_thrift::*;
use thrift::protocol::{TCompactOutputProtocol, TSerializable};
use thrift::transport::TBufferChannel;

fn main() {
    println!("Testing auto-generated Thrift serialization...");
    
    // Create minimal TPipelineFragmentParamsList
    let params_list = TPipelineFragmentParamsList {
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
        query_id: Some(TUniqueId { hi: 12345, lo: 67890 }),
        topn_filter_source_node_ids: None,
        runtime_filter_merge_addr: None,
        runtime_filter_info: None,
    };
    
    // Serialize using TCompactProtocol
    let mut transport = TBufferChannel::with_capacity(0, 4096);
    let mut protocol = TCompactOutputProtocol::new(&mut transport);
    
    match params_list.write_to_out_protocol(&mut protocol) {
        Ok(()) => {
            let bytes = transport.write_bytes();
            println!("✅ Serialization successful!");
            println!("   Payload size: {} bytes", bytes.len());
            println!("   First 32 bytes: {}", 
                bytes.iter().take(32).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));
        }
        Err(e) => {
            println!("❌ Serialization failed: {}", e);
        }
    }
}
