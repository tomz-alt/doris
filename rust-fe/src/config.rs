use serde::{Deserialize, Serialize};
use std::fs;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub mysql_port: u16,
    pub http_port: u16,
    pub query_queue_size: usize,
    pub max_concurrent_queries: usize,
    pub backend_nodes: Vec<BackendNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendNode {
    pub host: String,
    pub port: u16,
    pub grpc_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mysql_port: 9030,
            http_port: 8030,
            query_queue_size: 1024,
            max_concurrent_queries: 100,
            backend_nodes: vec![
                BackendNode {
                    host: "127.0.0.1".to_string(),
                    port: 9060,
                    grpc_port: 9070,
                }
            ],
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // Try to load from config file, otherwise use defaults
        match fs::read_to_string("fe_config.json") {
            Ok(content) => {
                let config: Config = serde_json::from_str(&content)?;
                Ok(config)
            }
            Err(_) => {
                // Use default config
                let config = Config::default();
                // Save default config for reference
                let _ = fs::write(
                    "fe_config.json.example",
                    serde_json::to_string_pretty(&config)?
                );
                Ok(config)
            }
        }
    }
}
