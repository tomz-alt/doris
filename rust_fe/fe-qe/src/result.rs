// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Query Result Types

use serde::{Deserialize, Serialize};

/// Query execution result
#[derive(Debug, Clone, PartialEq)]
pub enum QueryResult {
    /// OK result with message
    Ok(String),
    /// Result set with rows
    ResultSet(ResultSet),
    /// Error
    Error(String),
}

impl QueryResult {
    pub fn ok(msg: impl Into<String>) -> Self {
        QueryResult::Ok(msg.into())
    }

    pub fn error(msg: impl Into<String>) -> Self {
        QueryResult::Error(msg.into())
    }
}

/// Result set from SELECT query
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResultSet {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Row>,
}

/// Column metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
}

/// Row of data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub values: Vec<Value>,
}

/// Cell value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Boolean(bool),
    TinyInt(i8),
    SmallInt(i16),
    Int(i32),
    BigInt(i64),
    Float(f32),
    Double(f64),
    String(String),
    Date(String),
    DateTime(String),
}
