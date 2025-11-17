// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Comprehensive edge case tests for catalog components

use fe_catalog::{Catalog, OlapTable, Column, Partition, Replica};
use fe_common::{DataType, KeysType, ReplicaState, StorageMedium};
use std::sync::Arc;
use std::thread;

#[test]
fn test_database_quota_defaults() {
    // Java: Default quotas are -1 (unlimited)
    let catalog = Catalog::new();
    catalog.create_database("test".to_string(), "default".to_string()).unwrap();
    let db = catalog.get_database("test").unwrap();
    let db_guard = db.read();

    assert_eq!(db_guard.data_quota, -1); // Unlimited
    assert_eq!(db_guard.replica_quota, -1); // Unlimited
}

#[test]
fn test_column_nullable_defaults() {
    // Key columns should not be nullable, value columns should be
    let key_col = Column::new_key(1, "id".to_string(), DataType::Int);
    assert!(!key_col.nullable);

    let value_col = Column::new(2, "value".to_string(), DataType::Int);
    assert!(value_col.nullable); // Default nullable for value columns
}

#[test]
fn test_partition_version_sequence() {
    // Java: Versions must be monotonically increasing
    let mut partition = Partition::new(1, "p1".to_string(), 3);

    assert_eq!(partition.visible_version, 1);
    assert_eq!(partition.next_version, 2);

    partition.update_visible_version(2);
    assert_eq!(partition.visible_version, 2);
    assert_eq!(partition.next_version, 3);

    partition.update_visible_version(5);
    assert_eq!(partition.visible_version, 5);
    assert_eq!(partition.next_version, 6);
}

#[test]
fn test_replica_version_tracking() {
    // Java: Replica tracks version, last_success_version, last_failed_version
    let replica = Replica::new(1, 100, 1001);

    assert_eq!(replica.version, 1);
    assert_eq!(replica.last_success_version, 1);
    assert_eq!(replica.last_failed_version, -1); // No failures yet
}

#[test]
fn test_replica_state_transitions() {
    // Java: Replica can transition through different states
    let mut replica = Replica::new(1, 100, 1001);

    // Start in NORMAL
    assert_eq!(replica.state, ReplicaState::Normal);
    assert!(replica.is_healthy());

    // Transition to CLONE
    replica.state = ReplicaState::Clone;
    assert!(!replica.is_healthy());

    // Transition to SCHEMA_CHANGE
    replica.state = ReplicaState::SchemaChange;
    assert!(!replica.is_healthy());

    // Back to NORMAL
    replica.state = ReplicaState::Normal;
    assert!(replica.is_healthy());
}

#[test]
fn test_table_empty_columns() {
    // Edge case: Table with minimal columns
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
    ];

    let table = OlapTable::new(1001, "minimal".to_string(), 1, KeysType::DupKeys, columns);

    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.key_columns().len(), 1);
    assert_eq!(table.value_columns().len(), 0);
}

#[test]
fn test_table_max_replication() {
    // Java: MAX_REPLICATION_NUM = 32767
    let columns = vec![Column::new_key(1, "id".to_string(), DataType::Int)];
    let mut table = OlapTable::new(1001, "test".to_string(), 1, KeysType::DupKeys, columns);

    table.replication_num = 32767;
    assert_eq!(table.replication_num, 32767);
}

#[test]
fn test_partition_temp_flag() {
    // Java: Temporary partitions are marked with is_temp
    let mut partition = Partition::new(1, "temp_p1".to_string(), 3);

    assert!(!partition.is_temp); // Default not temporary

    partition.is_temp = true;
    assert!(partition.is_temp);
}

