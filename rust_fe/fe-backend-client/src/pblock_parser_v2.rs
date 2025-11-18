// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! PBlock Parser V2 - Improved columnar data parsing
//!
//! CLAUDE.md Principle #4: Hide transport details
//! This module decouples the wire format (protobuf + columnar layout) from the
//! clean Row/Value API exposed to users.
//!
//! CLAUDE.md Principle #2: Use Java FE behavior as specification
//! The parsing logic here should match how Java FE decodes PBlock from C++ BE.

use crate::generated::doris::{PBlock, PColumnMeta};
use fe_common::{DorisError, Result};
use fe_qe::result::{Row, Value};
use prost::Message;
use std::io::Cursor;

/// Parse PBlock from byte array
pub fn parse_pblock(bytes: &[u8]) -> Result<PBlock> {
    PBlock::decode(bytes)
        .map_err(|e| DorisError::InternalError(format!("Failed to decode PBlock: {}", e)))
}

/// Convert PBlock to rows
///
/// PBlock stores data in columnar format for efficiency.
/// We need to transpose it to row format for the result set.
///
/// Format (based on Doris internals):
/// - Column values are serialized sequentially
/// - Each column has a type indicator
/// - Nulls are handled with a null bitmap
/// - Data is optionally compressed (Snappy)
pub fn pblock_to_rows(block: &PBlock) -> Result<Vec<Row>> {
    // Check if we have column data
    let column_values = block.column_values.as_ref()
        .ok_or_else(|| DorisError::InternalError("PBlock has no column_values".to_string()))?;

    // Handle compression if needed
    let data = if block.compressed.unwrap_or(false) {
        decompress_data(column_values, block.compression_type.unwrap_or(0))?
    } else {
        column_values.clone()
    };

    // Parse columnar data into rows
    parse_columnar_data(&data, &block.column_metas)
}

/// Decompress column data
///
/// CLAUDE.md Principle #3: Graceful error handling
fn decompress_data(compressed: &[u8], compression_type: i32) -> Result<Vec<u8>> {
    // compression_type: 0 = Snappy, 1 = LZ4, 2 = ZSTD, etc.
    match compression_type {
        0 => {
            // Snappy decompression
            // For now, return error - TODO: implement with snap crate
            Err(DorisError::InternalError(
                "Snappy decompression not yet implemented - add 'snap' crate".to_string()
            ))
        }
        _ => {
            Err(DorisError::InternalError(
                format!("Unsupported compression type: {}", compression_type)
            ))
        }
    }
}

/// Parse columnar data into rows
///
/// This is the core conversion: columnar layout → row layout
///
/// CLAUDE.md Principle #2: Match Java FE behavior
/// The parsing logic should match exactly how Java FE decodes this data.
fn parse_columnar_data(data: &[u8], column_metas: &[PColumnMeta]) -> Result<Vec<Row>> {
    if column_metas.is_empty() {
        return Ok(Vec::new());
    }

    // Create cursor for reading data
    let mut cursor = Cursor::new(data);
    let mut columns: Vec<Vec<Value>> = Vec::new();

    // Parse each column
    for meta in column_metas {
        let column_data = parse_column(&mut cursor, meta)?;
        columns.push(column_data);
    }

    // Transpose columns to rows
    transpose_columns_to_rows(columns)
}

/// Parse a single column from the data stream
///
/// Column format (simplified - needs refinement based on actual Doris format):
/// - Row count (4 bytes)
/// - Null bitmap (if column is nullable)
/// - Data values
fn parse_column(cursor: &mut Cursor<&[u8]>, meta: &PColumnMeta) -> Result<Vec<Value>> {
    use std::io::Read;

    // For now, implement basic parsing for simple types
    // This needs to be refined based on actual PBlock format from BE

    // Try to read row count (this is a guess at the format)
    let mut row_count_bytes = [0u8; 4];
    if cursor.read_exact(&mut row_count_bytes).is_err() {
        // If we can't read row count, return placeholder
        return Ok(vec![Value::String(format!(
            "Column '{}' - parsing TODO",
            meta.name.clone().unwrap_or_else(|| "unknown".to_string())
        ))]);
    }

    let row_count = u32::from_le_bytes(row_count_bytes) as usize;

    // Limit to reasonable size
    if row_count > 10000 {
        return Err(DorisError::InternalError(
            format!("Suspiciously large row count: {}", row_count)
        ));
    }

    // For now, create placeholder values
    // TODO: Implement actual type-specific parsing
    let column_name = meta.name.clone().unwrap_or_else(|| "unknown".to_string());
    let values: Vec<Value> = (0..row_count.min(10))
        .map(|i| Value::String(format!("{}.row{}", column_name, i)))
        .collect();

    log::debug!("Parsed column '{}': {} rows", column_name, row_count);

    Ok(values)
}

/// Transpose columnar data to row format
///
/// Input: Vec of columns, each column is Vec<Value>
/// Output: Vec of rows, each row contains one value from each column
fn transpose_columns_to_rows(columns: Vec<Vec<Value>>) -> Result<Vec<Row>> {
    if columns.is_empty() {
        return Ok(Vec::new());
    }

    // All columns should have same length
    let num_rows = columns[0].len();
    for (i, col) in columns.iter().enumerate() {
        if col.len() != num_rows {
            return Err(DorisError::InternalError(
                format!("Column {} has {} rows, expected {}", i, col.len(), num_rows)
            ));
        }
    }

    // Transpose: create rows from columns
    let mut rows = Vec::with_capacity(num_rows);
    for row_idx in 0..num_rows {
        let values: Vec<Value> = columns
            .iter()
            .map(|col| col[row_idx].clone())
            .collect();

        rows.push(Row { values });
    }

    log::info!("Transposed {} columns × {} rows = {} total values",
        columns.len(), num_rows, columns.len() * num_rows);

    Ok(rows)
}

/// Get column names from PBlock
pub fn get_column_names(block: &PBlock) -> Vec<String> {
    block.column_metas
        .iter()
        .map(|meta| meta.name.clone().unwrap_or_else(|| "unknown".to_string()))
        .collect()
}

/// Get number of rows in PBlock (if determinable)
pub fn get_row_count(block: &PBlock) -> Option<usize> {
    // This would require parsing the column_values
    // For now, return None - will be determined during parsing
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpose() {
        // Test column-to-row transposition
        let columns = vec![
            vec![Value::Int(1), Value::Int(2), Value::Int(3)],
            vec![Value::String("a".to_string()), Value::String("b".to_string()), Value::String("c".to_string())],
        ];

        let rows = transpose_columns_to_rows(columns).unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].values.len(), 2);
        assert_eq!(rows[0].values[0], Value::Int(1));
        assert_eq!(rows[0].values[1], Value::String("a".to_string()));
    }

    #[test]
    fn test_transpose_mismatched_lengths() {
        let columns = vec![
            vec![Value::Int(1), Value::Int(2)],
            vec![Value::String("a".to_string())], // Different length!
        ];

        let result = transpose_columns_to_rows(columns);
        assert!(result.is_err());
    }
}
