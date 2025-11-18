// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Test connection to C++ Backend via Cloudflare tunnel
//!
//! Run with: cargo run --example test_tunnel_connection

use fe_backend_client::BackendClient;

#[tokio::main]
async fn main() {
    println!("🔍 Testing connection to C++ Backend via Cloudflare tunnel...\n");
    println!("Connecting to localhost:18060 (socat proxy)...\n");

    // Try to connect via the socat proxy
    match BackendClient::new("127.0.0.1", 18060).await {
        Ok(client) => {
            println!("✅ Successfully connected to BE at {}", client.address());
            println!("📡 gRPC client is ready");
            println!("\n🎉 Connection test PASSED!");
            println!("\nThe Rust FE can now communicate with the C++ BE!");
        }
        Err(e) => {
            eprintln!("❌ Failed to connect to BE: {}", e);
            eprintln!("\n💡 Troubleshooting:");
            eprintln!("  - Is socat running? ps aux | grep socat");
            eprintln!("  - Is the Cloudflare tunnel active?");
            eprintln!("  - Check: netstat -tlnp | grep 18060");
            std::process::exit(1);
        }
    }
}
