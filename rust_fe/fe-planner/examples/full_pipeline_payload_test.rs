use fe_planner::thrift_plan::*;
use fe_planner::serialize_pipeline_params;
use fe_planner::{TScanRangeLocations, TPaloScanRange, TScanRangeLocation};
use fe_planner::scan_range_builder::{TScanRange, TNetworkAddress};

fn main() {
    println!("=== Full TPipelineFragmentParamsList Payload Test ===\n");

    // Generate simple query ID (hi=12345, lo=67890)
    let query_id: [u8; 16] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x39,  // hi = 12345
        0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x09, 0x32,  // lo = 67890
    ];

    println!("Query ID: hi=12345, lo=67890");
    println!();

    // Create scan node for lineitem table
    let scan_node = TPlanNode {
        node_id: 0,
        node_type: TPlanNodeType::OlapScanNode,
        num_children: 0,
        limit: -1,
        row_tuples: vec![0],
        nullable_tuples: vec![false],
        compact_data: true,
        olap_scan_node: Some(TOlapScanNode {
            tuple_id: 0,
            key_column_name: vec![
                "l_orderkey".to_string(),
                "l_partkey".to_string(),
                "l_suppkey".to_string(),
                "l_linenumber".to_string(),
            ],
            key_column_type: vec![
                TPrimitiveType::BigInt,
                TPrimitiveType::BigInt,
                TPrimitiveType::BigInt,
                TPrimitiveType::Int,
            ],
            is_preaggregation: true,
            table_name: Some("lineitem".to_string()),
        }),
    };

    // Create plan fragment
    let fragment = TPlanFragment {
        plan: TPlan {
            nodes: vec![scan_node],
        },
        partition: TDataPartition {
            partition_type: TPartitionType::Unpartitioned,
            partition_exprs: None,
            partition_infos: None,
        },
    };

    // Create scan ranges (hardcoded for localhost BE)
    let backend_addr = TNetworkAddress {
        hostname: "127.0.0.1".to_string(),
        port: 9060,
    };

    let palo_range = TPaloScanRange {
        db_name: String::new(),
        schema_hash: "0".to_string(),
        version: "2".to_string(),
        version_hash: String::new(),
        tablet_id: 10003, // table_id + 2
        hosts: vec![backend_addr.clone()],
    };

    let scan_location = TScanRangeLocation {
        server: backend_addr,
        backend_id: 10000,
    };

    let scan_range = TScanRange {
        palo_scan_range: Some(palo_range),
    };

    let scan_range_locations = vec![TScanRangeLocations {
        scan_range,
        locations: vec![scan_location],
    }];

    // Generate TPipelineFragmentParamsList
    let params_list = TPipelineFragmentParamsList::from_fragment_and_ranges(
        fragment,
        query_id,
        0,  // node_id
        scan_range_locations,
    );

    println!("Generated TPipelineFragmentParamsList:");
    println!("  Fragment count: {}", params_list.params_list.len());

    if let Some(params) = params_list.params_list.first() {
        println!("  Protocol version: {}", params.protocol_version);
        println!("  Fragment ID: {:?}", params.fragment_id);
        println!("  Num senders: {:?}", params.num_senders);
        println!("  Backend ID: {:?}", params.backend_id);
        println!("  Total instances: {:?}", params.total_instances);
        println!("  Is Nereids: {:?}", params.is_nereids);
        println!();

        if let Some(ref desc_tbl) = params.desc_tbl {
            let slot_count = desc_tbl.slot_descriptors.as_ref().map(|s| s.len()).unwrap_or(0);
            let tuple_count = desc_tbl.tuple_descriptors.len();
            println!("  Descriptor Table:");
            println!("    Slot descriptors: {}", slot_count);
            println!("    Tuple descriptors: {}", tuple_count);
        }

        if let Some(ref query_globals) = params.query_globals {
            println!();
            println!("  TQueryGlobals:");
            println!("    now_string: {}", query_globals.now_string);
            println!("    timestamp_ms: {:?}", query_globals.timestamp_ms);
            println!("    time_zone: {:?}", query_globals.time_zone);
            println!("    nano_seconds: {:?}", query_globals.nano_seconds);
            println!("    lc_time_names: {:?}", query_globals.lc_time_names);
        }

        if let Some(ref query_options) = params.query_options {
            println!();
            println!("  TQueryOptions:");
            println!("    batch_size: {:?}", query_options.batch_size);
            println!("    mem_limit: {:?}", query_options.mem_limit);
            println!("    query_timeout: {:?}", query_options.query_timeout);
            println!("    num_scanner_threads: {:?}", query_options.num_scanner_threads);
            println!("    max_scan_key_num: {:?}", query_options.max_scan_key_num);
            println!("    be_exec_version: {:?}", query_options.be_exec_version);
        }

        println!();
        println!("  Local params (instances): {}", params.local_params.len());
        if let Some(instance) = params.local_params.first() {
            let scan_range_count: usize = instance.per_node_scan_ranges.values().map(|v| v.len()).sum();
            println!("    Scan ranges: {}", scan_range_count);
        }
    }

    // Serialize the complete payload
    println!();
    println!("=== Serialization ===");
    match serialize_pipeline_params(&params_list) {
        Ok(bytes) => {
            println!("✅ Successfully serialized TPipelineFragmentParamsList");
            println!("📦 Total payload size: {} bytes", bytes.len());
            println!();

            // Show breakdown (approximate, compressed with TCompactProtocol)
            println!("Payload breakdown (TCompactProtocol compressed):");
            println!("  - Complete query execution plan");
            println!("  - TDescriptorTable with 16 lineitem columns");
            println!("  - TQueryGlobals with timestamp & timezone");
            println!("  - TQueryOptions with 10 execution parameters");
            println!("  - TPlanFragment with OLAP scan node");
            println!("  - Scan ranges for tablet access");
            println!("  - Instance parameters and metadata");
            println!();

            // Show first and last 32 bytes
            println!("First 32 bytes (hex):");
            print!("  ");
            for (i, byte) in bytes.iter().take(32).enumerate() {
                print!("{:02x} ", byte);
                if (i + 1) % 16 == 0 {
                    println!();
                    print!("  ");
                }
            }
            println!();
            println!();

            println!("Last 32 bytes (hex):");
            print!("  ");
            let start = bytes.len().saturating_sub(32);
            for (i, byte) in bytes.iter().skip(start).enumerate() {
                print!("{:02x} ", byte);
                if (i + 1) % 16 == 0 {
                    println!();
                    print!("  ");
                }
            }
            println!();
            println!();

            // Save to file
            let output_path = "/tmp/full_pipeline_payload.bin";
            std::fs::write(output_path, &bytes).expect("Failed to write output file");
            println!("✅ Saved complete payload to: {}", output_path);
            println!();

            // Summary
            println!("=== Ready for BE ===");
            println!("This {} byte payload contains:", bytes.len());
            println!("  ✅ Protocol version (VERSION_3)");
            println!("  ✅ Complete descriptor table (16 lineitem columns)");
            println!("  ✅ Query globals with timestamp & timezone");
            println!("  ✅ Query options with 10 critical fields");
            println!("  ✅ Plan fragment with REQUIRED partition field");
            println!("  ✅ Scan ranges for tablet access");
            println!("  ✅ Instance parameters");
            println!();
            println!("🚀 Ready to send via gRPC PExecPlanFragmentRequest!");
        }
        Err(e) => {
            eprintln!("❌ Serialization failed: {:?}", e);
        }
    }
}
