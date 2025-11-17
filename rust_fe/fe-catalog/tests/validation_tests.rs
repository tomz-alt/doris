// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Validation tests - ensure constraints and invariants are enforced

use fe_catalog::{Catalog, Column, OlapTable};
use fe_common::{DataType, KeysType, AggregateType};

#[test]
fn test_catalog_get_nonexistent_database() {
    let catalog = Catalog::new();

    // Getting non-existent database should fail
    assert!(catalog.get_database("nonexistent").is_err());
    assert!(catalog.get_database_by_id(99999).is_err());
}

#[test]
fn test_catalog_get_nonexistent_table() {
    let catalog = Catalog::new();
    catalog.create_database("test".to_string(), "default".to_string()).unwrap();

    // Getting non-existent table should fail
    assert!(catalog.get_table(99999).is_err());
    assert!(catalog.get_table_by_name("test", "nonexistent").is_err());
}

#[test]
fn test_table_get_nonexistent_column() {
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new(2, "name".to_string(), DataType::Varchar { len: 100 }),
    ];

    let table = OlapTable::new(1001, "test".to_string(), 1, KeysType::DupKeys, columns);

    // Existing columns
    assert!(table.get_column("id").is_some());
    assert!(table.get_column("name").is_some());

    // Non-existent column returns None (not error)
    assert!(table.get_column("nonexistent").is_none());
}

#[test]
fn test_aggregate_type_with_dup_keys() {
    // Java: DUP_KEYS tables can have aggregate functions on value columns
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new_value(2, "total".to_string(), DataType::BigInt, AggregateType::Sum),
    ];

    let table = OlapTable::new(1001, "test".to_string(), 1, KeysType::DupKeys, columns);

    assert_eq!(table.columns[1].agg_type, AggregateType::Sum);
}

#[test]
fn test_aggregate_type_with_agg_keys() {
    // Java: AGG_KEYS tables must have aggregate functions on value columns
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new_value(2, "sum_val".to_string(), DataType::BigInt, AggregateType::Sum),
        Column::new_value(3, "max_val".to_string(), DataType::BigInt, AggregateType::Max),
        Column::new_value(4, "min_val".to_string(), DataType::BigInt, AggregateType::Min),
    ];

    let table = OlapTable::new(1001, "test".to_string(), 1, KeysType::AggKeys, columns);

    assert_eq!(table.keys_type, KeysType::AggKeys);
    assert_eq!(table.columns[1].agg_type, AggregateType::Sum);
    assert_eq!(table.columns[2].agg_type, AggregateType::Max);
    assert_eq!(table.columns[3].agg_type, AggregateType::Min);
}

#[test]
fn test_unique_keys_with_replace() {
    // Java: UNIQUE_KEYS tables typically use REPLACE for value columns
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new_value(2, "name".to_string(), DataType::Varchar { len: 100 }, AggregateType::Replace),
        Column::new_value(3, "age".to_string(), DataType::Int, AggregateType::Replace),
    ];

    let table = OlapTable::new(1001, "test".to_string(), 1, KeysType::UniqueKeys, columns);

    assert_eq!(table.keys_type, KeysType::UniqueKeys);
    assert_eq!(table.columns[1].agg_type, AggregateType::Replace);
    assert_eq!(table.columns[2].agg_type, AggregateType::Replace);
}

#[test]
fn test_column_default_values_preserved() {
    // Java: Default values should be preserved exactly
    let col1 = Column::new(1, "age".to_string(), DataType::Int).with_default("18".to_string());
    assert_eq!(col1.default_value, Some("18".to_string()));

    let col2 = Column::new(2, "status".to_string(), DataType::Varchar { len: 10 })
        .with_default("active".to_string());
    assert_eq!(col2.default_value, Some("active".to_string()));

    let col3 = Column::new(3, "score".to_string(), DataType::Float).with_default("0.0".to_string());
    assert_eq!(col3.default_value, Some("0.0".to_string()));
}

