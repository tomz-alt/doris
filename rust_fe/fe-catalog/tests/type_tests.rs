// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Type validation and comparison tests - matching Java ColumnTypeTest.java

use fe_catalog::Column;
use fe_common::DataType;

#[test]
fn test_primitive_type_equality() {
    // Java: Same types should be equal
    let col1 = Column::new_key(1, "id".to_string(), DataType::Int);
    let col2 = Column::new_key(2, "other_id".to_string(), DataType::Int);

    assert_eq!(col1.data_type, col2.data_type);
}

#[test]
fn test_primitive_type_inequality() {
    // Java: Different types should not be equal
    let int_col = Column::new_key(1, "id".to_string(), DataType::Int);
    let bigint_col = Column::new_key(2, "big_id".to_string(), DataType::BigInt);

    assert_ne!(int_col.data_type, bigint_col.data_type);
}

#[test]
fn test_varchar_type_equality() {
    // Java: VARCHAR with same length should be equal
    let col1 = Column::new(1, "name".to_string(), DataType::Varchar { len: 100 });
    let col2 = Column::new(2, "title".to_string(), DataType::Varchar { len: 100 });

    assert_eq!(col1.data_type, col2.data_type);
}

#[test]
fn test_varchar_type_different_length() {
    // Java: VARCHAR with different length should not be equal
    let col1 = Column::new(1, "name".to_string(), DataType::Varchar { len: 100 });
    let col2 = Column::new(2, "code".to_string(), DataType::Varchar { len: 10 });

    assert_ne!(col1.data_type, col2.data_type);
}

#[test]
fn test_decimal_type_equality() {
    // Java: DECIMAL with same precision/scale should be equal
    let col1 = Column::new(1, "price".to_string(), DataType::Decimal { precision: 12, scale: 2 });
    let col2 = Column::new(2, "cost".to_string(), DataType::Decimal { precision: 12, scale: 2 });

    assert_eq!(col1.data_type, col2.data_type);
}

#[test]
fn test_decimal_type_different_precision() {
    // Java: DECIMAL with different precision should not be equal
    let col1 = Column::new(1, "price".to_string(), DataType::Decimal { precision: 12, scale: 2 });
    let col2 = Column::new(2, "amount".to_string(), DataType::Decimal { precision: 10, scale: 2 });

    assert_ne!(col1.data_type, col2.data_type);
}

#[test]
fn test_decimal_type_different_scale() {
    // Java: DECIMAL with different scale should not be equal
    let col1 = Column::new(1, "price".to_string(), DataType::Decimal { precision: 12, scale: 2 });
    let col2 = Column::new(2, "rate".to_string(), DataType::Decimal { precision: 12, scale: 4 });

    assert_ne!(col1.data_type, col2.data_type);
}

#[test]
fn test_array_type_equality() {
    // Java: ARRAY with same element type should be equal
    let col1 = Column::new(1, "tags".to_string(),
        DataType::Array(Box::new(DataType::Varchar { len: 50 })));
    let col2 = Column::new(2, "labels".to_string(),
        DataType::Array(Box::new(DataType::Varchar { len: 50 })));

    assert_eq!(col1.data_type, col2.data_type);
}

#[test]
fn test_array_type_different_element() {
    // Java: ARRAY with different element types should not be equal
    let col1 = Column::new(1, "tags".to_string(),
        DataType::Array(Box::new(DataType::Varchar { len: 50 })));
    let col2 = Column::new(2, "ids".to_string(),
        DataType::Array(Box::new(DataType::Int)));

    assert_ne!(col1.data_type, col2.data_type);
}

#[test]
fn test_all_numeric_types() {
    // Java: Verify all numeric types are distinct
    let types = vec![
        DataType::TinyInt,
        DataType::SmallInt,
        DataType::Int,
        DataType::BigInt,
        DataType::LargeInt,
        DataType::Float,
        DataType::Double,
    ];

    // Each type should be unique
    for (i, type1) in types.iter().enumerate() {
        for (j, type2) in types.iter().enumerate() {
            if i == j {
                assert_eq!(type1, type2);
            } else {
                assert_ne!(type1, type2);
            }
        }
    }
}

#[test]
fn test_all_character_types() {
    // Java: Verify character types with different lengths
    let varchar10 = DataType::Varchar { len: 10 };
    let varchar100 = DataType::Varchar { len: 100 };
    let varchar255 = DataType::Varchar { len: 255 };
    let string = DataType::String;

    assert_ne!(varchar10, varchar100);
    assert_ne!(varchar100, varchar255);
    assert_ne!(varchar10, string);
}

#[test]
fn test_date_time_types() {
    // Java: Verify date/time types are distinct
    let date = DataType::Date;
    let datetime = DataType::DateTime;

    assert_ne!(date, datetime);
}

#[test]
fn test_complex_nested_types() {
    // Java: Nested types should work correctly
    let map_int_varchar = DataType::Map {
        key: Box::new(DataType::Int),
        value: Box::new(DataType::Varchar { len: 50 })
    };

    let map_int_varchar_same = DataType::Map {
        key: Box::new(DataType::Int),
        value: Box::new(DataType::Varchar { len: 50 })
    };

    let map_int_varchar_different = DataType::Map {
        key: Box::new(DataType::Int),
        value: Box::new(DataType::Varchar { len: 100 })
    };

    assert_eq!(map_int_varchar, map_int_varchar_same);
    assert_ne!(map_int_varchar, map_int_varchar_different);
}

#[test]
fn test_struct_type_equality() {
    // Java: STRUCT types with same fields should be equal
    let struct1 = DataType::Struct {
        fields: vec![
            ("id".to_string(), DataType::Int),
            ("name".to_string(), DataType::Varchar { len: 50 }),
        ]
    };

    let struct2 = DataType::Struct {
        fields: vec![
            ("id".to_string(), DataType::Int),
            ("name".to_string(), DataType::Varchar { len: 50 }),
        ]
    };

    assert_eq!(struct1, struct2);
}

#[test]
fn test_struct_type_different_fields() {
    // Java: STRUCT types with different fields should not be equal
    let struct1 = DataType::Struct {
        fields: vec![
            ("id".to_string(), DataType::Int),
            ("name".to_string(), DataType::Varchar { len: 50 }),
        ]
    };

    let struct2 = DataType::Struct {
        fields: vec![
            ("id".to_string(), DataType::BigInt),  // Different type
            ("name".to_string(), DataType::Varchar { len: 50 }),
        ]
    };

    assert_ne!(struct1, struct2);
}
