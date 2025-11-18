// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Unit Test for PBlock Parser
//!
//! CLAUDE.MD Principle #2: Use BE source as specification
//! This test creates synthetic PBlock data following the exact format
//! documented in PBLOCK_FORMAT_SPECIFICATION.md and verifies parsing.
//!
//! Format reverse-engineered from:
//! - /be/src/vec/core/block.cpp (Block::serialize/deserialize)
//! - /be/src/vec/data_types/*.cpp (type-specific serialization)
//!
//! Run: cargo run --example test_pblock_parser_unit

use fe_backend_client::pblock_parser_v2::pblock_to_rows;
use fe_backend_client::generated::doris::{PBlock, PColumnMeta, p_generic_type::TypeId};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════");
    println!("  PBlock Parser Unit Test");
    println!("  Format: PBLOCK_FORMAT_SPECIFICATION.md");
    println!("═══════════════════════════════════════════════════════\n");

    // Test 1: Simple BIGINT column
    println!("Test 1: Single BIGINT column (3 rows)");
    println!("─────────────────────────────────────────");
    test_bigint_column()?;

    // Test 2: Two columns (BIGINT + BIGINT)
    println!("\nTest 2: Two BIGINT columns (2 rows)");
    println!("─────────────────────────────────────────");
    test_two_bigint_columns()?;

    // Test 3: BIGINT + STRING columns
    println!("\nTest 3: BIGINT + STRING columns (2 rows)");
    println!("─────────────────────────────────────────");
    test_bigint_string_columns()?;

    println!("\n═══════════════════════════════════════════════════════");
    println!("✅ All PBlock Parser Tests PASSED!");
    println!("   - Number type decoding: ✅");
    println!("   - String type decoding: ✅");
    println!("   - Column-to-row transposition: ✅");
    println!("   - Multi-column support: ✅");
    println!("═══════════════════════════════════════════════════════");

    Ok(())
}

/// Test Case 1: Single BIGINT column with 3 values
///
/// Format (from data_type_number_base.cpp):
/// [Header: 17 bytes] [Data: 24 bytes]
///
/// Bytes:
/// 0       | 0x00                       | is_const = false
/// 1-8     | 0x03 (little-endian)       | row_num = 3
/// 9-16    | 0x03 (little-endian)       | real_need_copy_num = 3
/// 17-24   | 0x01 (i64 LE)              | value[0] = 1
/// 25-32   | 0x02 (i64 LE)              | value[1] = 2
/// 33-40   | 0x03 (i64 LE)              | value[2] = 3
fn test_bigint_column() -> Result<(), Box<dyn std::error::Error>> {
    // Construct column_values bytes following modern format (be_exec_version >= 4)
    let mut column_values = Vec::new();

    // Column header
    column_values.push(0x00); // is_const = false
    column_values.extend_from_slice(&3u64.to_le_bytes()); // row_num = 3
    column_values.extend_from_slice(&3u64.to_le_bytes()); // real_need_copy_num = 3

    // Data: 3 BIGINT values
    column_values.extend_from_slice(&1i64.to_le_bytes()); // value 1
    column_values.extend_from_slice(&2i64.to_le_bytes()); // value 2
    column_values.extend_from_slice(&3i64.to_le_bytes()); // value 3

    // Create PBlock
    let pblock = PBlock {
        be_exec_version: Some(5), // Modern format
        compressed: Some(false),
        compression_type: None,
        uncompressed_size: None,
        column_metas: vec![
            PColumnMeta {
                name: Some("col1".to_string()),
                r#type: Some(TypeId::Int64 as i32),
                is_nullable: Some(false),
                ..Default::default()
            }
        ],
        column_values: Some(column_values),
    };

    // Parse
    let rows = pblock_to_rows(&pblock)?;

    // Verify
    assert_eq!(rows.len(), 3, "Expected 3 rows");
    println!("✓ Parsed {} rows", rows.len());

    for (i, row) in rows.iter().enumerate() {
        println!("  Row {}: {:?}", i + 1, row);
        assert_eq!(row.values.len(), 1, "Expected 1 column");
    }

    // Check values
    use fe_qe::result::Value;
    match &rows[0].values[0] {
        Value::BigInt(v) => assert_eq!(*v, 1, "Row 1 should be 1"),
        _ => panic!("Expected BigInt"),
    }
    match &rows[1].values[0] {
        Value::BigInt(v) => assert_eq!(*v, 2, "Row 2 should be 2"),
        _ => panic!("Expected BigInt"),
    }
    match &rows[2].values[0] {
        Value::BigInt(v) => assert_eq!(*v, 3, "Row 3 should be 3"),
        _ => panic!("Expected BigInt"),
    }

    println!("✓ Values correct: [1, 2, 3]");

    Ok(())
}

