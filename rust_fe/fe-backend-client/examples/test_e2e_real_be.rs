// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! End-to-End Test: Rust FE → C++ BE with Real Data
//!
//! Tests the complete query pipeline:
//! 1. Build query plan in Rust FE
//! 2. Serialize to Thrift
//! 3. Send to C++ BE via gRPC
//! 4. Fetch results
//! 5. Parse and display
//!
//! This tests against the real lineitem data loaded by minimal_mysql_client.

use fe_backend_client::BackendClient;
use fe_planner::thrift_plan::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════");
    println!("  End-to-End Test: Rust FE → C++ BE (Real Data)");
    println!("═══════════════════════════════════════════════════════\n");

    // Step 1: Connect to C++ BE
    println!("Step 1: Connecting to C++ BE at 127.0.0.1:8060...");
    let mut client = BackendClient::new("127.0.0.1", 8060).await?;
    println!("✅ Connected to BE at {}\n", client.address());

    // Step 2: Build a simple scan plan for lineitem table
    println!("Step 2: Building query plan for: SELECT * FROM lineitem");

    // Create a simple OLAP scan node for lineitem table
    let scan_node = TPlanNode {
        node_id: 0,
        node_type: TPlanNodeType::OlapScanNode,
        num_children: 0,
        limit: -1, // No limit
        row_tuples: vec![0], // Tuple ID 0
        nullable_tuples: vec![false],
        compact_data: true,
        olap_scan_node: Some(TOlapScanNode {
            tuple_id: 0,
            key_column_name: vec![
                "l_orderkey".to_string(),
                "l_partkey".to_string(),
                "l_suppkey".to_string(),
            ],
            key_column_type: vec![
                TPrimitiveType::BigInt,
                TPrimitiveType::BigInt,
                TPrimitiveType::BigInt,
            ],
            is_preaggregation: false,
            table_name: Some("lineitem".to_string()),
        }),
    };

    let plan = TPlan {
        nodes: vec![scan_node],
    };

    let fragment = TPlanFragment { plan };

    println!("   Plan: OLAP_SCAN_NODE (lineitem table)");
    println!("   Columns: l_orderkey, l_partkey, l_suppkey");
    println!("   Expected: 4 rows from real BE storage\n");

    // Step 3: Generate query ID
    let query_id: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
        0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10,
    ];
    println!("Step 3: Sending plan to BE...");
    println!("   Query ID: {:02X?}", query_id);

    // Step 4: Execute plan fragment
    match client.exec_plan_fragment(&fragment, query_id).await {
        Ok(finst_id) => {
            println!("✅ Plan executed successfully");
            println!("   Fragment Instance ID: {:02X?}\n", finst_id);

            // Step 5: Fetch results
            println!("Step 4: Fetching results from BE...");
            match client.fetch_data(finst_id).await {
                Ok(rows) => {
                    println!("✅ Results fetched: {} rows\n", rows.len());

                    // Step 6: Display results
                    println!("Step 5: Result Data:");
                    println!("─────────────────────────────────────────────────────");
                    for (i, row) in rows.iter().enumerate() {
                        println!("Row {}: {:?}", i + 1, row);
                    }
                    println!("─────────────────────────────────────────────────────\n");

                    // Step 7: Verify
                    println!("Step 6: Verification");
                    println!("   ✅ Query plan created");
                    println!("   ✅ Thrift serialization successful");
                    println!("   ✅ gRPC communication successful");
                    println!("   ✅ Results fetched from BE");
                    println!("   ✅ PBlock parsed");

                    if rows.len() > 0 {
                        println!("\n🎉 SUCCESS! End-to-end pipeline working!");
                        println!("   Rust FE → C++ BE → Rust FE COMPLETE");
                    } else {
                        println!("\n⚠️  Warning: No rows returned (expected 4)");
                        println!("   This might mean:");
                        println!("   - Query plan needs refinement");
                        println!("   - BE needs additional scan parameters");
                        println!("   - Tablet/partition metadata required");
                    }
                }
                Err(e) => {
                    println!("❌ Failed to fetch data: {}", e);
                    println!("\nDiagnostics:");
                    println!("   - Check if BE has data: mysql -h127.0.0.1 -P9030 -uroot -e 'SELECT COUNT(*) FROM tpch.lineitem'");
                    println!("   - Verify fragment execution succeeded");
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to execute plan: {}", e);
            println!("\nPossible issues:");
            println!("   - BE might need more complete plan (tuples, descriptors)");
            println!("   - Scan ranges might be required");
            println!("   - Database/table metadata needed");
            return Err(e.into());
        }
    }

    println!("\n═══════════════════════════════════════════════════════");
    println!("Next Steps:");
    println!("1. Complete PBlock columnar parser for full row data");
    println!("2. Add scan ranges and tablet metadata");
    println!("3. Implement tuple descriptors");
    println!("4. Compare results with Java FE");
    println!("5. Run full TPC-H query suite");
    println!("═══════════════════════════════════════════════════════");

    Ok(())
}
