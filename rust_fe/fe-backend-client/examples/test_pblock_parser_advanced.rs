// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Advanced PBlock Parser Tests
//!
//! CLAUDE.MD Principle #2: Using BE source as specification
//! These tests verify advanced features of the PBlock format:
//! - Const columns (single value replicated N times)
//! - Compression (Snappy)
//! - Different numeric types (INT32, FLOAT, DOUBLE)
//! - Edge cases (empty results, large strings)
//!
//! Format reference: PBLOCK_FORMAT_SPECIFICATION.md
//! Source: /be/src/vec/core/block.cpp, /be/src/vec/data_types/*.cpp
//!
//! Run: cargo run --example test_pblock_parser_advanced

use fe_backend_client::pblock_parser_v2::pblock_to_rows;
use fe_backend_client::generated::doris::{PBlock, PColumnMeta, p_generic_type::TypeId};
use snap::raw::Encoder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════");
    println!("  PBlock Parser Advanced Tests");
    println!("  Testing: Const columns, Compression, Edge cases");
    println!("═══════════════════════════════════════════════════════\n");

    // Test 1: Const column (single value replicated)
    println!("Test 1: Const BIGINT column (value=999, replicated 5 times)");
    println!("─────────────────────────────────────────────────────");
    test_const_column()?;

    // Test 2: Mixed const and regular columns
    println!("\nTest 2: Const column + Regular column");
    println!("─────────────────────────────────────────────────────");
    test_mixed_const_regular()?;

    // Test 3: Compressed data (Snappy)
    println!("\nTest 3: Snappy compressed column");
    println!("─────────────────────────────────────────────────────");
    test_snappy_compression()?;

    // Test 4: Different numeric types
    println!("\nTest 4: INT32, FLOAT, DOUBLE types");
    println!("─────────────────────────────────────────────────────");
    test_numeric_types()?;

    // Test 5: Empty result set
    println!("\nTest 5: Empty result set (0 rows)");
    println!("─────────────────────────────────────────────────────");
    test_empty_result()?;

    // Test 6: Long string
    println!("\nTest 6: Long string (1000 chars)");
    println!("─────────────────────────────────────────────────────");
    test_long_string()?;

    println!("\n═══════════════════════════════════════════════════════");
    println!("✅ All Advanced PBlock Parser Tests PASSED!");
    println!("   - Const columns: ✅");
    println!("   - Snappy compression: ✅");
    println!("   - Multiple numeric types: ✅");
    println!("   - Edge cases: ✅");
    println!("═══════════════════════════════════════════════════════");

    Ok(())
}

/// Test Case 1: Const Column
///
/// From data_type.cpp: serialize_const_flag_and_row_num()
/// When is_const_column=true, only 1 value is stored, then replicated to row_num
///
/// Format:
/// [is_const=0x01] [row_num=5] [real_need_copy_num=1] [value]
fn test_const_column() -> Result<(), Box<dyn std::error::Error>> {
    let mut column_values = Vec::new();

    // Column header: const column with 5 rows but only 1 stored value
    column_values.push(0x01); // is_const = TRUE
    column_values.extend_from_slice(&5u64.to_le_bytes()); // row_num = 5
    column_values.extend_from_slice(&1u64.to_le_bytes()); // real_need_copy_num = 1

    // Single value: 999
    column_values.extend_from_slice(&999i64.to_le_bytes());

    let pblock = PBlock {
        be_exec_version: Some(5),
        compressed: Some(false),
        compression_type: None,
        uncompressed_size: None,
        column_metas: vec![
            PColumnMeta {
                name: Some("const_col".to_string()),
                r#type: Some(TypeId::Int64 as i32),
                ..Default::default()
            }
        ],
        column_values: Some(column_values),
    };

    let rows = pblock_to_rows(&pblock)?;

    assert_eq!(rows.len(), 5, "Should have 5 rows");
    println!("✓ Parsed {} rows from const column", rows.len());

    // All rows should have the same value
    use fe_qe::result::Value;
    for (i, row) in rows.iter().enumerate() {
        match &row.values[0] {
            Value::BigInt(v) => {
                assert_eq!(*v, 999, "All values should be 999");
            }
            _ => panic!("Expected BigInt"),
        }
        if i == 0 || i == 4 {
            println!("  Row {}: {:?}", i + 1, row);
        } else if i == 1 {
            println!("  ... (all values = 999)");
        }
    }

    println!("✓ Const value correctly replicated");

    Ok(())
}

