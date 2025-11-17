// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Serialization tests - matching Java MaterializedIndexTest.java

use fe_catalog::{Column, Database, OlapTable, Partition, Replica, Index};
use fe_catalog::index::IndexType;
use fe_common::{DataType, KeysType, AggregateType, ReplicaState};
use serde_json;

#[test]
fn test_column_serialization() {
    // Java: Column should serialize and deserialize correctly
    let column = Column::new_key(1, "id".to_string(), DataType::BigInt);

    let json = serde_json::to_string(&column).unwrap();
    let deserialized: Column = serde_json::from_str(&json).unwrap();

    assert_eq!(column.id, deserialized.id);
    assert_eq!(column.name, deserialized.name);
    assert_eq!(column.data_type, deserialized.data_type);
    assert_eq!(column.is_key, deserialized.is_key);
}

#[test]
fn test_column_with_default_value_serialization() {
    // Java: Column with default value should preserve it
    let column = Column::new(1, "status".to_string(), DataType::Varchar { len: 10 })
        .with_default("active".to_string());

    let json = serde_json::to_string(&column).unwrap();
    let deserialized: Column = serde_json::from_str(&json).unwrap();

    assert_eq!(column.default_value, deserialized.default_value);
    assert_eq!(Some("active".to_string()), deserialized.default_value);
}

#[test]
fn test_database_serialization() {
    // Java: Database should serialize and deserialize correctly
    let database = Database::new(1, "test_db".to_string(), "default".to_string());

    let json = serde_json::to_string(&database).unwrap();
    let deserialized: Database = serde_json::from_str(&json).unwrap();

    assert_eq!(database.id, deserialized.id);
    assert_eq!(database.name, deserialized.name);
    assert_eq!(database.cluster_name, deserialized.cluster_name);
    assert_eq!(database.data_quota, deserialized.data_quota);
    assert_eq!(database.replica_quota, deserialized.replica_quota);
}

#[test]
fn test_partition_serialization() {
    // Java: Partition should serialize and deserialize correctly
    let partition = Partition::new(1, "p20240101".to_string(), 3);

    let json = serde_json::to_string(&partition).unwrap();
    let deserialized: Partition = serde_json::from_str(&json).unwrap();

    assert_eq!(partition.id, deserialized.id);
    assert_eq!(partition.name, deserialized.name);
    assert_eq!(partition.visible_version, deserialized.visible_version);
    assert_eq!(partition.next_version, deserialized.next_version);
    assert_eq!(partition.replication_num, deserialized.replication_num);
}

#[test]
fn test_replica_serialization() {
    // Java: Replica should serialize and deserialize correctly
    let replica = Replica::new(1, 100, 1001);

    let json = serde_json::to_string(&replica).unwrap();
    let deserialized: Replica = serde_json::from_str(&json).unwrap();

    assert_eq!(replica.id, deserialized.id);
    assert_eq!(replica.backend_id, deserialized.backend_id);
    assert_eq!(replica.tablet_id, deserialized.tablet_id);
    assert_eq!(replica.state, deserialized.state);
    assert_eq!(replica.version, deserialized.version);
}

#[test]
fn test_replica_with_bad_state_serialization() {
    // Java: Replica in BAD state should preserve state
    let mut replica = Replica::new(1, 100, 1001);
    replica.mark_bad();

    let json = serde_json::to_string(&replica).unwrap();
    let deserialized: Replica = serde_json::from_str(&json).unwrap();

    assert_eq!(ReplicaState::Bad, deserialized.state);
    assert!(deserialized.is_bad);
}

#[test]
fn test_index_serialization() {
    // Java: Index should serialize and deserialize correctly
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::BigInt),
        Column::new(2, "name".to_string(), DataType::Varchar { len: 100 }),
    ];

    let index = Index::new(1, "idx_name".to_string(), IndexType::Base, columns);

    let json = serde_json::to_string(&index).unwrap();
    let deserialized: Index = serde_json::from_str(&json).unwrap();

    assert_eq!(index.id, deserialized.id);
    assert_eq!(index.name, deserialized.name);
    assert_eq!(index.index_type, deserialized.index_type);
    assert_eq!(index.columns.len(), deserialized.columns.len());
}

