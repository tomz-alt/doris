mod config;
mod mysql;
mod http;
mod be;
mod query;
mod error;
mod metadata;
mod parser;
mod planner;
mod catalog;

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "doris_rust_fe=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Doris Rust FE Service with DataFusion");

    // Initialize catalog (loads TPC-H schema)
    let _catalog = metadata::catalog::catalog();
    info!("Catalog initialized with {} databases",
          _catalog.list_databases().len());

    // Load configuration
    let config = config::Config::load()?;
    info!("Configuration loaded: {:?}", config);

    // Create shared state with DataFusion
    info!("Initializing DataFusion query engine...");
    let query_executor = Arc::new(query::QueryExecutor::with_datafusion(
        config.query_queue_size,
        config.max_concurrent_queries,
    ).await);
    info!("DataFusion initialized successfully");

    let be_client_pool = Arc::new(be::BackendClientPool::new(config.backend_nodes.clone()));

    // Register BE-backed TPC-H tables (quick prototype)
    // This allows Rust FE to "see" TPC-H tables stored in BE
    info!("Registering BE-backed TPC-H tables (hardcoded schemas)...");
    match query_executor.register_tpch_be_tables(be_client_pool.clone(), "tpch_sf1").await {
        Ok(_) => info!("✓ BE-backed TPC-H tables registered successfully"),
        Err(e) => error!("Failed to register BE-backed tables: {}", e),
    }

    // Optionally override with CSV data for local testing (if TPCH_DATA_DIR is set)
    if let Ok(tpch_data_dir) = std::env::var("TPCH_DATA_DIR") {
        info!("TPC-H data directory specified: {}", tpch_data_dir);
        info!("This will override BE-backed tables with local CSV data");
        match query_executor.register_tpch_csv(&tpch_data_dir).await {
            Ok(_) => info!("TPC-H CSV data registered successfully"),
            Err(e) => error!("Failed to register TPC-H data: {}", e),
        }
    } else {
        info!("TPCH_DATA_DIR not set - using BE-backed tables");
        info!("To override with local CSV, set: export TPCH_DATA_DIR=/path/to/tpch/data");
    }

    // Start MySQL server
    let mysql_server = mysql::MysqlServer::new(
        config.mysql_port,
        query_executor.clone(),
        be_client_pool.clone(),
    );

    let mysql_handle = tokio::spawn(async move {
        if let Err(e) = mysql_server.serve().await {
            error!("MySQL server error: {}", e);
        }
    });

    // Start HTTP server for streaming load
    let http_server = http::HttpServer::new(
        config.http_port,
        query_executor.clone(),
        be_client_pool.clone(),
    );

    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_server.serve().await {
            error!("HTTP server error: {}", e);
        }
    });

    info!("========================================");
    info!("Doris Rust FE with DataFusion started successfully!");
    info!("MySQL server: localhost:{}", config.mysql_port);
    info!("HTTP server: localhost:{}", config.http_port);
    info!("TPC-H schema: loaded and ready");
    info!("Query engine: DataFusion (Arrow-based)");
    info!("========================================");
    info!("Connect with: mysql -h 127.0.0.1 -P {} -u root", config.mysql_port);
    info!("Then run: USE tpch; SHOW TABLES; SELECT * FROM lineitem LIMIT 10;");
    info!("========================================");

    // Wait for both servers
    tokio::select! {
        _ = mysql_handle => {
            error!("MySQL server terminated");
        }
        _ = http_handle => {
            error!("HTTP server terminated");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    info!("Shutting down Doris Rust FE");
    Ok(())
}
