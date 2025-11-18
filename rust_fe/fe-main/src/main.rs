// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Doris Frontend Main Entry Point

use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};
use fe_catalog::Catalog;
use fe_mysql_protocol::MysqlServer;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "conf/fe.conf")]
    config: PathBuf,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// Daemon mode
    #[arg(short, long, default_value_t = false)]
    daemon: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args.log_level)?;

    info!("Starting Apache Doris Frontend (Rust)");
    info!("Version: {}", fe_common::version::VERSION);
    info!("Git Commit: {}", fe_common::version::GIT_COMMIT);
    info!("Build Time: {}", fe_common::version::BUILD_TIME);

    // Load configuration
    let config = load_config(&args.config)?;
    info!("Configuration loaded from: {:?}", args.config);
    info!("HTTP Port: {}", config.http_port);
    info!("RPC Port: {}", config.rpc_port);
    info!("Query Port: {}", config.query_port);
    info!("Meta Directory: {:?}", config.meta_dir);

    // Validate configuration
    config.validate()?;

    // Initialize catalog
    let catalog = Arc::new(Catalog::new());
    info!("Catalog initialized");

    // Initialize with default database and TPC-H schema for testing
    if let Err(e) = init_default_schema(&catalog) {
        warn!("Failed to initialize default schema: {:?}", e);
    }

    // Start MySQL protocol server
    let mysql_server = MysqlServer::new(catalog.clone(), config.query_port);
    info!("Starting MySQL protocol server on port {}", config.query_port);

    tokio::spawn(async move {
        if let Err(e) = mysql_server.start().await {
            tracing::error!("MySQL server error: {:?}", e);
        }
    });

    // TODO: Start other services
    // - HTTP server
    // - RPC server
    // - Metadata synchronization
    // - Background threads (replica checker, tablet scheduler, etc.)

    info!("Doris Frontend is ready to serve");

    // Wait for shutdown signal
    wait_for_shutdown().await;

    info!("Shutting down Doris Frontend");

    // TODO: Graceful shutdown
    // - Stop accepting new requests
    // - Wait for in-flight requests to complete
    // - Flush metadata
    // - Close connections

    info!("Doris Frontend shut down complete");

    Ok(())
}

/// Initialize logging
fn init_logging(log_level: &str) -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_ansi(true)
        .init();

    Ok(())
}

/// Load configuration from file
fn load_config(config_path: &PathBuf) -> anyhow::Result<fe_common::Config> {
    if config_path.exists() {
        fe_common::Config::from_file(config_path)
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))
    } else {
        tracing::warn!("Config file not found: {:?}, using defaults", config_path);
        Ok(fe_common::Config::default())
    }
}

/// Initialize default schema for testing
fn init_default_schema(catalog: &Catalog) -> anyhow::Result<()> {
    // Create 'default' database (cluster 'default_cluster')
    catalog.create_database("default".to_string(), "default_cluster".to_string())?;
    info!("Created default database");

    // Create 'tpch' database for TPC-H testing
    catalog.create_database("tpch".to_string(), "default_cluster".to_string())?;
    info!("Created tpch database");

    Ok(())
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
async fn wait_for_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        }
        _ = terminate => {
            info!("Received SIGTERM signal");
        }
    }
}
