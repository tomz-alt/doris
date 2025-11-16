// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

//! Rust implementation of Doris Meta Service using SlateDB
//!
//! This module provides a cloud-native metadata service for Apache Doris,
//! replacing the FoundationDB-based C++ implementation with a SlateDB-backed
//! Rust implementation.
//!
//! # Architecture
//!
//! - **Storage**: SlateDB (cloud-native LSM KV store on object storage)
//! - **Protocol**: gRPC with protobuf messages
//! - **Concurrency**: Single-writer, multi-reader model
//! - **Transactions**: Optimistic concurrency control with SI/SSI
//!
//! # Main Components
//!
//! - `keys`: Key encoding/decoding for hierarchical metadata keys
//! - `storage`: SlateDB wrapper with metadata-specific configuration
//! - `service`: Main MetaService gRPC server implementation
//! - `transaction`: Transaction management (begin, commit, abort)
//! - `tablet`: Tablet lifecycle and metadata operations
//! - `rowset`: Rowset operations (prepare, commit)
//! - `partition`: Partition operations
//! - `job`: Job management (compaction, schema change)
//! - `resource`: Resource and cluster management

pub mod keys;
pub mod storage;
pub mod service;
pub mod transaction;
pub mod tablet;
pub mod rowset;
pub mod partition;
pub mod job;
pub mod resource;
pub mod config;

pub use config::MetaServiceConfig;
pub use service::MetaServiceImpl;
pub use storage::MetaStorage;

/// Result type for meta service operations
pub type Result<T> = std::result::Result<T, MetaServiceError>;

/// Error types for meta service
#[derive(Debug, thiserror::Error)]
pub enum MetaServiceError {
    // Temporarily commented out for Phase 1 testing
    // #[error("Storage error: {0}")]
    // Storage(#[from] slatedb::error::SlateDBError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("Transaction conflict: {0}")]
    TransactionConflict(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Meta service error codes (matching C++ implementation)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MetaServiceCode {
    Ok = 0,
    InvalidArgument = 1,
    NotFound = 2,
    AlreadyExists = 3,
    TransactionConflict = 4,
    Internal = 100,
}

impl From<MetaServiceError> for MetaServiceCode {
    fn from(err: MetaServiceError) -> Self {
        match err {
            MetaServiceError::InvalidArgument(_) => MetaServiceCode::InvalidArgument,
            MetaServiceError::NotFound(_) => MetaServiceCode::NotFound,
            MetaServiceError::AlreadyExists(_) => MetaServiceCode::AlreadyExists,
            MetaServiceError::TransactionConflict(_) => MetaServiceCode::TransactionConflict,
            _ => MetaServiceCode::Internal,
        }
    }
}
