// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Scan Range Builder - Generate TScanRangeLocations from Metadata
//!
//! CLAUDE.MD Principle #2: Use Java FE behavior as specification
//! Reference: OlapScanNode.java lines 441-674 (addScanRangeLocations method)
//!
//! This module converts tablet metadata into Thrift structures needed by BE:
//! - TScanRangeLocations: Which tablets to scan and where they're located
//! - TPaloScanRange: Tablet ID, version, schema info
//! - TScanRangeLocation: Backend IP and port for each replica

use fe_catalog::metadata_fetcher::{ScanRangeMetadata, TabletWithBackend};
use fe_common::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Network address for BE communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TNetworkAddress {
    pub hostname: String,
    pub port: i32,
}

/// Single scan range location (one replica of a tablet)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TScanRangeLocation {
    /// Backend network address
    pub server: TNetworkAddress,

    /// Backend ID
    pub backend_id: i64,
}

/// Palo (Doris) specific scan range (one tablet)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TPaloScanRange {
    /// Database name (empty string for internal tables)
    pub db_name: String,

    /// Table schema hash (string representation)
    pub schema_hash: String,

    /// Tablet version (string representation)
    pub version: String,

    /// Version hash (deprecated, use empty string)
    pub version_hash: String,

    /// Tablet ID
    pub tablet_id: i64,

    /// Backend hosts for this tablet's replicas
    pub hosts: Vec<TNetworkAddress>,
}

/// Scan range with all its replica locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TScanRangeLocations {
    /// The scan range (tablet info)
    pub scan_range: TScanRange,

    /// All replica locations for this tablet
    pub locations: Vec<TScanRangeLocation>,
}

/// Scan range wrapper (can be extended for other scan types)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TScanRange {
    /// Palo/Doris OLAP scan range
    pub palo_scan_range: Option<TPaloScanRange>,
}

/// Builds TScanRangeLocations from tablet metadata
pub struct ScanRangeBuilder;

impl ScanRangeBuilder {
    /// Build scan range locations from metadata
    ///
    /// Reference: OlapScanNode.java:441-674 (addScanRangeLocations)
    ///
    /// For each tablet:
    /// 1. Create TPaloScanRange (tablet ID, version, schema)
    /// 2. For each replica:
    ///    - Create TNetworkAddress (backend IP, port)
    ///    - Create TScanRangeLocation (address + backend ID)
    /// 3. Combine into TScanRangeLocations
    pub fn build_from_metadata(metadata: &ScanRangeMetadata) -> Result<Vec<TScanRangeLocations>> {
        let mut scan_ranges = Vec::new();

        // Process each partition's tablets
        for (_partition_id, tablets) in &metadata.partitions {
            for tablet in tablets {
                scan_ranges.push(Self::build_single_tablet(tablet)?);
            }
        }

        Ok(scan_ranges)
    }

    /// Build scan range for a single tablet
    fn build_single_tablet(tablet: &TabletWithBackend) -> Result<TScanRangeLocations> {
        // Create backend network address (Java FE: TNetworkAddress(backend.getHost(), backend.getBePort()))
        let backend_addr = TNetworkAddress {
            hostname: tablet.backend_host.clone(),
            port: tablet.backend_port,
        };

        // Create TPaloScanRange (Java FE: lines 493-504)
        let palo_range = TPaloScanRange {
            db_name: String::new(), // Empty for internal tables
            schema_hash: "0".to_string(), // Default schema hash
            version: tablet.version.to_string(), // Tablet version
            version_hash: String::new(), // Deprecated field
            tablet_id: tablet.tablet_id,
            hosts: vec![backend_addr.clone()], // Backend addresses for replicas
        };

        // Create TScanRangeLocation (Java FE: lines 640-643)
        let scan_location = TScanRangeLocation {
            server: backend_addr,
            backend_id: tablet.backend_id,
        };

        // Create TScanRange wrapper
        let scan_range = TScanRange {
            palo_scan_range: Some(palo_range),
        };

        // Combine into TScanRangeLocations
        Ok(TScanRangeLocations {
            scan_range,
            locations: vec![scan_location],
        })
    }