#[test]
fn test_concurrent_table_creation() {
    // Verify thread-safety of catalog operations
    let catalog = Arc::new(Catalog::new());
    catalog.create_database("test_db".to_string(), "default".to_string()).unwrap();

    let mut handles = vec![];

    for i in 0..10 {
        let catalog = Arc::clone(&catalog);
        let handle = thread::spawn(move || {
            let columns = vec![
                Column::new_key(1, "id".to_string(), DataType::Int),
            ];
            let table = OlapTable::new(
                1000 + i,
                format!("table_{}", i),
                1,
                KeysType::DupKeys,
                columns
            );
            catalog.create_table("test_db", table)
        });
        handles.push(handle);
    }

    for handle in handles {
        assert!(handle.join().unwrap().is_ok());
    }

    let db = catalog.get_database("test_db").unwrap();
    assert_eq!(db.read().table_count(), 10);
}

#[test]
fn test_column_position_ordering() {
    // Java: Columns maintain position order
    let col1 = Column::new(1, "a".to_string(), DataType::Int).with_position(0);
    let col2 = Column::new(2, "b".to_string(), DataType::Int).with_position(1);
    let col3 = Column::new(3, "c".to_string(), DataType::Int).with_position(2);

    assert_eq!(col1.position, 0);
    assert_eq!(col2.position, 1);
    assert_eq!(col3.position, 2);
}

#[test]
fn test_replica_data_size_update() {
    // Java: updateVersionInfo updates data_size and row_count
    let mut replica = Replica::new(1, 100, 1001);

    assert_eq!(replica.data_size, 0);
    assert_eq!(replica.row_count, 0);

    replica.update_version(2, 1024 * 1024, 1000);
    assert_eq!(replica.data_size, 1024 * 1024);
    assert_eq!(replica.row_count, 1000);

    replica.update_version(3, 2 * 1024 * 1024, 2000);
    assert_eq!(replica.data_size, 2 * 1024 * 1024);
    assert_eq!(replica.row_count, 2000);
}

#[test]
fn test_table_storage_medium_transitions() {
    // Java: Tables can change storage medium
    let columns = vec![Column::new_key(1, "id".to_string(), DataType::Int)];
    let mut table = OlapTable::new(1001, "test".to_string(), 1, KeysType::DupKeys, columns);

    // HDD -> SSD -> S3
    assert_eq!(table.storage_medium, StorageMedium::Hdd);

    table.storage_medium = StorageMedium::Ssd;
    assert_eq!(table.storage_medium, StorageMedium::Ssd);

    table.storage_medium = StorageMedium::S3;
    assert_eq!(table.storage_medium, StorageMedium::S3);
}

#[test]
fn test_database_properties() {
    // Java: Database can have custom properties
    let catalog = Catalog::new();
    catalog.create_database("test".to_string(), "default".to_string()).unwrap();
    let db = catalog.get_database("test").unwrap();

    let db_guard = db.read();

    db_guard.set_property("replication_num".to_string(), "3".to_string());
    db_guard.set_property("storage_policy".to_string(), "hot".to_string());

    assert_eq!(db_guard.get_property("replication_num"), Some("3".to_string()));
    assert_eq!(db_guard.get_property("storage_policy"), Some("hot".to_string()));
    assert_eq!(db_guard.get_property("nonexistent"), None);
}

#[test]
fn test_partition_data_size_limits() {
    // Test large data sizes (Java uses long)
    let mut partition = Partition::new(1, "p1".to_string(), 3);

    partition.data_size = i64::MAX;
    partition.row_count = i64::MAX;

    assert_eq!(partition.data_size, i64::MAX);
    assert_eq!(partition.row_count, i64::MAX);
}

#[test]
fn test_table_bloom_filter_multiple_columns() {
    // Java: Multiple columns can have bloom filters
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new(2, "name".to_string(), DataType::Varchar { len: 100 }),
        Column::new(3, "email".to_string(), DataType::Varchar { len: 100 }),
    ];

    let mut table = OlapTable::new(1001, "users".to_string(), 1, KeysType::DupKeys, columns);

    table.bloom_filter_columns.extend_from_slice(&[
        "id".to_string(),
        "name".to_string(),
        "email".to_string(),
    ]);

    assert_eq!(table.bloom_filter_columns.len(), 3);
}
