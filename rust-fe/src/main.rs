mod config;
mod mysql;
mod http;
mod be;
mod query;
mod error;

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
                .unwrap_or_else(|_| "doris_rust_fe=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Doris Rust FE Service");

    // Load configuration
    let config = config::Config::load()?;
    info!("Configuration loaded: {:?}", config);

    // Create shared state
    let query_executor = Arc::new(query::QueryExecutor::new(
        config.query_queue_size,
        config.max_concurrent_queries,
    ));

    let be_client_pool = Arc::new(be::BackendClientPool::new(config.backend_nodes.clone()));

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

    info!("Doris Rust FE started successfully");
    info!("MySQL server listening on port {}", config.mysql_port);
    info!("HTTP server listening on port {}", config.http_port);

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
