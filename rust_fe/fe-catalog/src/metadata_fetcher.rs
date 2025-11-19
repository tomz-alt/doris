// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Metadata Fetcher - Query Java FE for Tablet Locations
//!
//! CLAUDE.MD Principle #2: Use Java FE behavior as specification
//! This module queries Java FE's metadata via MySQL protocol to get:
//! - Tablet IDs for a given table
//! - Backend addresses where tablets are located
//! - Version information for each tablet
//!
//! Reference: Java FE information_schema tables (tablets, backends)

use fe_common::{Result, DorisError};
use std::collections::HashMap;

/// Tablet location information from Java FE
#[derive(Debug, Clone)]
pub struct TabletLocation {
    /// Tablet ID
    pub tablet_id: i64,

    /// Backend ID where this tablet replica resides
    pub backend_id: i64,

    /// Tablet version (for versioned reads)
    pub version: i64,

    /// Partition ID this tablet belongs to
    pub partition_id: i64,
}

/// Backend server information from Java FE
#[derive(Debug, Clone)]
pub struct BackendInfo {
    /// Backend ID
    pub backend_id: i64,

    /// Backend host (IP or hostname)
    pub host: String,

    /// Backend RPC port (for Thrift communication)
    pub be_port: i32,

    /// Backend heartbeat port
    pub heartbeat_port: i32,

    /// Is backend alive
    pub is_alive: bool,
}

/// Metadata fetcher for querying Java FE
pub struct MetadataFetcher {
    /// MySQL connection pool (to Java FE port 9030)
    #[cfg(feature = "mysql-integration")]
    pool: Option<mysql::Pool>,

    /// FE address (for non-MySQL methods)
    fe_host: String,
    fe_mysql_port: u16,
    fe_http_port: u16,
}

impl MetadataFetcher {
    /// Create new metadata fetcher
    pub fn new(fe_host: String, fe_mysql_port: u16, fe_http_port: u16) -> Self {
        Self {
            #[cfg(feature = "mysql-integration")]
            pool: None,
            fe_host,
            fe_mysql_port,
            fe_http_port,
        }
    }

    /// Connect to Java FE via MySQL protocol
    #[cfg(feature = "mysql-integration")]
    pub fn connect(&mut self) -> Result<()> {
        let url = format!("mysql://root:@{}:{}/information_schema",
            self.fe_host, self.fe_mysql_port);

        self.pool = Some(mysql::Pool::new(url.as_str())
            .map_err(|e| DorisError::NetworkError(format!("Failed to connect to FE: {}", e)))?);

        Ok(())
    }

