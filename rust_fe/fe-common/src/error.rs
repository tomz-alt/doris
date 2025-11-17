// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Error types for Doris FE

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DorisError {
    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Analysis error: {0}")]
    AnalysisError(String),

    #[error("Catalog error: {0}")]
    CatalogError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Not supported: {0}")]
    NotSupported(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Query error: {0}")]
    QueryError(String),
}

pub type Result<T> = std::result::Result<T, DorisError>;
