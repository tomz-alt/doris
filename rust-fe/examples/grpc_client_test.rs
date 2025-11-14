/// gRPC Client Test - Demonstrates BE communication via gRPC
///
/// This example shows how the Rust FE communicates with Doris Backend using gRPC.
///
/// NOTE: This requires a running Doris BE instance. Without a BE, the connection
/// will fail, but it demonstrates the gRPC client functionality.
///
/// Usage: cargo run --example grpc_client_test

use uuid::Uuid;
use doris_rust_fe::be::client::BackendClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== gRPC Client Test ===\n");
    println!("Testing gRPC communication with Doris Backend\n");

    // Create a BE client
    let host = std::env::var("BE_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port = std::env::var("BE_PORT")
        .unwrap_or_else(|_| "9060".to_string())
        .parse::<u16>()
        .unwrap();
    let grpc_port = std::env::var("BE_GRPC_PORT")
        .unwrap_or_else(|_| "9070".to_string())
        .parse::<u16>()
        .unwrap();

    println!("BE Configuration:");
    println!("  Host: {}", host);
    println!("  Port: {}", port);
    println!("  gRPC Port: {}\n", grpc_port);

    let mut client = BackendClient::new(host.clone(), port, grpc_port);

    // Try to connect
    println!("Attempting to connect to BE at {}:{}...", host, grpc_port);

    match client.connect().await {
        Ok(()) => {
            println!("✓ Successfully connected to BE!\n");

            // Try to execute a simple query fragment
            let query_id = Uuid::new_v4();
            let fragment_id = 0;
            let test_query = "SELECT 1 as test";

            println!("Executing test fragment:");
            println!("  Query ID: {}", query_id);
            println!("  Fragment ID: {}", fragment_id);
            println!("  SQL: {}\n", test_query);

            match client.execute_fragment(query_id, fragment_id, test_query).await {
                Ok(result) => {
                    println!("✓ Fragment execution request sent!");
                    println!("  Status Code: {}", result.status_code);
                    println!("  Message: {}\n", result.message);

                    if result.status_code == 0 {
                        // Try to fetch results
                        println!("Fetching results...");

                        match client.fetch_data(query_id, fragment_id).await {
                            Ok(fetch_result) => {
                                println!("✓ Data fetched!");
                                println!("  Status Code: {}", fetch_result.status_code);
                                println!("  Data Size: {} bytes", fetch_result.data.len());
                                println!("  End of Stream: {}\n", fetch_result.eos);
                            }
                            Err(e) => {
                                println!("✗ Failed to fetch data: {}\n", e);
                            }
                        }

                        // Cancel the fragment
                        println!("Canceling fragment...");
                        match client.cancel_fragment(query_id, fragment_id).await {
                            Ok(cancel_result) => {
                                println!("✓ Fragment canceled!");
                                println!("  Status Code: {}\n", cancel_result.status_code);
                            }
                            Err(e) => {
                                println!("✗ Failed to cancel: {}\n", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Failed to execute fragment: {}\n", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to connect to BE: {}\n", e);
            println!("This is expected if no BE is running.");
            println!("\nTo test with a real BE:");
            println!("  1. Start a Doris BE instance");
            println!("  2. Set BE_HOST, BE_PORT, BE_GRPC_PORT environment variables");
            println!("  3. Run: BE_HOST=<host> BE_GRPC_PORT=<port> cargo run --example grpc_client_test\n");
        }
    }

    println!("=== Summary ===");
    println!("✓ Protobuf messages compiled successfully (via prost-build)");
    println!("✓ gRPC client generated (via tonic)");
    println!("✓ BE communication interface ready");
    println!("\nKey Components:");
    println!("- PBackendService gRPC client");
    println!("- ExecPlanFragment RPC (send fragments to BE)");
    println!("- FetchData RPC (retrieve results)");
    println!("- CancelPlanFragment RPC (cancel execution)");
    println!("\nNext Steps:");
    println!("- Integrate with fragment splitter (Phase 2)");
    println!("- Send multi-fragment distributed queries");
    println!("- Coordinate results from multiple BEs");

    Ok(())
}