    /// Get tablet locations for a table
    ///
    /// Queries Java FE's information_schema.tablets table
    /// Reference: fe/fe-core/src/main/java/org/apache/doris/catalog/InformationSchemaDataSource.java
    #[cfg(feature = "mysql-integration")]
    pub fn get_tablet_locations(&self, db_name: &str, table_name: &str) -> Result<Vec<TabletLocation>> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| DorisError::InternalError("Not connected to FE".to_string()))?;

        let mut conn = pool.get_conn()
            .map_err(|e| DorisError::NetworkError(format!("Failed to get connection: {}", e)))?;

        // Query tablets table
        // Reference SQL: SELECT TABLET_ID, BACKEND_ID, VERSION, PARTITION_ID
        //                FROM information_schema.tablets
        //                WHERE TABLE_NAME = ?
        let query = format!(
            "SELECT TABLET_ID, BACKEND_ID, VERSION, PARTITION_ID \
             FROM information_schema.tablets \
             WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}'",
            db_name, table_name
        );

        let result: Vec<(i64, i64, i64, i64)> = conn.query(query)
            .map_err(|e| DorisError::InternalError(format!("Query failed: {}", e)))?;

        let tablets = result.into_iter().map(|(tablet_id, backend_id, version, partition_id)| {
            TabletLocation {
                tablet_id,
                backend_id,
                version,
                partition_id,
            }
        }).collect();

        Ok(tablets)
    }

    /// Get backend information
    ///
    /// Queries Java FE's information_schema.backends table
    #[cfg(feature = "mysql-integration")]
    pub fn get_backends(&self) -> Result<Vec<BackendInfo>> {
        let pool = self.pool.as_ref()
            .ok_or_else(|| DorisError::InternalError("Not connected to FE".to_string()))?;

        let mut conn = pool.get_conn()
            .map_err(|e| DorisError::NetworkError(format!("Failed to get connection: {}", e)))?;

        // Query backends table
        let query = "SELECT BACKEND_ID, HOST, HEARTBEAT_PORT, BE_PORT, ALIVE \
                     FROM information_schema.backends";

        let result: Vec<(i64, String, i32, i32, bool)> = conn.query(query)
            .map_err(|e| DorisError::InternalError(format!("Query failed: {}", e)))?;

        let backends = result.into_iter().map(|(backend_id, host, heartbeat_port, be_port, is_alive)| {
            BackendInfo {
                backend_id,
                host,
                be_port,
                heartbeat_port,
                is_alive,
            }
        }).collect();

        Ok(backends)
    }

    /// Get complete scan range information (tablets + backends)
    ///
    /// This is what Rust FE needs to build TScanRangeLocations for query execution
    #[cfg(feature = "mysql-integration")]
    pub fn get_scan_range_metadata(&self, db_name: &str, table_name: &str)
        -> Result<ScanRangeMetadata>
    {
        // Get all tablets for the table
        let tablets = self.get_tablet_locations(db_name, table_name)?;

        // Get all backends
        let backends = self.get_backends()?;

        // Build backend_id -> BackendInfo map
        let backend_map: HashMap<i64, BackendInfo> = backends.into_iter()
            .map(|b| (b.backend_id, b))
            .collect();

        // Group tablets by partition
        let mut partitions: HashMap<i64, Vec<TabletWithBackend>> = HashMap::new();

        for tablet in tablets {
            let backend = backend_map.get(&tablet.backend_id)
                .ok_or_else(|| DorisError::InternalError(
                    format!("Backend {} not found for tablet {}", tablet.backend_id, tablet.tablet_id)
                ))?;

            let tablet_with_backend = TabletWithBackend {
                tablet_id: tablet.tablet_id,
                version: tablet.version,
                backend_host: backend.host.clone(),
                backend_port: backend.be_port,
                backend_id: backend.backend_id,
            };

            partitions.entry(tablet.partition_id)
                .or_insert_with(Vec::new)
                .push(tablet_with_backend);
        }

        Ok(ScanRangeMetadata {
            partitions,
        })
    }

    /// Hardcoded metadata for testing (when FE is not running)
    ///
    /// Returns fake tablet locations assuming:
    /// - 1 tablet (ID: 10003)
    /// - 1 backend (127.0.0.1:9060)
    /// - 1 partition (ID: 10001)
    pub fn get_hardcoded_metadata(table_id: i64) -> ScanRangeMetadata {
        let mut partitions = HashMap::new();

        // Single tablet for testing
        let tablet = TabletWithBackend {
            tablet_id: table_id + 2, // Assume tablet_id = table_id + 2 (convention in minimal_mysql_client)
            version: 2,
            backend_host: "127.0.0.1".to_string(),
            backend_port: 9060,
            backend_id: 10001,
        };

        partitions.insert(table_id, vec![tablet]);

        ScanRangeMetadata { partitions }
    }
}

/// Complete scan range metadata (tablets grouped by partition + backend info)
#[derive(Debug, Clone)]
pub struct ScanRangeMetadata {
    /// Tablets grouped by partition_id
    pub partitions: HashMap<i64, Vec<TabletWithBackend>>,
}

/// Tablet with associated backend information
#[derive(Debug, Clone)]
pub struct TabletWithBackend {
    pub tablet_id: i64,
    pub version: i64,
    pub backend_host: String,
    pub backend_port: i32,
    pub backend_id: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardcoded_metadata() {
        let metadata = MetadataFetcher::get_hardcoded_metadata(10001);

        assert_eq!(metadata.partitions.len(), 1);
        let tablets = metadata.partitions.get(&10001).unwrap();
        assert_eq!(tablets.len(), 1);
        assert_eq!(tablets[0].tablet_id, 10003);
        assert_eq!(tablets[0].backend_host, "127.0.0.1");
        assert_eq!(tablets[0].backend_port, 9060);
    }

    #[test]
    #[cfg(feature = "mysql-integration")]
    fn test_connect_to_fe() {
        // This test only runs if Java FE is running on localhost:9030
        let mut fetcher = MetadataFetcher::new("127.0.0.1".to_string(), 9030, 8030);

        match fetcher.connect() {
            Ok(_) => {
                println!("✅ Connected to Java FE");

                // Try to fetch backends
                match fetcher.get_backends() {
                    Ok(backends) => {
                        println!("Found {} backends", backends.len());
                        for backend in backends {
                            println!("  Backend {}: {}:{} (alive: {})",
                                backend.backend_id, backend.host, backend.be_port, backend.is_alive);
                        }
                    }
                    Err(e) => println!("⚠️  Could not fetch backends: {}", e),
                }
            }
            Err(e) => println!("⚠️  FE not running: {}", e),
        }
    }
}
