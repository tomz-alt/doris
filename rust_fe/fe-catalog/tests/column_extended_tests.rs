// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Extended Column tests - matching Java FE ColumnTest.java behavior

use fe_catalog::Column;
use fe_common::{DataType, AggregateType};

#[test]
fn test_column_properties_char_type() {
    // Java test: CHAR(20) column with aggregate SUM
    let col = Column::new_value(
        1,
        "user".to_string(),
        DataType::Char { len: 20 },
        AggregateType::Sum
    )
    .with_nullable(false)
    .with_default("".to_string());

    assert_eq!(col.name, "user");
    assert_eq!(col.data_type, DataType::Char { len: 20 });
    assert_eq!(col.agg_type, AggregateType::Sum);
    assert_eq!(col.default_value, Some("".to_string()));
    assert!(!col.nullable);
}

#[test]
fn test_column_properties_int_with_default() {
    // Java test: INT column with REPLACE aggregate and default value "20"
    let col = Column::new_value(
        2,
        "age".to_string(),
        DataType::Int,
        AggregateType::Replace
    )
    .with_nullable(false)
    .with_default("20".to_string());

    assert_eq!(col.name, "age");
    assert_eq!(col.data_type, DataType::Int);
    assert_eq!(col.agg_type, AggregateType::Replace);
    assert_eq!(col.default_value, Some("20".to_string()));
}

#[test]
fn test_column_key_properties() {
    // Java test: Key column with BIGINT type
    let col = Column::new_key(3, "name".to_string(), DataType::BigInt);

    assert!(col.is_key);
    assert!(!col.nullable); // Key columns are not nullable
    assert_eq!(col.agg_type, AggregateType::None);
}

#[test]
fn test_column_equality() {
    // Java test: Column equality checks
    let col1 = Column::new_value(
        1,
        "age".to_string(),
        DataType::Int,
        AggregateType::Replace
    ).with_default("20".to_string());

    let col2 = Column::new_value(
        1,
        "age".to_string(),
        DataType::Int,
        AggregateType::Replace
    ).with_default("20".to_string());

    // Same properties should be equal
    assert_eq!(col1, col2);

    // Self equality
    assert_eq!(col1, col1);
}

#[test]
fn test_column_varchar_type() {
    let col = Column::new(2, "str_col".to_string(), DataType::Varchar { len: 255 });

    assert_eq!(col.name, "str_col");
    match col.data_type {
        DataType::Varchar { len } => assert_eq!(len, 255),
        _ => panic!("Expected Varchar type"),
    }
}

#[test]
fn test_column_decimal_type() {
    let col = Column::new(3, "dec_col".to_string(), DataType::Decimal { precision: 10, scale: 2 });

    assert_eq!(col.name, "dec_col");
    match col.data_type {
        DataType::Decimal { precision, scale } => {
            assert_eq!(precision, 10);
            assert_eq!(scale, 2);
        },
        _ => panic!("Expected Decimal type"),
    }
}

#[test]
fn test_all_primitive_types() {
    // Test all basic primitive types like Java test
    let types = vec![
        ("tinyint", DataType::TinyInt),
        ("smallint", DataType::SmallInt),
        ("int", DataType::Int),
        ("bigint", DataType::BigInt),
        ("float", DataType::Float),
        ("double", DataType::Double),
        ("date", DataType::Date),
        ("datetime", DataType::DateTime),
        ("boolean", DataType::Boolean),
    ];

    for (i, (name, data_type)) in types.into_iter().enumerate() {
        let col = Column::new(i as i32, name.to_string(), data_type.clone());
        assert_eq!(col.name, name);
        assert_eq!(col.data_type, data_type);
    }
}

#[test]
fn test_column_with_comment() {
    let col = Column::new(1, "id".to_string(), DataType::Int)
        .with_comment("Primary key column".to_string());

    assert_eq!(col.comment, Some("Primary key column".to_string()));
}

#[test]
fn test_column_position() {
    let col = Column::new(1, "id".to_string(), DataType::Int)
        .with_position(5);

    assert_eq!(col.position, 5);
}

#[test]
fn test_all_aggregate_types_enum() {
    // Ensure all aggregate types work correctly
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

    for agg_type in agg_types {
        let col = Column::new_value(1, "col".to_string(), DataType::BigInt, agg_type);
        assert_eq!(col.agg_type, agg_type);
    }
}

#[test]
fn test_column_auto_increment() {
    let mut col = Column::new_key(1, "id".to_string(), DataType::BigInt);
    col.auto_increment = true;

    assert!(col.auto_increment);
    assert!(col.is_key);
}
