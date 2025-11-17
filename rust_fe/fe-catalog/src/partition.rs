// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Partition definition

use fe_common::{PartitionId, TabletId, Result};
use fe_common::types::{StorageMedium, PartitionType};
use serde::{Deserialize, Serialize};
use dashmap::DashMap;

/// Partition metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partition {
    /// Partition ID
    pub id: PartitionId,

    /// Partition name
    pub name: String,

    /// Create time (milliseconds)
    pub create_time: u64,

    /// Visible version
    pub visible_version: i64,

    /// Next version
    pub next_version: i64,

    /// Data size (bytes)
    pub data_size: i64,

    /// Row count
    pub row_count: i64,

    /// Replication number
    pub replication_num: i16,

    /// Storage medium
    pub storage_medium: StorageMedium,

    /// Tablets in this partition
    #[serde(skip)]
    pub tablets: DashMap<TabletId, i64>, // tablet_id -> index_id

    /// Is temporary partition
    pub is_temp: bool,
}

impl Partition {
    pub fn new(id: PartitionId, name: String, replication_num: i16) -> Self {
        Self {
            id,
            name,
            create_time: fe_common::utils::current_timestamp_ms(),
            visible_version: 1,
            next_version: 2,
            data_size: 0,
            row_count: 0,
            replication_num,
            storage_medium: StorageMedium::Hdd,
            tablets: DashMap::new(),
            is_temp: false,
        }
    }

    /// Add tablet to this partition
    pub fn add_tablet(&self, tablet_id: TabletId, index_id: i64) {
        self.tablets.insert(tablet_id, index_id);
    }

    /// Get tablet count
    pub fn tablet_count(&self) -> usize {
        self.tablets.len()
    }

    /// Update visible version
    pub fn update_visible_version(&mut self, version: i64) {
        self.visible_version = version;
        self.next_version = version + 1;
    }
}

/// Range partition info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangePartitionInfo {
    /// Partition columns
    pub partition_columns: Vec<String>,

    /// Range map (partition_id -> range)
    #[serde(skip)]
    pub ranges: DashMap<PartitionId, PartitionRange>,
}

/// Partition range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionRange {
    /// Lower bound (inclusive)
    pub lower: PartitionKey,

    /// Upper bound (exclusive)
    pub upper: PartitionKey,
}

/// Partition key value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum PartitionKey {
    MinValue,
    MaxValue,
    Value(Vec<u8>),
}
