use fe_backend_client::BackendClient;
use fe_planner::thrift_plan::*;
use fe_planner::{TScanRangeLocations, TPaloScanRange, TScanRangeLocation};
use fe_planner::scan_range_builder::{TScanRange, TNetworkAddress};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() {
    println!("=== Rust FE → C++ BE Integration Test ===\n");

    // Configuration
    let be_host = "127.0.0.1";
    let be_port = 8060; // BE brpc port (8060) - trying instead of 9060

    // Generate unique query ID from timestamp
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let hi = now.as_secs();
    let lo = now.subsec_nanos() as u64;
    let query_id: [u8; 16] = [
        (hi >> 56) as u8, (hi >> 48) as u8, (hi >> 40) as u8, (hi >> 32) as u8,
        (hi >> 24) as u8, (hi >> 16) as u8, (hi >> 8) as u8, hi as u8,
        (lo >> 56) as u8, (lo >> 48) as u8, (lo >> 40) as u8, (lo >> 32) as u8,
        (lo >> 24) as u8, (lo >> 16) as u8, (lo >> 8) as u8, lo as u8,
    ];

    println!("Configuration:");
    println!("  BE Address: {}:{}", be_host, be_port);
    println!("  Query ID: hi={}, lo={} (timestamp-based)", hi, lo);
    println!();

    // Create scan node for lineitem table
    println!("Step 1: Creating query plan...");
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
                TPrimitiveType::BigInt,  // l_orderkey: BigInt (catalog)
                TPrimitiveType::BigInt,  // l_partkey: BigInt (catalog)
                TPrimitiveType::BigInt,  // l_suppkey: BigInt (catalog)
                TPrimitiveType::Int,     // l_linenumber: Int (catalog - NOT BigInt!)
            ],
            is_preaggregation: true,
            table_name: Some("lineitem".to_string()),
            // CRITICAL: Full column descriptors for BE to read from storage!
            columns_desc: Some(fe_planner::thrift_plan::build_lineitem_columns_desc()),
            // CRITICAL: Table keys type - lineitem is DUPLICATE KEY table!
            key_type: Some(1),  // TKeysType::DUP_KEYS = 1
            // CRITICAL: Tell BE which columns to read and return (all 16 for SELECT *)
            output_column_unique_ids: Some(vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
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

    println!("  ✅ Plan fragment created");
    println!("     - 1 OLAP scan node");
    println!("     - lineitem table");
    println!("     - 4 key columns");
    println!();

    // Create scan ranges
    println!("Step 2: Generating scan ranges...");
    let backend_addr = TNetworkAddress {
        hostname: be_host.to_string(),
        port: be_port as i32,
    };

    let palo_range = TPaloScanRange {
        db_name: String::new(),
        schema_hash: "0".to_string(),  // Java FE uses "0" - not the real hash! (deprecated field?)
        version: "8".to_string(),  // Latest version in rowset [2-8] which contains our 4.06KB data
        version_hash: String::new(),
        tablet_id: 1763520834036, // REAL tablet ID from Java FE
        hosts: vec![backend_addr.clone()],
    };

    let scan_location = TScanRangeLocation {
        server: backend_addr,
        backend_id: 1763520834074, // REAL backend ID from Java FE
    };

    let scan_range = TScanRange {
        palo_scan_range: Some(palo_range),
    };

    let scan_ranges = vec![TScanRangeLocations {
        scan_range,
        locations: vec![scan_location],
    }];

    println!("  ✅ Scan ranges generated");
    println!("     - Tablet ID: 1763520834036 (REAL from Java FE)");
    println!("     - Backend ID: 1763520834074 (REAL from Java FE)");
    println!("     - Backend: {}:{}", be_host, be_port);
    println!("     - Version: 8 (range [0-8])");
    println!();

    // Connect to BE
    println!("Step 3: Connecting to BE at {}:{}...", be_host, be_port);
    let mut client = match BackendClient::new(be_host, be_port).await {
        Ok(c) => {
            println!("  ✅ Connected to BE successfully!");
            println!();
            c
        }
        Err(e) => {
            eprintln!("  ❌ Failed to connect to BE: {:?}", e);
            eprintln!();
            eprintln!("BE is not running. To start the BE:");
            eprintln!("  1. Build Doris BE: cd /home/user/doris && ./build.sh --be");
            eprintln!("  2. Start BE: cd /home/user/doris && ./bin/start_be.sh --daemon");
            eprintln!("  3. Run this test again");
            eprintln!();
            eprintln!("Note: This test demonstrates the complete Rust FE → C++ BE flow:");
            eprintln!("  - TPlanFragment with lineitem OLAP scan");
            eprintln!("  - TDescriptorTable with 16 columns");
            eprintln!("  - TQueryGlobals with timestamp");
            eprintln!("  - TQueryOptions with 10 execution parameters");
            eprintln!("  - TScanRangeLocations for tablet access");
            eprintln!("  - Serialized to 1,053 bytes TCompactProtocol");
            eprintln!("  - Ready to send via gRPC PExecPlanFragmentRequest");
            std::process::exit(1);
        }
    };

    // Execute plan fragment
    println!("Step 4: Executing query fragment on BE...");
    println!("  Sending TPipelineFragmentParamsList:");
    println!("    - Protocol version: VERSION_3");
    println!("    - Complete descriptor table (16 lineitem columns)");
    println!("    - Query globals (timestamp + timezone)");
    println!("    - Query options (10 critical fields)");
    println!("    - Plan fragment with partition");
    println!("    - Scan ranges");
    println!();

    match client.exec_plan_fragment(&fragment, query_id, &scan_ranges, 0).await {
        Ok(finst_id) => {
            println!("  ✅ Fragment execution started!");
            println!("     Fragment instance ID: {:?}", finst_id);
            println!();

            // Fetch results
            println!("Step 5: Fetching query results...");
            match client.fetch_data(finst_id).await {
                Ok(rows) => {
                    println!("  ✅ Query executed successfully!");
                    println!("     Rows returned: {}", rows.len());
                    println!();

                    if !rows.is_empty() {
                        println!("First few rows:");
                        for (i, row) in rows.iter().take(5).enumerate() {
                            println!("  Row {}: {} columns", i + 1, row.values.len());
                        }
                    }
                    println!();
                    println!("🎉 E2E Query Execution Complete!");
                    println!("   Rust FE successfully sent query to C++ BE and got results!");
                }
                Err(e) => {
                    eprintln!("  ❌ Failed to fetch results: {:?}", e);
                    eprintln!();
                    eprintln!("Fragment execution started but result fetch failed.");
                    eprintln!("This might be a timeout or data format issue.");
                }
            }
        }
        Err(e) => {
            eprintln!("  ❌ Failed to execute fragment: {:?}", e);
            eprintln!();
            eprintln!("Possible issues:");
            eprintln!("  1. BE received invalid Thrift payload");
            eprintln!("  2. Tablet 10003 doesn't exist in BE");
            eprintln!("  3. lineitem table not created in BE");
            eprintln!("  4. BE version mismatch");
            eprintln!();
            eprintln!("To debug:");
            eprintln!("  - Check BE logs: /home/user/doris/log/be.INFO");
            eprintln!("  - Verify BE is running: ps aux | grep doris_be");
            eprintln!("  - Check BE status: curl http://127.0.0.1:8040/api/health");
        }
    }
}
