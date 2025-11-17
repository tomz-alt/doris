//! Enhanced decoder that specifically inspects scan ranges in TPipelineFragmentParamsList
//!
//! Usage:
//!   cargo run --no-default-features --features real_be_proto \
//!     --example decode_scan_ranges -- /path/to/thrift.bin

use std::env;
use std::fs::File;
use std::io::Read;

use thrift::protocol::{TCompactInputProtocol, TInputProtocol, TSerializable};
use thrift::transport::TBufferChannel;

use doris_thrift::palo_internal_service::TPipelineFragmentParamsList;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .expect("usage: decode_scan_ranges <file>");

    let mut buf = Vec::new();
    File::open(&path)?.read_to_end(&mut buf)?;

    println!("📦 Payload file size: {} bytes", buf.len());

    // Feed the captured payload into an in-memory Thrift transport.
    let mut channel = TBufferChannel::with_capacity(buf.len(), 0);
    channel.set_readable_bytes(&buf);

    let mut i_prot = TCompactInputProtocol::new(channel);
    let value = TPipelineFragmentParamsList::read_from_in_protocol(&mut i_prot)
        .map_err(|e| format!("failed to decode compact Thrift payload: {e}"))?;

    let num_params = value
        .params_list
        .as_ref()
        .map(|v| v.len())
        .unwrap_or(0);
    println!("✓ Successfully decoded Thrift payload");
    println!("  params_list.len = {num_params}\n");

    let mut total_backends = 0usize;
    let mut total_scan_ranges = 0usize;

    if let Some(params_list) = value.params_list.as_ref() {
        println!("═══════════════════════════════════════");
        println!("  SCAN RANGES ANALYSIS");
        println!("═══════════════════════════════════════\n");

        for (p_idx, params) in params_list.iter().enumerate() {
            println!("▶ Fragment [{}]", p_idx);
            println!("  fragment_id: {:?}", params.fragment_id);
            println!("  table_name: {:?}", params.table_name);
            println!("  is_nereids: {:?}", params.is_nereids);

            let local_params_len = params
                .local_params
                .as_ref()
                .map(|v| v.len())
                .unwrap_or(0);
            println!("  local_params.len = {}", local_params_len);
            total_backends += local_params_len;

            if let Some(ref local_params) = params.local_params {
                for (lp_idx, local_param) in local_params.iter().enumerate() {
                    println!("\n  ┌─ local_param[{lp_idx}] (Backend shard)");
                    println!("  │  backend_num: {:?}", local_param.backend_num);

                    let scan_ranges_map = &local_param.per_node_scan_ranges;
                    println!("  │  per_node_scan_ranges: {} nodes", scan_ranges_map.len());

                    for (node_id, scan_range_params_vec) in scan_ranges_map {
                        println!("  │");
                        println!("  │  ┌─ node_id={} → {} scan ranges", node_id, scan_range_params_vec.len());
                        total_scan_ranges += scan_range_params_vec.len();

                        for (sr_idx, scan_range_params) in scan_range_params_vec.iter().enumerate() {
                            println!("  │  │");
                            println!("  │  │  ╔═ scan_range[{}]", sr_idx);

                            let scan_range = &scan_range_params.scan_range;

                            if let Some(ref palo_scan) = scan_range.palo_scan_range {
                                println!("  │  │  ║  Type: TPaloScanRange");
                                println!("  │  │  ║  tablet_id: {}", palo_scan.tablet_id);
                                println!("  │  │  ║  schema_hash: {:?}", palo_scan.schema_hash);
                                println!("  │  │  ║  version: {:?}", palo_scan.version);
                                println!("  │  │  ║  version_hash: {:?}", palo_scan.version_hash);
                                println!("  │  │  ║  db_name: {:?}", palo_scan.db_name);
                                println!("  │  │  ║  table_name: {:?}", palo_scan.table_name);
                                println!("  │  │  ║  index_name: {:?}", palo_scan.index_name);
                                println!("  │  │  ║  hosts.len: {}", palo_scan.hosts.len());
                                for (h_idx, host) in palo_scan.hosts.iter().enumerate() {
                                    println!("  │  │  ║    host[{}]: {}:{}", h_idx, host.hostname, host.port);
                                }
                            } else if scan_range.broker_scan_range.is_some() {
                                println!("  │  │  ║  Type: TBrokerScanRange");
                            } else if scan_range.es_scan_range.is_some() {
                                println!("  │  │  ║  Type: TEsScanRange");
                            } else {
                                println!("  │  │  ║  Type: UNKNOWN (no variant set)");
                            }
                            println!("  │  │  ╚═");
                        }
                        println!("  │  └─");
                    }
                    println!("  └─");
                }
            } else {
                println!("  ⚠️  local_params is None for this fragment!");
            }

            println!();
        }

        println!("═══════════════════════════════════════");
        println!("  SUMMARY");
        println!("═══════════════════════════════════════");
        println!("Payload size: {} bytes", buf.len());
        println!("Fragments (params_list.len): {}", num_params);
        println!("Total backends (local_params across all fragments): {}", total_backends);
        println!("Total scan ranges (all fragments / nodes): {}", total_scan_ranges);
    }

    Ok(())
}