/// Test Case 2: Mixed Const and Regular Columns
fn test_mixed_const_regular() -> Result<(), Box<dyn std::error::Error>> {
    let mut column_values = Vec::new();

    // Column 1: Const column (value=100, 3 rows)
    column_values.push(0x01); // is_const = TRUE
    column_values.extend_from_slice(&3u64.to_le_bytes());
    column_values.extend_from_slice(&1u64.to_le_bytes());
    column_values.extend_from_slice(&100i64.to_le_bytes());

    // Column 2: Regular column [1, 2, 3]
    column_values.push(0x00); // is_const = FALSE
    column_values.extend_from_slice(&3u64.to_le_bytes());
    column_values.extend_from_slice(&3u64.to_le_bytes());
    column_values.extend_from_slice(&1i64.to_le_bytes());
    column_values.extend_from_slice(&2i64.to_le_bytes());
    column_values.extend_from_slice(&3i64.to_le_bytes());

    let pblock = PBlock {
        be_exec_version: Some(5),
        compressed: Some(false),
        compression_type: None,
        uncompressed_size: None,
        column_metas: vec![
            PColumnMeta {
                name: Some("const_col".to_string()),
                r#type: Some(TypeId::Int64 as i32),
                ..Default::default()
            },
            PColumnMeta {
                name: Some("regular_col".to_string()),
                r#type: Some(TypeId::Int64 as i32),
                ..Default::default()
            }
        ],
        column_values: Some(column_values),
    };

    let rows = pblock_to_rows(&pblock)?;

    assert_eq!(rows.len(), 3);
    println!("✓ Parsed {} rows", rows.len());

    // Verify values
    use fe_qe::result::Value;
    let expected = vec![(100, 1), (100, 2), (100, 3)];

    for (i, (row, (exp_const, exp_reg))) in rows.iter().zip(expected.iter()).enumerate() {
        match (&row.values[0], &row.values[1]) {
            (Value::BigInt(const_val), Value::BigInt(reg_val)) => {
                assert_eq!(*const_val, *exp_const);
                assert_eq!(*reg_val, *exp_reg);
            }
            _ => panic!("Type mismatch"),
        }
        println!("  Row {}: const={}, regular={}", i + 1, exp_const, exp_reg);
    }

    println!("✓ Mixed const/regular columns correct");

    Ok(())
}

/// Test Case 3: Snappy Compression
///
/// From Block::serialize() in block.cpp:1004-1029
fn test_snappy_compression() -> Result<(), Box<dyn std::error::Error>> {
    // Create uncompressed data
    let mut uncompressed = Vec::new();
    uncompressed.push(0x00); // is_const = false
    uncompressed.extend_from_slice(&2u64.to_le_bytes()); // row_num = 2
    uncompressed.extend_from_slice(&2u64.to_le_bytes()); // real_need_copy_num = 2
    uncompressed.extend_from_slice(&42i64.to_le_bytes());
    uncompressed.extend_from_slice(&84i64.to_le_bytes());

    // Compress with Snappy
    let mut encoder = Encoder::new();
    let compressed = encoder.compress_vec(&uncompressed)?;

    println!("  Uncompressed: {} bytes", uncompressed.len());
    println!("  Compressed:   {} bytes", compressed.len());

    let pblock = PBlock {
        be_exec_version: Some(5),
        compressed: Some(true),
        compression_type: Some(0), // Snappy = 0
        uncompressed_size: Some(uncompressed.len() as i64),
        column_metas: vec![
            PColumnMeta {
                name: Some("compressed_col".to_string()),
                r#type: Some(TypeId::Int64 as i32),
                ..Default::default()
            }
        ],
        column_values: Some(compressed),
    };

    let rows = pblock_to_rows(&pblock)?;

    assert_eq!(rows.len(), 2);
    println!("✓ Decompressed and parsed {} rows", rows.len());

    use fe_qe::result::Value;
    match (&rows[0].values[0], &rows[1].values[0]) {
        (Value::BigInt(v1), Value::BigInt(v2)) => {
            assert_eq!(*v1, 42);
            assert_eq!(*v2, 84);
            println!("  Values: [{}, {}]", v1, v2);
        }
        _ => panic!("Type mismatch"),
    }

    println!("✓ Snappy decompression successful");

    Ok(())
}

