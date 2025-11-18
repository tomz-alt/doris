// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Main catalog manager

use fe_common::{DbId, TableId, Result};
use crate::database::Database;
use crate::table::OlapTable;
use crate::datastore::DataStore;
use std::sync::Arc;
use parking_lot::RwLock;
use dashmap::DashMap;

/// Catalog - the root of all metadata
pub struct Catalog {
    /// Databases (db_name -> database)
    databases: DashMap<String, Arc<RwLock<Database>>>,

    /// Database ID map (db_id -> db_name)
    db_id_map: DashMap<DbId, String>,

    /// Tables (table_id -> table)
    tables: DashMap<TableId, Arc<RwLock<OlapTable>>>,

    /// Next database ID
    next_db_id: Arc<RwLock<DbId>>,

    /// Next table ID (reserved for future auto-ID allocation)
    #[allow(dead_code)]
    next_table_id: Arc<RwLock<TableId>>,

    /// In-memory data storage (for testing before BE integration)
    datastore: DataStore,
}

impl Catalog {
    pub fn new() -> Self {
        let catalog = Self {
            databases: DashMap::new(),
            db_id_map: DashMap::new(),
            tables: DashMap::new(),
            next_db_id: Arc::new(RwLock::new(1)),
            next_table_id: Arc::new(RwLock::new(1)),
            datastore: DataStore::new(),
        };

        // Create system databases
        catalog.create_system_databases();

        catalog
    }

    /// Get the datastore for data operations
    pub fn datastore(&self) -> &DataStore {
        &self.datastore
    }

    /// Create system databases (information_schema, etc.)
    fn create_system_databases(&self) {
        // Create information_schema database
        let info_schema_db = Database::new(
            0,
            fe_common::constants::INFORMATION_SCHEMA_DB.to_string(),
            fe_common::constants::DEFAULT_CLUSTER.to_string(),
        );
        self.databases.insert(
            fe_common::constants::INFORMATION_SCHEMA_DB.to_string(),
            Arc::new(RwLock::new(info_schema_db)),
        );
        self.db_id_map.insert(0, fe_common::constants::INFORMATION_SCHEMA_DB.to_string());
    }

    /// Allocate a new database ID
    fn next_db_id(&self) -> DbId {
        let mut id = self.next_db_id.write();
        let result = *id;
        *id += 1;
        result
    }

    /// Allocate a new table ID (reserved for future use)
    #[allow(dead_code)]
    fn next_table_id(&self) -> TableId {
        let mut id = self.next_table_id.write();
        let result = *id;
        *id += 1;
        result
    }

    /// Create a database
    pub fn create_database(&self, db_name: String, cluster_name: String) -> Result<DbId> {
        if self.databases.contains_key(&db_name) {
            return Err(fe_common::DorisError::AlreadyExists(
                format!("Database {} already exists", db_name)
            ));
        }

        let db_id = self.next_db_id();
        let database = Database::new(db_id, db_name.clone(), cluster_name);

        self.databases.insert(db_name.clone(), Arc::new(RwLock::new(database)));
        self.db_id_map.insert(db_id, db_name);

        Ok(db_id)
    }

    /// Drop a database
    pub fn drop_database(&self, db_name: &str) -> Result<()> {
        // Check if database exists
        let db = self.databases.get(db_name)
            .ok_or_else(|| fe_common::DorisError::NotFound(
                format!("Database {} not found", db_name)
            ))?;

        let db_guard = db.read();

        // Check if database is empty
        if db_guard.table_count() > 0 {
            return Err(fe_common::DorisError::InvalidArgument(
                format!("Database {} is not empty", db_name)
            ));
        }

        let db_id = db_guard.id;
        drop(db_guard);
        drop(db);

        // Remove from maps
        self.databases.remove(db_name);
        self.db_id_map.remove(&db_id);

        Ok(())
    }

    /// Get database by name
    pub fn get_database(&self, db_name: &str) -> Result<Arc<RwLock<Database>>> {
        self.databases
            .get(db_name)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| fe_common::DorisError::NotFound(
                format!("Database {} not found", db_name)
            ))
    }

    /// Get database by ID
    pub fn get_database_by_id(&self, db_id: DbId) -> Result<Arc<RwLock<Database>>> {
        let db_name = self.db_id_map
            .get(&db_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| fe_common::DorisError::NotFound(
                format!("Database {} not found", db_id)
            ))?;
        self.get_database(&db_name)
    }

    /// List all database names
    pub fn list_databases(&self) -> Vec<String> {
        self.databases.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Create a table
    pub fn create_table(&self, db_name: &str, table: OlapTable) -> Result<TableId> {
        let db = self.get_database(db_name)?;
        let db_guard = db.read();

        let table_name = table.name.clone();
        let table_id = table.id;

        // Add table to database
        db_guard.add_table(table_name, table_id)?;

        // Add table to catalog
        self.tables.insert(table_id, Arc::new(RwLock::new(table)));

        Ok(table_id)
    }

    /// Drop a table
    pub fn drop_table(&self, db_name: &str, table_name: &str) -> Result<()> {
        let db = self.get_database(db_name)?;
        let db_guard = db.read();

        // Remove from database
        let table_id = db_guard.remove_table(table_name)?;

        // Remove from catalog
        self.tables.remove(&table_id);

        Ok(())
    }

    /// Get table by ID
    pub fn get_table(&self, table_id: TableId) -> Result<Arc<RwLock<OlapTable>>> {
        self.tables
            .get(&table_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| fe_common::DorisError::NotFound(
                format!("Table {} not found", table_id)
            ))
    }

    /// Get table by name
    pub fn get_table_by_name(&self, db_name: &str, table_name: &str) -> Result<Arc<RwLock<OlapTable>>> {
        let db = self.get_database(db_name)?;
        let db_guard = db.read();

        let table_id = db_guard.get_table_id(table_name)
            .ok_or_else(|| fe_common::DorisError::NotFound(
                format!("Table {}.{} not found", db_name, table_name)
            ))?;

        self.get_table(table_id)
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}
