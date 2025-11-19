// Test program to dump hex representation of TPipelineFragmentParamsList
// This helps debug Thrift serialization issues with BE

use fe_planner::thrift_plan::*;
use fe_planner::thrift_serialize::serialize_pipeline_params;
use std::collections::HashMap;

fn main() {
    println!("Creating minimal TPipelineFragmentParamsList for debugging\n");

    let unique_id = TUniqueId { hi: 12345, lo: 67890 };

    // Minimal descriptor table with all required fields
    let desc_tbl = Some(TDescriptorTable {
        tuple_descriptors: vec![],
        slot_descriptors: None,
        table_descriptors: None,
    });

    // Minimal plan - just an empty plan
    let fragment = TPlanFragment {
        plan: TPlan {
            nodes: vec![],
        },
    };

    // Create minimal local params
    let local_params = vec![TPipelineInstanceParams {
        fragment_instance_id: unique_id.clone(),
        backend_num: Some(0),
        sender_id: Some(0),
        per_node_scan_ranges: HashMap::new(),
    }];

    // Create minimal params matching Java FE structure
    let params = TPipelineFragmentParams {
        protocol_version: 0,
        query_id: unique_id.clone(),
        fragment_id: Some(0),
        per_exch_num_senders: HashMap::new(),
        desc_tbl,
        destinations: Vec::new(),  // Empty but REQUIRED (not optional!)
        fragment,
        local_params,
        coord: None,
        num_senders: Some(1),
        query_globals: Some(TQueryGlobals::minimal()),
        query_options: Some(TQueryOptions::minimal()),
        fragment_num_on_host: Some(1),
        backend_id: Some(10001),
        total_instances: Some(1),
        is_nereids: Some(true),
    };

    // Create params list
    let params_list = TPipelineFragmentParamsList {
        params_list: vec![params],
    };

    // Serialize using the real serialization code
    match serialize_pipeline_params(&params_list) {
        Ok(bytes) => {
            println!("✓ Successfully serialized {} bytes\n", bytes.len());
            println!("Hex dump:");
            print_hex_dump(&bytes);

            println!("\n\nFirst 100 bytes (for quick inspection):");
            for (i, byte) in bytes.iter().take(100).enumerate() {
                if i % 16 == 0 {
                    print!("\n{:04x}: ", i);
                }
                print!("{:02x} ", byte);
            }
            println!("\n");

            // Also save to file
            std::fs::write("/tmp/rust_thrift_dump.bin", &bytes)
                .expect("Failed to write binary dump");
            println!("Binary dump saved to /tmp/rust_thrift_dump.bin");
        }
        Err(e) => {
            eprintln!("✗ Serialization error: {:?}", e);
            std::process::exit(1);
        }
    }
}

fn print_hex_dump(bytes: &[u8]) {
    for (i, chunk) in bytes.chunks(16).enumerate() {
        print!("{:08x}  ", i * 16);

        // Print hex
        for (j, byte) in chunk.iter().enumerate() {
            print!("{:02x} ", byte);
            if j == 7 {
                print!(" ");
            }
        }

        // Padding
        for _ in chunk.len()..16 {
            print!("   ");
            if chunk.len() <= 8 {
                print!(" ");
            }
        }

        // Print ASCII
        print!(" |");
        for byte in chunk {
            if *byte >= 32 && *byte < 127 {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
}
