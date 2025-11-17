/// Experimental TPCH lineitem pipeline fragment execution using the real
/// Doris internal_service.proto and Thrift `TPipelineFragmentParamsList`.
///
/// This sends a single-fragment OLAP scan over `tpch.lineitem` using the
/// Rust FE's catalog-driven Thrift encoder, without going through the
/// normal `execute_sql` path.
///
/// Requirements:
/// - Build with `--features real_be_proto` (uses internal_service.proto).
/// - Do NOT set `SKIP_PROTO=1` (gRPC client must be available).
/// - A Doris BE reachable via `fe_config.json` or `BE_ADDR` env.
///
/// Usage:
///   cargo run --features real_be_proto --example tpch_lineitem_fragment

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use doris_rust_fe::{Config};
    use doris_rust_fe::be::BackendClientPool;

    // Initialize basic logging
    tracing_subscriber::fmt::init();

    println!("=== TPCH lineitem pipeline fragment (experimental) ===\n");

    // Load FE config to discover backend nodes (host/ports).
    let config = Config::load()?;
    println!("Using {} backend node(s) from config\n", config.backend_nodes.len());

    let pool = BackendClientPool::new(config.backend_nodes.clone());

    println!("Sending TPCH lineitem scan fragment via Thrift → gRPC...");
    match pool.execute_tpch_lineitem_scan_fragments().await {
        Ok(_) => {
            println!("✓ Pipeline fragment request completed.");
            println!("  Check Doris BE logs for fragment planning/execution details.");
        }
        Err(e) => {
            eprintln!("✗ Failed to execute pipeline fragments: {e}");
            eprintln!("  Ensure Doris BE is running and reachable from this FE.");
        }
    }

    Ok(())
}

// Fallback main when built without the real BE proto or with SKIP_PROTO.
#[cfg(not(all(not(skip_proto), feature = "real_be_proto")))]
fn main() {
    eprintln!("This example requires the real Doris BE internal_service.proto and gRPC client:");
    eprintln!("  cargo run --features real_be_proto --example tpch_lineitem_fragment");
    eprintln!("and must NOT be run with SKIP_PROTO=1.");
}

