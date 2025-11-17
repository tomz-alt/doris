// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Common utilities and foundational types for Doris Rust FE
//!
//! This module provides:
//! - Error types and result handling
//! - Configuration management
//! - Common data types
//! - Utility functions
//! - Constants

pub mod error;
pub mod config;
pub mod types;
pub mod utils;
pub mod constants;
pub mod version;

pub use error::{DorisError, Result};
pub use config::Config;
pub use types::{
    TableId, DbId, PartitionId, TabletId, BackendId, TransactionId,
    IndexId, ColumnId, ReplicaId, FrontendNodeType, TableType,
    PartitionType, KeysType, DataType, AggregateType, StorageMedium,
    ReplicaState,
};
