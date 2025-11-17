// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Constants used throughout Doris FE

/// Default HTTP port
pub const DEFAULT_HTTP_PORT: u16 = 8030;

/// Default RPC port
pub const DEFAULT_RPC_PORT: u16 = 9020;

/// Default query port (MySQL protocol)
pub const DEFAULT_QUERY_PORT: u16 = 9030;

/// Default edit log port
pub const DEFAULT_EDIT_LOG_PORT: u16 = 9010;

/// Information schema database name
pub const INFORMATION_SCHEMA_DB: &str = "information_schema";

/// Internal database name
pub const INTERNAL_DB_NAME: &str = "__internal_schema";

/// Default cluster name
pub const DEFAULT_CLUSTER: &str = "default_cluster";

/// Maximum database name length
pub const MAX_DATABASE_NAME_LENGTH: usize = 256;

/// Maximum table name length
pub const MAX_TABLE_NAME_LENGTH: usize = 256;

/// Maximum column name length
pub const MAX_COLUMN_NAME_LENGTH: usize = 256;

/// Maximum partition name length
pub const MAX_PARTITION_NAME_LENGTH: usize = 256;

/// Default replication number
pub const DEFAULT_REPLICATION_NUM: i16 = 3;

/// Minimum replication number
pub const MIN_REPLICATION_NUM: i16 = 1;

/// Maximum replication number
pub const MAX_REPLICATION_NUM: i16 = 32767;

/// Default query timeout (seconds)
pub const DEFAULT_QUERY_TIMEOUT_S: u64 = 300;

/// Default transaction timeout (seconds)
pub const DEFAULT_TXN_TIMEOUT_S: u64 = 86400; // 1 day

/// Invalid ID
pub const INVALID_ID: i64 = -1;
