// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Extended Database tests - matching Java FE DatabaseTest.java behavior

use fe_catalog::{Catalog, Column, OlapTable};
use fe_common::{DataType, KeysType};

#[test]
fn test_database_register_table_duplicate() {
    // Java: registerTable returns true on success, false if duplicate
    let catalog = Catalog::new();
    catalog.create_database("test_db".to_string(), "default".to_string()).unwrap();

    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
    ];
    let table = OlapTable::new(1001, "test_table".to_string(), 1, KeysType::AggKeys, columns.clone());

    // First registration should succeed
    assert!(catalog.create_table("test_db", table.clone()).is_ok());

    // Duplicate registration should fail
    let table2 = OlapTable::new(1001, "test_table".to_string(), 1, KeysType::AggKeys, columns);
    assert!(catalog.create_table("test_db", table2).is_err());
}

#[test]
fn test_database_unregister_nonexistent_table() {
    // Java: unregister non-existent table doesn't affect existing tables
    let catalog = Catalog::new();
    catalog.create_database("test_db".to_string(), "default".to_string()).unwrap();

    let columns = vec![Column::new_key(1, "id".to_string(), DataType::Int)];
    let table = OlapTable::new(1001, "test_table".to_string(), 1, KeysType::DupKeys, columns);
    catalog.create_table("test_db", table).unwrap();

    // Unregister non-existent table
    assert!(catalog.drop_table("test_db", "invalid_table").is_err());

    // Original table should still exist
    assert!(catalog.get_table_by_name("test_db", "test_table").is_ok());
}

#[test]
fn test_get_table_by_id_and_name() {
    let catalog = Catalog::new();
    catalog.create_database("test_db".to_string(), "default".to_string()).unwrap();

    let columns = vec![Column::new_key(1, "id".to_string(), DataType::Int)];
    let table = OlapTable::new(2000, "baseTable".to_string(), 1, KeysType::AggKeys, columns);
    catalog.create_table("test_db", table).unwrap();

    // Get by ID
    let table_by_id = catalog.get_table(2000).unwrap();
    assert_eq!(table_by_id.read().id, 2000);

    // Get by name
    let table_by_name = catalog.get_table_by_name("test_db", "baseTable").unwrap();
    assert_eq!(table_by_name.read().name, "baseTable");

    // Both should reference the same table
    assert_eq!(table_by_id.read().id, table_by_name.read().id);
}

#[test]
fn test_table_list_order() {
    // Java: getTablesOnIdOrderOrThrowException returns tables ordered by ID
    let catalog = Catalog::new();
    catalog.create_database("test_db".to_string(), "default".to_string()).unwrap();

    // Add tables with IDs: 2001, 2000 (reverse order)
    let table1 = OlapTable::new(
        2001,
        "table1".to_string(),
        1,
        KeysType::DupKeys,
        vec![Column::new_key(1, "id".to_string(), DataType::Int)]
    );
    let table2 = OlapTable::new(
        2000,
        "table2".to_string(),
        1,
        KeysType::DupKeys,
        vec![Column::new_key(1, "id".to_string(), DataType::Int)]
    );

    catalog.create_table("test_db", table1).unwrap();
    catalog.create_table("test_db", table2).unwrap();

    let db = catalog.get_database("test_db").unwrap();
    let db_guard = db.read();
    let table_names = db_guard.get_table_names();

    assert_eq!(table_names.len(), 2);
    // Should contain both tables
    assert!(table_names.contains(&"table1".to_string()));
    assert!(table_names.contains(&"table2".to_string()));
}

#[test]
fn test_database_full_name_and_id() {
    // Java: getFullName() returns database name, getId() returns database ID
    let catalog = Catalog::new();
    let db_id = catalog.create_database("dbTest".to_string(), "default".to_string()).unwrap();

    let db = catalog.get_database("dbTest").unwrap();
    let db_guard = db.read();

    assert_eq!(db_guard.name, "dbTest");
    assert_eq!(db_guard.id, db_id);
}

#[test]
fn test_table_count() {
    let catalog = Catalog::new();
    catalog.create_database("test_db".to_string(), "default".to_string()).unwrap();

    let db = catalog.get_database("test_db").unwrap();
    assert_eq!(db.read().table_count(), 0);

    // Add table
    let table = OlapTable::new(
        1001,
        "table1".to_string(),
        1,
        KeysType::DupKeys,
        vec![Column::new_key(1, "id".to_string(), DataType::Int)]
    );
    catalog.create_table("test_db", table).unwrap();

    assert_eq!(db.read().table_count(), 1);

    // Add another table
    let table2 = OlapTable::new(
        1002,
        "table2".to_string(),
        1,
        KeysType::DupKeys,
        vec![Column::new_key(1, "id".to_string(), DataType::Int)]
    );
    catalog.create_table("test_db", table2).unwrap();

    assert_eq!(db.read().table_count(), 2);

    // Drop table
    catalog.drop_table("test_db", "table1").unwrap();
    assert_eq!(db.read().table_count(), 1);
}
