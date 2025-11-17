// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Integration tests for catalog module
//! These tests verify behavior parity with Java FE

use fe_catalog::{Catalog, Column, OlapTable};
use fe_common::{DataType, KeysType};

#[test]
fn test_create_drop_database() {
    let catalog = Catalog::new();

    // Create database
    let db_id = catalog.create_database("test_db".to_string(), "default_cluster".to_string()).unwrap();
    assert!(db_id > 0);

    // Verify database exists
    assert!(catalog.get_database("test_db").is_ok());

    // Create duplicate should fail
    assert!(catalog.create_database("test_db".to_string(), "default_cluster".to_string()).is_err());

    // Drop database
    assert!(catalog.drop_database("test_db").is_ok());

    // Verify database removed
    assert!(catalog.get_database("test_db").is_err());
}

#[test]
fn test_database_list() {
    let catalog = Catalog::new();

    catalog.create_database("db1".to_string(), "default_cluster".to_string()).unwrap();
    catalog.create_database("db2".to_string(), "default_cluster".to_string()).unwrap();
    catalog.create_database("db3".to_string(), "default_cluster".to_string()).unwrap();

    let dbs = catalog.list_databases();
    assert!(dbs.contains(&"db1".to_string()));
    assert!(dbs.contains(&"db2".to_string()));
    assert!(dbs.contains(&"db3".to_string()));
    assert!(dbs.contains(&"information_schema".to_string())); // System DB
}

#[test]
fn test_create_drop_table() {
    let catalog = Catalog::new();

    // Create database first
    catalog.create_database("test_db".to_string(), "default_cluster".to_string()).unwrap();

    // Create table
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new(2, "name".to_string(), DataType::Varchar { len: 255 }),
    ];

    let table = OlapTable::new(
        1001,
        "test_table".to_string(),
        1,
        KeysType::DupKeys,
        columns,
    );

    let table_id = catalog.create_table("test_db", table).unwrap();
    assert_eq!(table_id, 1001);

    // Verify table exists
    assert!(catalog.get_table_by_name("test_db", "test_table").is_ok());

    // Drop table
    assert!(catalog.drop_table("test_db", "test_table").is_ok());

    // Verify table removed
    assert!(catalog.get_table_by_name("test_db", "test_table").is_err());
}

#[test]
fn test_table_columns() {
    let catalog = Catalog::new();
    catalog.create_database("test_db".to_string(), "default_cluster".to_string()).unwrap();

    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new_key(2, "name".to_string(), DataType::Varchar { len: 255 }),
        Column::new(3, "age".to_string(), DataType::Int),
        Column::new(4, "email".to_string(), DataType::Varchar { len: 255 }),
    ];

    let table = OlapTable::new(1001, "users".to_string(), 1, KeysType::DupKeys, columns);
    catalog.create_table("test_db", table).unwrap();

    let table = catalog.get_table_by_name("test_db", "users").unwrap();
    let table_guard = table.read();

    // Test key columns
    let key_cols = table_guard.key_columns();
    assert_eq!(key_cols.len(), 2);
    assert_eq!(key_cols[0].name, "id");
    assert_eq!(key_cols[1].name, "name");

    // Test value columns
    let value_cols = table_guard.value_columns();
    assert_eq!(value_cols.len(), 2);
    assert_eq!(value_cols[0].name, "age");
    assert_eq!(value_cols[1].name, "email");

    // Test get column by name
    assert!(table_guard.get_column("id").is_some());
    assert!(table_guard.get_column("age").is_some());
    assert!(table_guard.get_column("nonexistent").is_none());
}

#[test]
fn test_drop_non_empty_database() {
    let catalog = Catalog::new();
    catalog.create_database("test_db".to_string(), "default_cluster".to_string()).unwrap();

    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
    ];
    let table = OlapTable::new(1001, "test_table".to_string(), 1, KeysType::DupKeys, columns);
    catalog.create_table("test_db", table).unwrap();

    // Cannot drop non-empty database
    assert!(catalog.drop_database("test_db").is_err());

    // Drop table first
    catalog.drop_table("test_db", "test_table").unwrap();

    // Now can drop database
    assert!(catalog.drop_database("test_db").is_ok());
}

#[test]
fn test_information_schema_exists() {
    let catalog = Catalog::new();

    // information_schema should exist by default
    assert!(catalog.get_database("information_schema").is_ok());

    let dbs = catalog.list_databases();
    assert!(dbs.contains(&"information_schema".to_string()));
}

#[test]
fn test_concurrent_database_operations() {
    use std::sync::Arc;
    use std::thread;

    let catalog = Arc::new(Catalog::new());
    let mut handles = vec![];

    // Create databases concurrently
    for i in 0..10 {
        let catalog = Arc::clone(&catalog);
        let handle = thread::spawn(move || {
            let db_name = format!("db_{}", i);
            catalog.create_database(db_name.clone(), "default_cluster".to_string()).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all databases created
    let dbs = catalog.list_databases();
    for i in 0..10 {
        assert!(dbs.contains(&format!("db_{}", i)));
    }
}
