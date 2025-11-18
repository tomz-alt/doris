// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! In-memory data storage for tables
//!
//! Provides simple in-memory storage to demonstrate data insertion and querying
//! before integrating with the real C++ Backend.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A single row of data (array of values as strings for simplicity)
pub type Row = Vec<String>;

/// Data storage for a single table
#[derive(Debug, Clone)]
pub struct TableData {
    /// Rows stored in this table
    pub rows: Vec<Row>,
}

impl TableData {
    pub fn new() -> Self {
        Self { rows: Vec::new() }
    }

    pub fn insert(&mut self, row: Row) {
        self.rows.push(row);
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// In-memory data store for all tables
///
/// Key format: "database.table"
#[derive(Debug, Clone)]
pub struct DataStore {
    data: Arc<RwLock<HashMap<String, TableData>>>,
}

impl DataStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get table key
    fn table_key(db_name: &str, table_name: &str) -> String {
        format!("{}.{}", db_name, table_name)
    }

    /// Insert a row into a table
    pub fn insert_row(&self, db_name: &str, table_name: &str, row: Row) {
        let key = Self::table_key(db_name, table_name);
        let mut data = self.data.write().unwrap();

        data.entry(key)
            .or_insert_with(TableData::new)
            .insert(row);
    }

    /// Get all rows from a table
    pub fn get_rows(&self, db_name: &str, table_name: &str) -> Vec<Row> {
        let key = Self::table_key(db_name, table_name);
        let data = self.data.read().unwrap();

        data.get(&key)
            .map(|table_data| table_data.rows.clone())
            .unwrap_or_default()
    }

    /// Get row count for a table
    pub fn row_count(&self, db_name: &str, table_name: &str) -> usize {
        let key = Self::table_key(db_name, table_name);
        let data = self.data.read().unwrap();

        data.get(&key)
            .map(|table_data| table_data.len())
            .unwrap_or(0)
    }

    /// Clear all data from a table
    pub fn clear_table(&self, db_name: &str, table_name: &str) {
        let key = Self::table_key(db_name, table_name);
        let mut data = self.data.write().unwrap();
        data.remove(&key);
    }

    /// Clear all data
    pub fn clear_all(&self) {
        let mut data = self.data.write().unwrap();
        data.clear();
    }
}

impl Default for DataStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datastore_insert_and_get() {
        let store = DataStore::new();

        // Insert some rows
        store.insert_row("test_db", "test_table", vec!["1".to_string(), "foo".to_string()]);
        store.insert_row("test_db", "test_table", vec!["2".to_string(), "bar".to_string()]);

        // Get rows back
        let rows = store.get_rows("test_db", "test_table");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["1", "foo"]);
        assert_eq!(rows[1], vec!["2", "bar"]);
    }

    #[test]
    fn test_datastore_row_count() {
        let store = DataStore::new();

        assert_eq!(store.row_count("test_db", "test_table"), 0);

        store.insert_row("test_db", "test_table", vec!["1".to_string()]);
        assert_eq!(store.row_count("test_db", "test_table"), 1);

        store.insert_row("test_db", "test_table", vec!["2".to_string()]);
        assert_eq!(store.row_count("test_db", "test_table"), 2);
    }

    #[test]
    fn test_datastore_clear() {
        let store = DataStore::new();

        store.insert_row("test_db", "test_table", vec!["1".to_string()]);
        assert_eq!(store.row_count("test_db", "test_table"), 1);

        store.clear_table("test_db", "test_table");
        assert_eq!(store.row_count("test_db", "test_table"), 0);
    }

    #[test]
    fn test_datastore_multiple_tables() {
        let store = DataStore::new();

        store.insert_row("db1", "table1", vec!["a".to_string()]);
        store.insert_row("db1", "table2", vec!["b".to_string()]);
        store.insert_row("db2", "table1", vec!["c".to_string()]);

        assert_eq!(store.row_count("db1", "table1"), 1);
        assert_eq!(store.row_count("db1", "table2"), 1);
        assert_eq!(store.row_count("db2", "table1"), 1);

        let rows = store.get_rows("db2", "table1");
        assert_eq!(rows[0], vec!["c"]);
    }
}
