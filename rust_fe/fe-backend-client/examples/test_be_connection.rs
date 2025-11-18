// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Test connection to real C++ Backend
//!
//! Run with: cargo run --example test_be_connection

use fe_backend_client::BackendClient;

#[tokio::main]
async fn main() {
    println!("🔍 Testing connection to C++ Backend at localhost:8060...\n");

    // Try to connect to the running BE
    match BackendClient::new("127.0.0.1", 8060).await {
        Ok(client) => {
            println!("✅ Successfully connected to BE at {}", client.address());
            println!("📡 gRPC client is ready");
            println!("\nNext steps:");
            println!("  1. Query metadata to find available tables");
            println!("  2. Execute a test query");
            println!("  3. Fetch results");
        }
        Err(e) => {
            eprintln!("❌ Failed to connect to BE: {}", e);
            eprintln!("\n💡 Troubleshooting:");
            eprintln!("  - Is the C++ BE running on port 8060?");
            eprintln!("  - Check: netstat -tlnp | grep 8060");
            eprintln!("  - Verify BE logs for errors");
            std::process::exit(1);
        }
    }
}
