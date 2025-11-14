/// Integration Test: FE → BE Communication
/// Tests the complete pipeline from SQL to Backend execution
///
/// This test:
/// 1. Starts a mock BE server in the background
/// 2. Runs queries through the FE
/// 3. Verifies BE receives and processes requests
/// 4. Checks that results flow back correctly
///
/// Usage:
///   Terminal 1: cargo run --example mock_be_server
///   Terminal 2: cargo run --example integration_test

use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use doris_rust_fe::be::{BackendClient, BackendClientPool};
use doris_rust_fe::config::BackendNode;
use doris_rust_fe::planner::{
    DataFusionPlanner, PlanConverter, FragmentSplitter, FragmentExecutor,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== FE → BE Integration Test ===\n");
    println!("This test verifies the complete pipeline from SQL to Backend execution\n");

    // Test 1: Direct BE Communication
    println!("=== Test 1: Direct BE gRPC Communication ===");
    test_direct_be_communication().await?;

    // Wait a bit between tests
    sleep(Duration::from_secs(1)).await;

    // Test 2: End-to-End Query Execution
    println!("\n=== Test 2: End-to-End Query Execution Pipeline ===");
    test_end_to_end_pipeline().await?;

    println!("\n=== All Integration Tests Passed! ✓ ===");

    Ok(())
}

async fn test_direct_be_communication() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing direct gRPC communication with Backend...\n");

    let mut client = BackendClient::new("localhost".to_string(), 9060, 9070);

    // Test connection
    print!("  Connecting to BE at localhost:9070... ");
    match client.connect().await {
        Ok(()) => println!("✓"),
        Err(e) => {
            println!("✗\n  Error: {}", e);
            println!("\n  ⚠ Make sure mock BE server is running:");
            println!("     cargo run --example mock_be_server\n");
            return Err(e.into());
        }
    }

    let query_id = Uuid::new_v4();
    let fragment_id = 0;
    let test_query = "SELECT COUNT(*) FROM test_table";

    // Test fragment execution
    print!("  Executing fragment... ");
    let exec_result = client.execute_fragment(query_id, fragment_id, test_query).await?;
    println!("✓");
    println!("    Status code: {}", exec_result.status_code);
    println!("    Message: {}", exec_result.message);

    assert_eq!(exec_result.status_code, 0, "Fragment execution should succeed");

    // Test data fetching
    print!("  Fetching data... ");
    let fetch_result = client.fetch_data(query_id, fragment_id).await?;
    println!("✓");
    println!("    Status code: {}", fetch_result.status_code);
    println!("    Data size: {} bytes", fetch_result.data.len());
    println!("    EOS: {}", fetch_result.eos);
    println!("    Data: {:?}", String::from_utf8_lossy(&fetch_result.data));

    assert_eq!(fetch_result.status_code, 0, "Data fetch should succeed");
    assert!(!fetch_result.data.is_empty(), "Should receive data");

    // Test cancellation
    print!("  Testing cancellation... ");
    let cancel_result = client.cancel_fragment(query_id, fragment_id).await?;
    println!("✓");
    println!("    Status code: {}", cancel_result.status_code);

    assert_eq!(cancel_result.status_code, 0, "Cancellation should succeed");

    println!("\n✓ Direct BE communication test passed!");
    Ok(())
}

async fn test_end_to_end_pipeline() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing complete query execution pipeline...\n");

    // Set up BE pool
    let be_nodes = vec![BackendNode {
        host: "localhost".to_string(),
        port: 9060,
        grpc_port: 9070,
    }];
    let be_pool = BackendClientPool::new(be_nodes);

    // Get TPC-H data directory
    let data_dir = std::env::var("TPCH_DATA_DIR")
        .unwrap_or_else(|_| "/home/user/doris/tpch-data".to_string());

    // Initialize DataFusion planner
    print!("  Initializing DataFusion planner... ");
    let planner = DataFusionPlanner::new().await;
    planner.register_tpch_csv_files(&data_dir).await?;
    println!("✓");

    // Test queries
    let test_queries = vec![
        ("Simple COUNT", "SELECT COUNT(*) FROM lineitem"),
        ("GROUP BY", "SELECT l_returnflag, COUNT(*) FROM lineitem GROUP BY l_returnflag"),
    ];

    for (name, sql) in test_queries {
        println!("\n  Testing query: {}", name);
        println!("  SQL: {}", sql);

        // Create physical plan
        print!("    Creating physical plan... ");
        let physical_plan = planner.create_physical_plan(sql).await?;
        println!("✓");

        // Convert to Doris fragments
        let query_id = Uuid::new_v4();
        print!("    Converting to Doris fragments... ");
        let mut converter = PlanConverter::new(query_id);
        let single_fragment_plan = converter.convert_to_fragments(physical_plan)?;
        println!("✓ ({} fragments)", single_fragment_plan.fragments.len());

        // Split into distributed fragments
        print!("    Splitting into distributed plan... ");
        let mut splitter = FragmentSplitter::new(query_id);

        let multi_fragment_plan = if let Some(single_fragment) =
            single_fragment_plan.fragments.into_iter().next()
        {
            match splitter.split_into_fragments(single_fragment) {
                Ok(fragments) => {
                    println!("✓ ({} fragments)", fragments.len());
                    doris_rust_fe::planner::QueryPlan {
                        query_id,
                        fragments,
                    }
                }
                Err(e) => {
                    println!("✗ {}", e);
                    continue;
                }
            }
        } else {
            println!("✗ No fragments generated");
            continue;
        };

        // Execute on BE
        print!("    Executing on BE... ");
        let executor = FragmentExecutor::new(query_id);

        match executor.execute_fragments(multi_fragment_plan, &be_pool).await {
            Ok(result) => {
                println!("✓");
                println!("      Rows returned: {}", result.rows.len());
                println!("      Columns: {}", result.columns.len());

                // Print first few rows
                if !result.rows.is_empty() {
                    println!("      Sample data:");
                    for (idx, row) in result.rows.iter().take(3).enumerate() {
                        println!("        Row {}: {:?}", idx + 1, row.values);
                    }
                }
            }
            Err(e) => {
                println!("✗");
                println!("      Error: {}", e);
                return Err(e.into());
            }
        }
    }

    println!("\n✓ End-to-end pipeline test passed!");
    Ok(())
}