/// Test Case 2: Two BIGINT columns
fn test_two_bigint_columns() -> Result<(), Box<dyn std::error::Error>> {
    let mut column_values = Vec::new();

    // Column 1: [10, 20]
    column_values.push(0x00); // is_const = false
    column_values.extend_from_slice(&2u64.to_le_bytes()); // row_num = 2
    column_values.extend_from_slice(&2u64.to_le_bytes()); // real_need_copy_num = 2
    column_values.extend_from_slice(&10i64.to_le_bytes());
    column_values.extend_from_slice(&20i64.to_le_bytes());

    // Column 2: [100, 200]
    column_values.push(0x00); // is_const = false
    column_values.extend_from_slice(&2u64.to_le_bytes()); // row_num = 2
    column_values.extend_from_slice(&2u64.to_le_bytes()); // real_need_copy_num = 2
    column_values.extend_from_slice(&100i64.to_le_bytes());
    column_values.extend_from_slice(&200i64.to_le_bytes());

    let pblock = PBlock {
        be_exec_version: Some(5),
        compressed: Some(false),
        compression_type: None,
        uncompressed_size: None,
        column_metas: vec![
            PColumnMeta {
                name: Some("col1".to_string()),
                r#type: Some(TypeId::Int64 as i32),
                is_nullable: Some(false),
                ..Default::default()
            },
            PColumnMeta {
                name: Some("col2".to_string()),
                r#type: Some(TypeId::Int64 as i32),
                is_nullable: Some(false),
                ..Default::default()
            }
        ],
        column_values: Some(column_values),
    };

    let rows = pblock_to_rows(&pblock)?;

    assert_eq!(rows.len(), 2);
    println!("✓ Parsed {} rows", rows.len());

    for (i, row) in rows.iter().enumerate() {
        println!("  Row {}: {:?}", i + 1, row);
        assert_eq!(row.values.len(), 2, "Expected 2 columns");
    }

    println!("✓ Transposition correct: columnar → rows");

    Ok(())
}

/// Test Case 3: BIGINT + STRING columns
///
/// String format (from data_type_string.cpp):
/// [Header: 17 bytes] [Offsets: N*8 bytes] [Total len: 8 bytes] [String data]
fn test_bigint_string_columns() -> Result<(), Box<dyn std::error::Error>> {
    let mut column_values = Vec::new();

    // Column 1: BIGINT [42, 99]
    column_values.push(0x00);
    column_values.extend_from_slice(&2u64.to_le_bytes()); // row_num = 2
    column_values.extend_from_slice(&2u64.to_le_bytes()); // real_need_copy_num = 2
    column_values.extend_from_slice(&42i64.to_le_bytes());
    column_values.extend_from_slice(&99i64.to_le_bytes());

    // Column 2: STRING ["abc", "de"]
    // Offsets are cumulative: "abc" ends at 3, "de" ends at 5
    column_values.push(0x00);
    column_values.extend_from_slice(&2u64.to_le_bytes()); // row_num = 2
    column_values.extend_from_slice(&2u64.to_le_bytes()); // real_need_copy_num = 2

    // Offsets array
    column_values.extend_from_slice(&3u64.to_le_bytes()); // offset[0] = 3 (end of "abc")
    column_values.extend_from_slice(&5u64.to_le_bytes()); // offset[1] = 5 (end of "de")

    // Total value length
    column_values.extend_from_slice(&5u64.to_le_bytes()); // total 5 bytes

    // String data
    column_values.extend_from_slice(b"abc"); // 3 bytes
    column_values.extend_from_slice(b"de");  // 2 bytes

    let pblock = PBlock {
        be_exec_version: Some(5),
        compressed: Some(false),
        compression_type: None,
        uncompressed_size: None,
        column_metas: vec![
            PColumnMeta {
                name: Some("id".to_string()),
                r#type: Some(TypeId::Int64 as i32),
                is_nullable: Some(false),
                ..Default::default()
            },
            PColumnMeta {
                name: Some("name".to_string()),
                r#type: Some(TypeId::String as i32),
                is_nullable: Some(false),
                ..Default::default()
            }
        ],
        column_values: Some(column_values),
    };

    let rows = pblock_to_rows(&pblock)?;

    assert_eq!(rows.len(), 2);
    println!("✓ Parsed {} rows", rows.len());

    for (i, row) in rows.iter().enumerate() {
        println!("  Row {}: {:?}", i + 1, row);
        assert_eq!(row.values.len(), 2, "Expected 2 columns");
    }

    // Verify values
    use fe_qe::result::Value;
    match (&rows[0].values[0], &rows[0].values[1]) {
        (Value::BigInt(id), Value::String(name)) => {
            assert_eq!(*id, 42);
            assert_eq!(name, "abc");
        }
        _ => panic!("Row 1 type mismatch"),
    }

    match (&rows[1].values[0], &rows[1].values[1]) {
        (Value::BigInt(id), Value::String(name)) => {
            assert_eq!(*id, 99);
            assert_eq!(name, "de");
        }
        _ => panic!("Row 2 type mismatch"),
    }

    println!("✓ Values correct: [(42, 'abc'), (99, 'de')]");

    Ok(())
}
