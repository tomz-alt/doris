// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Tests for Column module - verify behavior parity with Java FE Column.java

use fe_catalog::Column;
use fe_common::{DataType, AggregateType};

#[test]
fn test_column_creation() {
    let col = Column::new(1, "test_col".to_string(), DataType::Int);

    assert_eq!(col.id, 1);
    assert_eq!(col.name, "test_col");
    assert_eq!(col.data_type, DataType::Int);
    assert!(col.nullable); // Default nullable
    assert!(!col.is_key); // Default not key
    assert_eq!(col.agg_type, AggregateType::None);
}

#[test]
fn test_key_column() {
    let col = Column::new_key(1, "id".to_string(), DataType::BigInt);

    assert!(col.is_key);
    assert!(!col.nullable); // Key columns are not nullable
    assert_eq!(col.agg_type, AggregateType::None);
}

#[test]
fn test_value_column_with_aggregate() {
    let col = Column::new_value(1, "total".to_string(), DataType::BigInt, AggregateType::Sum);

    assert!(!col.is_key);
    assert!(col.nullable);
    assert_eq!(col.agg_type, AggregateType::Sum);
}

#[test]
fn test_column_builder_pattern() {
    let col = Column::new(1, "test".to_string(), DataType::Varchar { len: 100 })
        .with_nullable(false)
        .with_default("default_value".to_string())
        .with_comment("Test column".to_string())
        .with_position(5);

    assert!(!col.nullable);
    assert_eq!(col.default_value, Some("default_value".to_string()));
    assert_eq!(col.comment, Some("Test column".to_string()));
    assert_eq!(col.position, 5);
}

#[test]
fn test_data_types() {
    // Test various data types
    let int_col = Column::new(1, "int_col".to_string(), DataType::Int);
    assert_eq!(int_col.data_type, DataType::Int);

    let varchar_col = Column::new(2, "str_col".to_string(), DataType::Varchar { len: 255 });
    assert!(matches!(varchar_col.data_type, DataType::Varchar { len: 255 }));

    let decimal_col = Column::new(3, "dec_col".to_string(), DataType::Decimal { precision: 10, scale: 2 });
    assert!(matches!(decimal_col.data_type, DataType::Decimal { precision: 10, scale: 2 }));
}

#[test]
fn test_aggregate_types() {
    // Test all aggregate types
    let agg_types = vec![
        AggregateType::None,
        AggregateType::Sum,
        AggregateType::Min,
        AggregateType::Max,
        AggregateType::Replace,
        AggregateType::ReplaceIfNotNull,
        AggregateType::Hll,
        AggregateType::Bitmap,
        AggregateType::Quantile,
    ];

    for (i, agg_type) in agg_types.into_iter().enumerate() {
        let col = Column::new_value(i as i32, format!("col_{}", i), DataType::BigInt, agg_type);
        assert_eq!(col.agg_type, agg_type);
    }
}
