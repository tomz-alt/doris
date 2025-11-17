// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Column definition

use fe_common::types::{DataType, AggregateType, ColumnId};
use serde::{Deserialize, Serialize};

/// Column metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Column {
    /// Column ID (unique within a table)
    pub id: ColumnId,

    /// Column name
    pub name: String,

    /// Data type
    pub data_type: DataType,

    /// Is nullable
    pub nullable: bool,

    /// Is key column
    pub is_key: bool,

    /// Aggregate type (for aggregate key model)
    pub agg_type: AggregateType,

    /// Default value
    pub default_value: Option<String>,

    /// Comment
    pub comment: Option<String>,

    /// Is auto-increment
    pub auto_increment: bool,

    /// Column position
    pub position: i32,
}

impl Column {
    pub fn new(id: ColumnId, name: String, data_type: DataType) -> Self {
        Self {
            id,
            name,
            data_type,
            nullable: true,
            is_key: false,
            agg_type: AggregateType::None,
            default_value: None,
            comment: None,
            auto_increment: false,
            position: 0,
        }
    }

    /// Create a key column
    pub fn new_key(id: ColumnId, name: String, data_type: DataType) -> Self {
        Self {
            id,
            name,
            data_type,
            nullable: false,
            is_key: true,
            agg_type: AggregateType::None,
            default_value: None,
            comment: None,
            auto_increment: false,
            position: 0,
        }
    }

    /// Create a value column with aggregate type
    pub fn new_value(id: ColumnId, name: String, data_type: DataType, agg_type: AggregateType) -> Self {
        Self {
            id,
            name,
            data_type,
            nullable: true,
            is_key: false,
            agg_type,
            default_value: None,
            comment: None,
            auto_increment: false,
            position: 0,
        }
    }

    /// Set nullable
    pub fn with_nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    /// Set default value
    pub fn with_default(mut self, default_value: String) -> Self {
        self.default_value = Some(default_value);
        self
    }

    /// Set comment
    pub fn with_comment(mut self, comment: String) -> Self {
        self.comment = Some(comment);
        self
    }

    /// Set position
    pub fn with_position(mut self, position: i32) -> Self {
        self.position = position;
        self
    }
}