    /// Build scan ranges from hardcoded metadata (for testing without BE)
    ///
    /// Assumes:
    /// - Single tablet at localhost:9060
    /// - Table ID determines tablet ID
    /// - Version 2
    pub fn build_hardcoded(table_id: i64) -> Vec<TScanRangeLocations> {
        let tablet_id = table_id + 2; // Convention from metadata_fetcher

        let backend_addr = TNetworkAddress {
            hostname: "127.0.0.1".to_string(),
            port: 9060,
        };

        let palo_range = TPaloScanRange {
            db_name: String::new(),
            schema_hash: "0".to_string(),
            version: "2".to_string(),
            version_hash: String::new(),
            tablet_id,
            hosts: vec![backend_addr.clone()],
        };

        let scan_location = TScanRangeLocation {
            server: backend_addr,
            backend_id: 10001,
        };

        let scan_range = TScanRange {
            palo_scan_range: Some(palo_range),
        };

        vec![TScanRangeLocations {
            scan_range,
            locations: vec![scan_location],
        }]
    }

    /// Convert to JSON for debugging/comparison with Java FE
    pub fn to_json(scan_ranges: &[TScanRangeLocations]) -> Result<String> {
        serde_json::to_string_pretty(scan_ranges)
            .map_err(|e| fe_common::DorisError::InternalError(format!("JSON serialization failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardcoded_scan_ranges() {
        let ranges = ScanRangeBuilder::build_hardcoded(10001);

        assert_eq!(ranges.len(), 1);

        let range = &ranges[0];
        let palo_range = range.scan_range.palo_scan_range.as_ref().unwrap();

        assert_eq!(palo_range.tablet_id, 10003);
        assert_eq!(palo_range.version, "2");
        assert_eq!(palo_range.hosts.len(), 1);
        assert_eq!(palo_range.hosts[0].hostname, "127.0.0.1");
        assert_eq!(palo_range.hosts[0].port, 9060);

        assert_eq!(range.locations.len(), 1);
        assert_eq!(range.locations[0].backend_id, 10001);
    }

    #[test]
    fn test_build_from_metadata() {
        // Create test metadata
        let mut partitions = HashMap::new();
        let tablet = TabletWithBackend {
            tablet_id: 10003,
            version: 2,
            backend_host: "127.0.0.1".to_string(),
            backend_port: 9060,
            backend_id: 10001,
        };
        partitions.insert(10001, vec![tablet]);

        let metadata = ScanRangeMetadata { partitions };

        // Build scan ranges
        let ranges = ScanRangeBuilder::build_from_metadata(&metadata).unwrap();

        assert_eq!(ranges.len(), 1);
        let range = &ranges[0];
        let palo_range = range.scan_range.palo_scan_range.as_ref().unwrap();

        assert_eq!(palo_range.tablet_id, 10003);
        assert_eq!(palo_range.version, "2");
    }

    #[test]
    fn test_json_serialization() {
        let ranges = ScanRangeBuilder::build_hardcoded(10001);
        let json = ScanRangeBuilder::to_json(&ranges).unwrap();

        // Verify JSON contains key fields
        assert!(json.contains("tablet_id"));
        assert!(json.contains("10003"));
        assert!(json.contains("127.0.0.1"));
        assert!(json.contains("9060"));

        println!("Scan range JSON:\n{}", json);
    }

    #[test]
    fn test_multiple_tablets() {
        let mut partitions = HashMap::new();

        // Partition 1 with 2 tablets
        let tablets_p1 = vec![
            TabletWithBackend {
                tablet_id: 10003,
                version: 2,
                backend_host: "192.168.1.1".to_string(),
                backend_port: 9060,
                backend_id: 10001,
            },
            TabletWithBackend {
                tablet_id: 10004,
                version: 2,
                backend_host: "192.168.1.2".to_string(),
                backend_port: 9060,
                backend_id: 10002,
            },
        ];
        partitions.insert(10001, tablets_p1);

        // Partition 2 with 1 tablet
        let tablets_p2 = vec![
            TabletWithBackend {
                tablet_id: 10005,
                version: 3,
                backend_host: "192.168.1.3".to_string(),
                backend_port: 9060,
                backend_id: 10003,
            },
        ];
        partitions.insert(10002, tablets_p2);

        let metadata = ScanRangeMetadata { partitions };
        let ranges = ScanRangeBuilder::build_from_metadata(&metadata).unwrap();

        // Should have 3 total scan ranges (2 from partition 1, 1 from partition 2)
        assert_eq!(ranges.len(), 3);

        // Verify each tablet has correct info
        let tablet_ids: Vec<i64> = ranges.iter()
            .map(|r| r.scan_range.palo_scan_range.as_ref().unwrap().tablet_id)
            .collect();

        assert!(tablet_ids.contains(&10003));
        assert!(tablet_ids.contains(&10004));
        assert!(tablet_ids.contains(&10005));
    }
}
