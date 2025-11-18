// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! PBlock Parser - Deserialize columnar data from C++ BE
//!
//! The BE returns query results in PBlock format (protobuf + columnar layout).

use crate::generated::doris::PBlock;
use fe_common::{DorisError, Result};
use fe_qe::result::{Row, Value};
use prost::Message;

/// Parse PBlock from byte array
pub fn parse_pblock(bytes: &[u8]) -> Result<PBlock> {
    PBlock::decode(bytes)
        .map_err(|e| DorisError::InternalError(format!("Failed to decode PBlock: {}", e)))
}

/// Convert PBlock to rows
///
/// PBlock is columnar format, we need to transpose to row format.
pub fn pblock_to_rows(block: &PBlock) -> Result<Vec<Row>> {
    // Check if we have column data
    let column_values = block.column_values.as_ref()
        .ok_or_else(|| DorisError::InternalError("PBlock has no column_values".to_string()))?;

    // Check if data is compressed
    if block.compressed.unwrap_or(false) {
        return Err(DorisError::InternalError(
            "Compressed PBlock not yet supported - TODO: implement decompression".to_string()
        ));
    }

    // For now, return basic information as a single row
    // TODO: Implement full columnar-to-row conversion
    let num_columns = block.column_metas.len();
    let data_size = column_values.len();

    // Create a single informational row
    let row = Row {
        values: vec![
            Value::String(format!("PBlock: {} columns, {} bytes of data", num_columns, data_size))
        ],
    };

    Ok(vec![row])
}

/// Get column names from PBlock
pub fn get_column_names(block: &PBlock) -> Vec<String> {
    block.column_metas
        .iter()
        .map(|meta| meta.name.clone().unwrap_or_else(|| "unknown".to_string()))
        .collect()
}

/// Get number of rows in PBlock
///
/// This is tricky because PBlock is columnar. We need to parse the column_values
/// to determine row count. For now, returns None if unknown.
pub fn get_row_count(_block: &PBlock) -> Option<usize> {
    // TODO: Parse column_values to determine row count
    // This requires understanding the columnar format encoding
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::doris::PColumnMeta;

    #[test]
    fn test_parse_empty_pblock() {
        // Create a minimal PBlock
        let block = PBlock {
            column_metas: vec![],
            column_values: Some(vec![]),
            compressed: Some(false),
            uncompressed_size: None,
            compression_type: None,
            be_exec_version: Some(0),
        };

        // Encode it
        let mut bytes = Vec::new();
        block.encode(&mut bytes).unwrap();

        // Parse it back
        let parsed = parse_pblock(&bytes).unwrap();
        assert_eq!(parsed.column_metas.len(), 0);
    }

    #[test]
    fn test_pblock_with_column_meta() {
        let block = PBlock {
            column_metas: vec![
                PColumnMeta {
                    name: Some("col1".to_string()),
                    r#type: Some(5), // INT type
                    is_nullable: Some(false),
                    decimal_param: None,
                    children: vec![],
                    result_is_nullable: None,
                    function_name: None,
                    be_exec_version: None,
                    column_path: None,
                    variant_max_subcolumns_count: None,
                }
            ],
            column_values: Some(vec![0, 0, 0, 4]), // 4 bytes of mock data
            compressed: Some(false),
            uncompressed_size: None,
            compression_type: None,
            be_exec_version: Some(0),
        };

        let names = get_column_names(&block);
        assert_eq!(names, vec!["col1"]);

        let rows = pblock_to_rows(&block).unwrap();
        assert!(!rows.is_empty());
    }
}
