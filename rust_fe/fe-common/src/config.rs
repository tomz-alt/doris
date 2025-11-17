// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Configuration management for Doris FE

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Frontend metadata directory
    pub meta_dir: PathBuf,

    /// HTTP server port
    pub http_port: u16,

    /// RPC server port
    pub rpc_port: u16,

    /// MySQL server port
    pub query_port: u16,

    /// Edit log port
    pub edit_log_port: u16,

    /// Frontend role (FOLLOWER, OBSERVER, UNKNOWN)
    pub role: String,

    /// Cluster ID
    pub cluster_id: Option<i32>,

    /// Node name
    pub node_name: String,

    /// Priority networks (CIDR notation)
    pub priority_networks: Option<String>,

    /// Master nodes
    pub helper_nodes: Vec<String>,

    /// Enable leader election
    pub enable_election: bool,

    /// Metadata sync interval (ms)
    pub metadata_sync_interval_ms: u64,

    /// Query timeout (seconds)
    pub query_timeout_s: u64,

    /// Max connections
    pub max_connections: usize,

    /// Thread pool size
    pub thread_pool_size: usize,

    /// Enable query cache
    pub enable_query_cache: bool,

    /// Query cache size (bytes)
    pub query_cache_size: usize,

    /// Enable statistics auto-analyze
    pub enable_auto_analyze: bool,

    /// Log level
    pub log_level: String,

    /// Log directory
    pub log_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            meta_dir: PathBuf::from("./doris-meta"),
            http_port: 8030,
            rpc_port: 9020,
            query_port: 9030,
            edit_log_port: 9010,
            role: "FOLLOWER".to_string(),
            cluster_id: None,
            node_name: "localhost".to_string(),
            priority_networks: None,
            helper_nodes: Vec::new(),
            enable_election: true,
            metadata_sync_interval_ms: 60000,
            query_timeout_s: 300,
            max_connections: 4096,
            thread_pool_size: num_cpus::get(),
            enable_query_cache: false,
            query_cache_size: 1024 * 1024 * 1024, // 1GB
            enable_auto_analyze: true,
            log_level: "info".to_string(),
            log_dir: PathBuf::from("./log"),
        }
    }
}

impl Config {
    pub fn from_file(path: &PathBuf) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| crate::error::DorisError::InvalidArgument(e.to_string()))?;
        Ok(config)
    }

    pub fn validate(&self) -> crate::Result<()> {
        if self.http_port == 0 {
            return Err(crate::error::DorisError::InvalidArgument(
                "http_port cannot be 0".to_string(),
            ));
        }
        if self.rpc_port == 0 {
            return Err(crate::error::DorisError::InvalidArgument(
                "rpc_port cannot be 0".to_string(),
            ));
        }
        if self.query_port == 0 {
            return Err(crate::error::DorisError::InvalidArgument(
                "query_port cannot be 0".to_string(),
            ));
        }
        Ok(())
    }
}
