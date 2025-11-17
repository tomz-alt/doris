// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Table tests - matching Java FE OlapTable behavior

use fe_catalog::{OlapTable, Column, Partition};
use fe_common::{DataType, KeysType, StorageMedium, PartitionType};

#[test]
fn test_olap_table_keys_types() {
    // Test all three keys types from Java: AGG_KEYS, DUP_KEYS, UNIQUE_KEYS
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new(2, "value".to_string(), DataType::BigInt),
    ];

    // AGG_KEYS
    let agg_table = OlapTable::new(
        1001,
        "agg_table".to_string(),
        1,
        KeysType::AggKeys,
        columns.clone()
    );
    assert_eq!(agg_table.keys_type, KeysType::AggKeys);

    // DUP_KEYS
    let dup_table = OlapTable::new(
        1002,
        "dup_table".to_string(),
        1,
        KeysType::DupKeys,
        columns.clone()
    );
    assert_eq!(dup_table.keys_type, KeysType::DupKeys);

    // UNIQUE_KEYS
    let unique_table = OlapTable::new(
        1003,
        "unique_table".to_string(),
        1,
        KeysType::UniqueKeys,
        columns
    );
    assert_eq!(unique_table.keys_type, KeysType::UniqueKeys);
}

#[test]
fn test_olap_table_partitions() {
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
    ];
    let table = OlapTable::new(
        1001,
        "test_table".to_string(),
        1,
        KeysType::DupKeys,
        columns
    );

    assert_eq!(table.partition_count(), 0);

    // Add partition
    let partition = Partition::new(20000, "p1".to_string(), 3);
    table.add_partition(partition).unwrap();

    assert_eq!(table.partition_count(), 1);

    // Add duplicate partition should fail
    let partition2 = Partition::new(20000, "p1".to_string(), 3);
    assert!(table.add_partition(partition2).is_err());

    // Remove partition
    table.remove_partition(20000).unwrap();
    assert_eq!(table.partition_count(), 0);

    // Remove non-existent partition should fail
    assert!(table.remove_partition(99999).is_err());
}

#[test]
fn test_olap_table_default_values() {
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
    ];
    let table = OlapTable::new(
        1001,
        "test_table".to_string(),
        1,
        KeysType::DupKeys,
        columns
    );

    // Check defaults match Java
    assert_eq!(table.replication_num, 3); // DEFAULT_REPLICATION_NUM
    assert_eq!(table.storage_medium, StorageMedium::Hdd);
    assert_eq!(table.partition_type, PartitionType::Unpartitioned);
    assert_eq!(table.bloom_filter_columns.len(), 0);
}

#[test]
fn test_olap_table_key_value_columns() {
    let columns = vec![
        Column::new_key(1, "k1".to_string(), DataType::Int),
        Column::new_key(2, "k2".to_string(), DataType::Varchar { len: 50 }),
        Column::new(3, "v1".to_string(), DataType::BigInt),
        Column::new(4, "v2".to_string(), DataType::Double),
    ];

    let table = OlapTable::new(
        1001,
        "test_table".to_string(),
        1,
        KeysType::DupKeys,
        columns
    );

    let key_cols = table.key_columns();
    assert_eq!(key_cols.len(), 2);
    assert_eq!(key_cols[0].name, "k1");
    assert_eq!(key_cols[1].name, "k2");

    let value_cols = table.value_columns();
    assert_eq!(value_cols.len(), 2);
    assert_eq!(value_cols[0].name, "v1");
    assert_eq!(value_cols[1].name, "v2");
}

#[test]
fn test_olap_table_get_column_by_name() {
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new(2, "name".to_string(), DataType::Varchar { len: 100 }),
        Column::new(3, "age".to_string(), DataType::Int),
    ];

    let table = OlapTable::new(
        1001,
        "test_table".to_string(),
        1,
        KeysType::DupKeys,
        columns
    );

    // Find existing columns
    assert!(table.get_column("id").is_some());
    assert!(table.get_column("name").is_some());
    assert!(table.get_column("age").is_some());

    // Non-existent column
    assert!(table.get_column("nonexistent").is_none());
}

#[test]
fn test_olap_table_storage_medium() {
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
    ];
    let mut table = OlapTable::new(
        1001,
        "test_table".to_string(),
        1,
        KeysType::DupKeys,
        columns
    );

    // Default is HDD
    assert_eq!(table.storage_medium, StorageMedium::Hdd);

    // Change to SSD
    table.storage_medium = StorageMedium::Ssd;
    assert_eq!(table.storage_medium, StorageMedium::Ssd);

    // Change to S3
    table.storage_medium = StorageMedium::S3;
    assert_eq!(table.storage_medium, StorageMedium::S3);
}

#[test]
fn test_olap_table_replication_num() {
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
    ];
    let mut table = OlapTable::new(
        1001,
        "test_table".to_string(),
        1,
        KeysType::DupKeys,
        columns
    );

    assert_eq!(table.replication_num, 3); // Default

    table.replication_num = 1;
    assert_eq!(table.replication_num, 1);

    table.replication_num = 5;
    assert_eq!(table.replication_num, 5);
}

#[test]
fn test_olap_table_bloom_filter_columns() {
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new(2, "name".to_string(), DataType::Varchar { len: 100 }),
    ];
    let mut table = OlapTable::new(
        1001,
        "test_table".to_string(),
        1,
        KeysType::DupKeys,
        columns
    );

    assert!(table.bloom_filter_columns.is_empty());

    table.bloom_filter_columns.push("id".to_string());
    table.bloom_filter_columns.push("name".to_string());

    assert_eq!(table.bloom_filter_columns.len(), 2);
    assert_eq!(table.bloom_filter_columns[0], "id");
    assert_eq!(table.bloom_filter_columns[1], "name");
}

#[test]
fn test_olap_table_comment() {
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
    ];
    let mut table = OlapTable::new(
        1001,
        "test_table".to_string(),
        1,
        KeysType::DupKeys,
        columns
    );

    assert!(table.comment.is_none());

    table.comment = Some("Test table comment".to_string());
    assert_eq!(table.comment, Some("Test table comment".to_string()));
}