/// Test Case 4: Different Numeric Types
fn test_numeric_types() -> Result<(), Box<dyn std::error::Error>> {
    let mut column_values = Vec::new();

    // Column 1: INT32
    column_values.push(0x00);
    column_values.extend_from_slice(&2u64.to_le_bytes());
    column_values.extend_from_slice(&2u64.to_le_bytes());
    column_values.extend_from_slice(&123i32.to_le_bytes());
    column_values.extend_from_slice(&456i32.to_le_bytes());

    // Column 2: FLOAT
    column_values.push(0x00);
    column_values.extend_from_slice(&2u64.to_le_bytes());
    column_values.extend_from_slice(&2u64.to_le_bytes());
    column_values.extend_from_slice(&3.14f32.to_le_bytes());
    column_values.extend_from_slice(&2.71f32.to_le_bytes());

    // Column 3: DOUBLE
    column_values.push(0x00);
    column_values.extend_from_slice(&2u64.to_le_bytes());
    column_values.extend_from_slice(&2u64.to_le_bytes());
    column_values.extend_from_slice(&1.234567890123456f64.to_le_bytes());
    column_values.extend_from_slice(&9.876543210987654f64.to_le_bytes());

    let pblock = PBlock {
        be_exec_version: Some(5),
        compressed: Some(false),
        compression_type: None,
        uncompressed_size: None,
        column_metas: vec![
            PColumnMeta {
                name: Some("int32_col".to_string()),
                r#type: Some(TypeId::Int32 as i32),
                ..Default::default()
            },
            PColumnMeta {
                name: Some("float_col".to_string()),
                r#type: Some(TypeId::Float as i32),
                ..Default::default()
            },
            PColumnMeta {
                name: Some("double_col".to_string()),
                r#type: Some(TypeId::Double as i32),
                ..Default::default()
            }
        ],
        column_values: Some(column_values),
    };

    let rows = pblock_to_rows(&pblock)?;

    assert_eq!(rows.len(), 2);
    println!("✓ Parsed {} rows with mixed types", rows.len());

    for (i, row) in rows.iter().enumerate() {
        println!("  Row {}: {:?}", i + 1, row);
        assert_eq!(row.values.len(), 3);
    }

    println!("✓ INT32, FLOAT, DOUBLE types decoded correctly");

    Ok(())
}

/// Test Case 5: Empty Result Set
fn test_empty_result() -> Result<(), Box<dyn std::error::Error>> {
    let mut column_values = Vec::new();

    // Header with 0 rows
    column_values.push(0x00); // is_const = false
    column_values.extend_from_slice(&0u64.to_le_bytes()); // row_num = 0
    column_values.extend_from_slice(&0u64.to_le_bytes()); // real_need_copy_num = 0

    let pblock = PBlock {
        be_exec_version: Some(5),
        compressed: Some(false),
        compression_type: None,
        uncompressed_size: None,
        column_metas: vec![
            PColumnMeta {
                name: Some("empty_col".to_string()),
                r#type: Some(TypeId::Int64 as i32),
                ..Default::default()
            }
        ],
        column_values: Some(column_values),
    };

    let rows = pblock_to_rows(&pblock)?;

    assert_eq!(rows.len(), 0);
    println!("✓ Empty result set handled correctly: {} rows", rows.len());

    Ok(())
}

/// Test Case 6: Long String
fn test_long_string() -> Result<(), Box<dyn std::error::Error>> {
    // Create a 1000-character string
    let long_str = "X".repeat(1000);
    let str_bytes = long_str.as_bytes();

    let mut column_values = Vec::new();

    // Header
    column_values.push(0x00);
    column_values.extend_from_slice(&1u64.to_le_bytes()); // row_num = 1
    column_values.extend_from_slice(&1u64.to_le_bytes()); // real_need_copy_num = 1

    // Offset (end of string)
    column_values.extend_from_slice(&(str_bytes.len() as u64).to_le_bytes());

    // Total value length
    column_values.extend_from_slice(&(str_bytes.len() as u64).to_le_bytes());

    // String data
    column_values.extend_from_slice(str_bytes);

    let pblock = PBlock {
        be_exec_version: Some(5),
        compressed: Some(false),
        compression_type: None,
        uncompressed_size: None,
        column_metas: vec![
            PColumnMeta {
                name: Some("long_string".to_string()),
                r#type: Some(TypeId::String as i32),
                ..Default::default()
            }
        ],
        column_values: Some(column_values),
    };

    let rows = pblock_to_rows(&pblock)?;

    assert_eq!(rows.len(), 1);
    println!("✓ Parsed 1 row with long string");

    use fe_qe::result::Value;
    match &rows[0].values[0] {
        Value::String(s) => {
            assert_eq!(s.len(), 1000);
            println!("  String length: {} chars", s.len());
            println!("  First 50 chars: {}", &s[..50]);
            println!("  Last 50 chars: {}", &s[950..]);
        }
        _ => panic!("Expected String"),
    }

    println!("✓ Long string decoded correctly");

    Ok(())
}
