// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Table definitions

use fe_common::{TableId, DbId, PartitionId, Result};
use fe_common::types::{TableType, KeysType, PartitionType, StorageMedium};
use crate::column::Column;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;

/// Base table trait
pub trait Table: Send + Sync {
    fn id(&self) -> TableId;
    fn name(&self) -> &str;
    fn table_type(&self) -> TableType;
    fn db_id(&self) -> DbId;
    fn create_time(&self) -> u64;
    fn columns(&self) -> &[Column];
}

/// OLAP table (native Doris table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlapTable {
    /// Table ID
    pub id: TableId,

    /// Table name
    pub name: String,

    /// Database ID
    pub db_id: DbId,

    /// Create time (milliseconds)
    pub create_time: u64,

    /// Last check time
    pub last_check_time: u64,

    /// Keys type (DUP_KEYS, UNIQUE_KEYS, AGG_KEYS)
    pub keys_type: KeysType,

    /// Partition type
    pub partition_type: PartitionType,

    /// Columns
    pub columns: Vec<Column>,

    /// Indexes (index_id -> index)
    #[serde(skip)]
    pub indexes: Arc<DashMap<i64, Arc<RwLock<super::index::Index>>>>,

    /// Partitions (partition_id -> partition)
    #[serde(skip)]
    pub partitions: Arc<DashMap<PartitionId, Arc<RwLock<super::partition::Partition>>>>,

    /// Default replication number
    pub replication_num: i16,

    /// Storage medium
    pub storage_medium: StorageMedium,

    /// Bloom filter columns
    pub bloom_filter_columns: Vec<String>,

    /// Table properties
    #[serde(skip)]
    pub properties: DashMap<String, String>,

    /// Comment
    pub comment: Option<String>,
}

impl OlapTable {
    pub fn new(
        id: TableId,
        name: String,
        db_id: DbId,
        keys_type: KeysType,
        columns: Vec<Column>,
    ) -> Self {
        Self {
            id,
            name,
            db_id,
            create_time: fe_common::utils::current_timestamp_ms(),
            last_check_time: 0,
            keys_type,
            partition_type: PartitionType::Unpartitioned,
            columns,
            indexes: Arc::new(DashMap::new()),
            partitions: Arc::new(DashMap::new()),
            replication_num: fe_common::constants::DEFAULT_REPLICATION_NUM,
            storage_medium: StorageMedium::Hdd,
            bloom_filter_columns: Vec::new(),
            properties: DashMap::new(),
            comment: None,
        }
    }

    /// Get key columns
    pub fn key_columns(&self) -> Vec<&Column> {
        self.columns
            .iter()
            .filter(|col| col.is_key)
            .collect()
    }

    /// Get value columns
    pub fn value_columns(&self) -> Vec<&Column> {
        self.columns
            .iter()
            .filter(|col| !col.is_key)
            .collect()
    }

    /// Get column by name
    pub fn get_column(&self, name: &str) -> Option<&Column> {
        self.columns.iter().find(|col| col.name == name)
    }

    /// Add partition
    pub fn add_partition(&self, partition: super::partition::Partition) -> Result<()> {
        let partition_id = partition.id;
        if self.partitions.contains_key(&partition_id) {
            return Err(fe_common::DorisError::AlreadyExists(
                format!("Partition {} already exists", partition.name)
            ));
        }
        self.partitions.insert(partition_id, Arc::new(RwLock::new(partition)));
        Ok(())
    }

    /// Remove partition
    pub fn remove_partition(&self, partition_id: PartitionId) -> Result<()> {
        self.partitions.remove(&partition_id)
            .ok_or_else(|| fe_common::DorisError::NotFound(
                format!("Partition {} not found", partition_id)
            ))?;
        Ok(())
    }

    /// Get partition count
    pub fn partition_count(&self) -> usize {
        self.partitions.len()
    }
}

impl Table for OlapTable {
    fn id(&self) -> TableId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn table_type(&self) -> TableType {
        TableType::Olap
    }

    fn db_id(&self) -> DbId {
        self.db_id
    }

    fn create_time(&self) -> u64 {
        self.create_time
    }

    fn columns(&self) -> &[Column] {
        &self.columns
    }
}
