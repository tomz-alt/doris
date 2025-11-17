// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Index definition

use fe_common::types::IndexId;
use crate::column::Column;
use serde::{Deserialize, Serialize};

/// Index type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    /// Base index (the original table)
    Base,
    /// Rollup index (aggregated materialized view)
    Rollup,
    /// Bitmap index
    Bitmap,
    /// Inverted index
    Inverted,
    /// Bloom filter index
    BloomFilter,
    /// NGram bloom filter index
    NgramBloomFilter,
}

/// Index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    /// Index ID
    pub id: IndexId,

    /// Index name
    pub name: String,

    /// Index type
    pub index_type: IndexType,

    /// Columns in this index
    pub columns: Vec<Column>,

    /// Comment
    pub comment: Option<String>,

    /// Index properties
    pub properties: std::collections::HashMap<String, String>,
}

impl Index {
    pub fn new(id: IndexId, name: String, index_type: IndexType, columns: Vec<Column>) -> Self {
        Self {
            id,
            name,
            index_type,
            columns,
            comment: None,
            properties: std::collections::HashMap::new(),
        }
    }

    /// Create a base index
    pub fn new_base(id: IndexId, name: String, columns: Vec<Column>) -> Self {
        Self::new(id, name, IndexType::Base, columns)
    }

    /// Create a rollup index
    pub fn new_rollup(id: IndexId, name: String, columns: Vec<Column>) -> Self {
        Self::new(id, name, IndexType::Rollup, columns)
    }
}
