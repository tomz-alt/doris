// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Common type definitions

use serde::{Deserialize, Serialize};

/// Table ID type
pub type TableId = i64;

/// Database ID type
pub type DbId = i64;

/// Partition ID type
pub type PartitionId = i64;

/// Tablet ID type
pub type TabletId = i64;

/// Backend ID type
pub type BackendId = i64;

/// Transaction ID type
pub type TransactionId = i64;

/// Index ID type
pub type IndexId = i64;

/// Column ID type (unique within a table)
pub type ColumnId = i32;

/// Replica ID type
pub type ReplicaId = i64;

/// Frontend node type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrontendNodeType {
    /// Leader node (can read and write metadata)
    Leader,
    /// Follower node (can be elected as leader)
    Follower,
    /// Observer node (read-only, cannot be elected)
    Observer,
    /// Unknown status
    Unknown,
}

/// Table type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableType {
    /// OLAP table (native Doris table)
    Olap,
    /// MySQL external table
    Mysql,
    /// ODBC external table
    Odbc,
    /// Broker external table
    Broker,
    /// Elasticsearch external table
    Elasticsearch,
    /// Hive external table
    Hive,
    /// Iceberg external table
    Iceberg,
    /// Hudi external table
    Hudi,
    /// JDBC external table
    Jdbc,
    /// Schema table (information_schema, etc.)
    Schema,
    /// View
    View,
    /// Materialized view
    MaterializedView,
}

/// Partition type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartitionType {
    /// Not partitioned
    Unpartitioned,
    /// Range partitioning
    Range,
    /// List partitioning
    List,
}

/// Key type for OLAP tables
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeysType {
    /// Duplicate key model
    DupKeys,
    /// Unique key model
    UniqueKeys,
    /// Aggregate key model
    AggKeys,
}

/// Data type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    Boolean,
    TinyInt,
    SmallInt,
    Int,
    BigInt,
    LargeInt,
    Float,
    Double,
    Decimal { precision: u8, scale: u8 },
    Date,
    DateTime,
    Char { len: u32 },
    Varchar { len: u32 },
    String,
    Binary,
    Json,
    Array(Box<DataType>),
    Map { key: Box<DataType>, value: Box<DataType> },
    Struct { fields: Vec<(String, DataType)> },
    Hll,
    Bitmap,
    Quantile,
}

/// Aggregate function type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregateType {
    None,
    Sum,
    Min,
    Max,
    Replace,
    ReplaceIfNotNull,
    Hll,
    Bitmap,
    Quantile,
}

/// Storage medium
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageMedium {
    Hdd,
    Ssd,
    S3,
}

/// Replica status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicaState {
    Normal,
    Clone,
    SchemaChange,
    Rollup,
    Decommission,
    Bad,
}
