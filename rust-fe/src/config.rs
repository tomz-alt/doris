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
        let mut config = match fs::read_to_string("fe_config.json") {
            Ok(content) => {
                let config: Config = serde_json::from_str(&content)?;
                config
            }
            Err(_) => {
                // Use default config
                let config = Config::default();
                // Save default config for reference
                let _ = fs::write(
                    "fe_config.json.example",
                    serde_json::to_string_pretty(&config)?
                );
                config
            }
        };

        // Override with environment variables if present
        if let Ok(port) = std::env::var("FE_QUERY_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                config.mysql_port = port_num;
            }
        }

        if let Ok(port) = std::env::var("FE_HTTP_PORT") {
            if let Ok(port_num) = port.parse::<u16>() {
                config.http_port = port_num;
            }
        }

        if let Ok(size) = std::env::var("QUERY_QUEUE_SIZE") {
            if let Ok(queue_size) = size.parse::<usize>() {
                config.query_queue_size = queue_size;
            }
        }

        if let Ok(max) = std::env::var("MAX_CONCURRENT_QUERIES") {
            if let Ok(max_queries) = max.parse::<usize>() {
                config.max_concurrent_queries = max_queries;
            }
        }

        // Parse BE_ADDR environment variable (format: "host:port" or "host:port:grpc_port")
        if let Ok(be_addr) = std::env::var("BE_ADDR") {
            let parts: Vec<&str> = be_addr.split(':').collect();
            if parts.len() >= 2 {
                if let Ok(port) = parts[1].parse::<u16>() {
                    let grpc_port = if parts.len() >= 3 {
                        parts[2].parse::<u16>().unwrap_or(9070)
                    } else {
                        9070  // Default gRPC port
                    };

                    config.backend_nodes = vec![BackendNode {
                        host: parts[0].to_string(),
                        port,
                        grpc_port,
                    }];
                }
            }
        }

        Ok(config)
    }
}