#[test]
fn test_table_create_time_set() {
    // Java: create_time is set on table creation
    let columns = vec![Column::new_key(1, "id".to_string(), DataType::Int)];
    let table = OlapTable::new(1001, "test".to_string(), 1, KeysType::DupKeys, columns);

    assert!(table.create_time > 0);
}

#[test]
fn test_partition_create_time_set() {
    // Java: create_time is set on partition creation
    use fe_catalog::Partition;

    let partition = Partition::new(1, "p1".to_string(), 3);
    assert!(partition.create_time > 0);
}

#[test]
fn test_replica_initial_state() {
    // Java: New replica starts in NORMAL state
    use fe_catalog::Replica;
    use fe_common::ReplicaState;

    let replica = Replica::new(1, 100, 1001);

    assert_eq!(replica.state, ReplicaState::Normal);
    assert!(!replica.is_bad);
    assert!(replica.is_healthy());
}

#[test]
fn test_column_key_vs_value_exclusivity() {
    // Java: A column is either a key or value column, not both
    let key_col = Column::new_key(1, "id".to_string(), DataType::Int);
    assert!(key_col.is_key);
    assert_eq!(key_col.agg_type, AggregateType::None); // Key columns don't have aggregates

    let value_col = Column::new_value(2, "val".to_string(), DataType::Int, AggregateType::Sum);
    assert!(!value_col.is_key);
    assert_eq!(value_col.agg_type, AggregateType::Sum);
}

#[test]
fn test_database_table_count_consistency() {
    let catalog = Catalog::new();
    catalog.create_database("test".to_string(), "default".to_string()).unwrap();

    let db = catalog.get_database("test").unwrap();

    // Initially empty
    assert_eq!(db.read().table_count(), 0);

    // Add tables
    for i in 0..5 {
        let columns = vec![Column::new_key(1, "id".to_string(), DataType::Int)];
        let table = OlapTable::new(1000 + i, format!("table{}", i), 1, KeysType::DupKeys, columns);
        catalog.create_table("test", table).unwrap();
    }

    assert_eq!(db.read().table_count(), 5);

    // Drop some tables
    catalog.drop_table("test", "table0").unwrap();
    catalog.drop_table("test", "table2").unwrap();

    assert_eq!(db.read().table_count(), 3);
}

#[test]
fn test_table_name_uniqueness_per_database() {
    // Java: Table names are unique within a database, but can repeat across databases
    let catalog = Catalog::new();
    catalog.create_database("db1".to_string(), "default".to_string()).unwrap();
    catalog.create_database("db2".to_string(), "default".to_string()).unwrap();

    let columns = vec![Column::new_key(1, "id".to_string(), DataType::Int)];

    // Same name "test_table" in different databases - should succeed
    let table1 = OlapTable::new(1001, "test_table".to_string(), 1, KeysType::DupKeys, columns.clone());
    let table2 = OlapTable::new(1002, "test_table".to_string(), 2, KeysType::DupKeys, columns);

    assert!(catalog.create_table("db1", table1).is_ok());
    assert!(catalog.create_table("db2", table2).is_ok());

    // Both should exist independently
    assert!(catalog.get_table_by_name("db1", "test_table").is_ok());
    assert!(catalog.get_table_by_name("db2", "test_table").is_ok());
}

#[test]
fn test_drop_nonexistent_table() {
    let catalog = Catalog::new();
    catalog.create_database("test".to_string(), "default".to_string()).unwrap();

    // Dropping non-existent table should fail
    assert!(catalog.drop_table("test", "nonexistent").is_err());
}

#[test]
fn test_drop_database_with_tables() {
    let catalog = Catalog::new();
    catalog.create_database("test".to_string(), "default".to_string()).unwrap();

    // Add a table
    let columns = vec![Column::new_key(1, "id".to_string(), DataType::Int)];
    let table = OlapTable::new(1001, "table1".to_string(), 1, KeysType::DupKeys, columns);
    catalog.create_table("test", table).unwrap();

    // Cannot drop non-empty database
    assert!(catalog.drop_database("test").is_err());

    // Drop table first, then database
    catalog.drop_table("test", "table1").unwrap();
    assert!(catalog.drop_database("test").is_ok());
}