#[test]
fn test_table_basic_serialization() {
    // Java: OlapTable should serialize and deserialize correctly
    let columns = vec![
        Column::new_key(1, "id".to_string(), DataType::Int),
        Column::new(2, "value".to_string(), DataType::BigInt),
    ];

    let table = OlapTable::new(1001, "test_table".to_string(), 1, KeysType::DupKeys, columns);

    let json = serde_json::to_string(&table).unwrap();
    let deserialized: OlapTable = serde_json::from_str(&json).unwrap();

    assert_eq!(table.id, deserialized.id);
    assert_eq!(table.name, deserialized.name);
    assert_eq!(table.db_id, deserialized.db_id);
    assert_eq!(table.keys_type, deserialized.keys_type);
    assert_eq!(table.columns.len(), deserialized.columns.len());
}

#[test]
fn test_table_with_agg_columns_serialization() {
    // Java: Table with aggregate columns should preserve aggregate types
    let columns = vec![
        Column::new_key(1, "user_id".to_string(), DataType::BigInt),
        Column::new_value(2, "total".to_string(), DataType::BigInt, AggregateType::Sum),
        Column::new_value(3, "count".to_string(), DataType::BigInt, AggregateType::Sum),
        Column::new_value(4, "max_val".to_string(), DataType::Int, AggregateType::Max),
    ];

    let table = OlapTable::new(1001, "agg_table".to_string(), 1, KeysType::AggKeys, columns);

    let json = serde_json::to_string(&table).unwrap();
    let deserialized: OlapTable = serde_json::from_str(&json).unwrap();

    assert_eq!(KeysType::AggKeys, deserialized.keys_type);
    assert_eq!(AggregateType::None, deserialized.columns[0].agg_type);  // Key column
    assert_eq!(AggregateType::Sum, deserialized.columns[1].agg_type);
    assert_eq!(AggregateType::Sum, deserialized.columns[2].agg_type);
    assert_eq!(AggregateType::Max, deserialized.columns[3].agg_type);
}

#[test]
fn test_complex_datatype_serialization() {
    // Java: Complex types (ARRAY, MAP, STRUCT) should serialize correctly
    let array_col = Column::new(1, "tags".to_string(),
        DataType::Array(Box::new(DataType::Varchar { len: 50 })));

    let json = serde_json::to_string(&array_col).unwrap();
    let deserialized: Column = serde_json::from_str(&json).unwrap();

    assert_eq!(array_col.data_type, deserialized.data_type);
}

#[test]
fn test_map_datatype_serialization() {
    // Java: MAP type should serialize correctly
    let map_col = Column::new(1, "properties".to_string(),
        DataType::Map {
            key: Box::new(DataType::Varchar { len: 50 }),
            value: Box::new(DataType::String)
        });

    let json = serde_json::to_string(&map_col).unwrap();
    let deserialized: Column = serde_json::from_str(&json).unwrap();

    assert_eq!(map_col.data_type, deserialized.data_type);
}

#[test]
fn test_struct_datatype_serialization() {
    // Java: STRUCT type should serialize correctly
    let struct_col = Column::new(1, "user_info".to_string(),
        DataType::Struct {
            fields: vec![
                ("id".to_string(), DataType::BigInt),
                ("name".to_string(), DataType::Varchar { len: 100 }),
                ("age".to_string(), DataType::Int),
            ]
        });

    let json = serde_json::to_string(&struct_col).unwrap();
    let deserialized: Column = serde_json::from_str(&json).unwrap();

    assert_eq!(struct_col.data_type, deserialized.data_type);
}

#[test]
fn test_decimal_serialization() {
    // Java: DECIMAL type should preserve precision and scale
    let decimal_col = Column::new(1, "price".to_string(),
        DataType::Decimal { precision: 18, scale: 4 });

    let json = serde_json::to_string(&decimal_col).unwrap();
    let deserialized: Column = serde_json::from_str(&json).unwrap();

    match deserialized.data_type {
        DataType::Decimal { precision, scale } => {
            assert_eq!(18, precision);
            assert_eq!(4, scale);
        },
        _ => panic!("Expected Decimal type"),
    }
}
