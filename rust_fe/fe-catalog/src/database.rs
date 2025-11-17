// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Database definition and operations

use fe_common::{DbId, TableId, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;

/// Database metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Database {
    /// Database ID
    pub id: DbId,

    /// Database name
    pub name: String,

    /// Cluster name
    pub cluster_name: String,

    /// Create time (milliseconds)
    pub create_time: u64,

    /// Data quota (bytes)
    pub data_quota: i64,

    /// Replica quota
    pub replica_quota: i64,

    /// Database properties
    #[serde(skip)]
    pub properties: DashMap<String, String>,

    /// Tables in this database (table_name -> table_id)
    #[serde(skip)]
    pub tables: Arc<DashMap<String, TableId>>,
}

impl Database {
    pub fn new(id: DbId, name: String, cluster_name: String) -> Self {
        Self {
            id,
            name,
            cluster_name,
            create_time: fe_common::utils::current_timestamp_ms(),
            data_quota: -1, // -1 means no limit
            replica_quota: -1,
            properties: DashMap::new(),
            tables: Arc::new(DashMap::new()),
        }
    }

    /// Add a table to this database
    pub fn add_table(&self, table_name: String, table_id: TableId) -> Result<()> {
        if self.tables.contains_key(&table_name) {
            return Err(fe_common::DorisError::AlreadyExists(
                format!("Table {} already exists in database {}", table_name, self.name)
            ));
        }
        self.tables.insert(table_name, table_id);
        Ok(())
    }

    /// Remove a table from this database
    pub fn remove_table(&self, table_name: &str) -> Result<TableId> {
        self.tables
            .remove(table_name)
            .map(|(_, table_id)| table_id)
            .ok_or_else(|| {
                fe_common::DorisError::NotFound(
                    format!("Table {} not found in database {}", table_name, self.name)
                )
            })
    }

    /// Get table ID by name
    pub fn get_table_id(&self, table_name: &str) -> Option<TableId> {
        self.tables.get(table_name).map(|entry| *entry.value())
    }

    /// Get all table names
    pub fn get_table_names(&self) -> Vec<String> {
        self.tables.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get table count
    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// Set property
    pub fn set_property(&self, key: String, value: String) {
        self.properties.insert(key, value);
    }

    /// Get property
    pub fn get_property(&self, key: &str) -> Option<String> {
        self.properties.get(key).map(|entry| entry.value().clone())
    }
}
