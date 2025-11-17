use serde::{Deserialize, Serialize};
use datafusion::arrow::datatypes::DataType as ArrowDataType;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    // Integer types
    TinyInt,
    SmallInt,
    Int,
    BigInt,

    // Floating point
    Float,
    Double,
    Decimal { precision: u8, scale: u8 },

    // String types
    Char { length: u32 },
    Varchar { length: u32 },
    String,
    Text,

    // Date/Time
    Date,
    DateTime,
    Timestamp,

    // Boolean
    Boolean,

    // Binary
    Binary,
    Varbinary { length: u32 },

    // JSON
    Json,

    // Array
    Array(Box<DataType>),
}

impl DataType {
    pub fn to_mysql_type(&self) -> u8 {
        match self {
            DataType::TinyInt => 0x01, // MYSQL_TYPE_TINY
            DataType::SmallInt => 0x02, // MYSQL_TYPE_SHORT
            DataType::Int => 0x03, // MYSQL_TYPE_LONG
            DataType::BigInt => 0x08, // MYSQL_TYPE_LONGLONG
            DataType::Float => 0x04, // MYSQL_TYPE_FLOAT
            DataType::Double => 0x05, // MYSQL_TYPE_DOUBLE
            DataType::Decimal { .. } => 0xf6, // MYSQL_TYPE_NEWDECIMAL
            DataType::Char { .. } => 0xfe, // MYSQL_TYPE_STRING
            DataType::Varchar { .. } => 0x0f, // MYSQL_TYPE_VARCHAR
            DataType::String => 0xfd, // MYSQL_TYPE_VAR_STRING
            DataType::Text => 0xfc, // MYSQL_TYPE_BLOB
            DataType::Date => 0x0a, // MYSQL_TYPE_DATE
            DataType::DateTime => 0x0c, // MYSQL_TYPE_DATETIME
            DataType::Timestamp => 0x07, // MYSQL_TYPE_TIMESTAMP
            DataType::Boolean => 0x01, // MYSQL_TYPE_TINY
            DataType::Binary => 0xfc, // MYSQL_TYPE_BLOB
            DataType::Varbinary { .. } => 0xfc, // MYSQL_TYPE_BLOB
            DataType::Json => 0xf5, // MYSQL_TYPE_JSON
            DataType::Array(_) => 0xfd, // MYSQL_TYPE_VAR_STRING
        }
    }

    pub fn default_length(&self) -> u32 {
        match self {
            DataType::TinyInt => 4,
            DataType::SmallInt => 6,
            DataType::Int => 11,
            DataType::BigInt => 20,
            DataType::Float => 12,
            DataType::Double => 22,
            DataType::Decimal { precision, scale } => (*precision + *scale + 2) as u32,
            DataType::Char { length } => *length,
            DataType::Varchar { length } => *length,
            DataType::String => 65535,
            DataType::Text => 65535,
            DataType::Date => 10,
            DataType::DateTime => 19,
            DataType::Timestamp => 19,
            DataType::Boolean => 1,
            DataType::Binary => 255,
            DataType::Varbinary { length } => *length,
            DataType::Json => 65535,
            DataType::Array(_) => 65535,
        }
    }

    pub fn from_sql_type(sql_type: &str) -> Option<Self> {
        let sql_type_lower = sql_type.to_lowercase();

        match sql_type_lower.as_str() {
            "tinyint" => Some(DataType::TinyInt),
            "smallint" => Some(DataType::SmallInt),
            "int" | "integer" => Some(DataType::Int),
            "bigint" => Some(DataType::BigInt),
            "float" => Some(DataType::Float),
            "double" => Some(DataType::Double),
            "date" => Some(DataType::Date),
            "datetime" => Some(DataType::DateTime),
            "timestamp" => Some(DataType::Timestamp),
            "boolean" | "bool" => Some(DataType::Boolean),
            "json" => Some(DataType::Json),
            "text" => Some(DataType::Text),
            "string" => Some(DataType::String),
            _ => {
                // Handle parameterized types
                if sql_type_lower.starts_with("varchar") {
                    Some(DataType::Varchar { length: 65535 })
                } else if sql_type_lower.starts_with("char") {
                    Some(DataType::Char { length: 255 })
                } else if sql_type_lower.starts_with("decimal") {
                    Some(DataType::Decimal { precision: 10, scale: 0 })
                } else {
                    None
                }
            }
        }
    }

    /// Convert from Arrow/DataFusion DataType to Doris metadata DataType
    /// This ensures proper type mapping when registering BE-backed tables
    pub fn from_arrow_type(arrow_type: &ArrowDataType) -> Option<Self> {
        match arrow_type {
            // Integer types - CRITICAL: Int64 -> BigInt, Int32 -> Int
            ArrowDataType::Int8 => Some(DataType::TinyInt),
            ArrowDataType::Int16 => Some(DataType::SmallInt),
            ArrowDataType::Int32 => Some(DataType::Int),
            ArrowDataType::Int64 => Some(DataType::BigInt),

            // Unsigned integers (map to next larger signed type)
            ArrowDataType::UInt8 => Some(DataType::SmallInt),
            ArrowDataType::UInt16 => Some(DataType::Int),
            ArrowDataType::UInt32 => Some(DataType::BigInt),
            ArrowDataType::UInt64 => Some(DataType::BigInt), // May overflow, but best effort

            // Floating point
            ArrowDataType::Float32 => Some(DataType::Float),
            ArrowDataType::Float64 => Some(DataType::Double),

            // Decimal types
            ArrowDataType::Decimal128(precision, scale) => {
                Some(DataType::Decimal {
                    precision: (*precision as u8).min(38),
                    scale: (*scale as u8).min(38),
                })
            }
            ArrowDataType::Decimal256(precision, scale) => {
                Some(DataType::Decimal {
                    precision: (*precision as u8).min(38),
                    scale: (*scale as u8).min(38),
                })
            }

            // String types
            ArrowDataType::Utf8 => Some(DataType::String),
            ArrowDataType::LargeUtf8 => Some(DataType::Text),

            // Date/Time types
            ArrowDataType::Date32 => Some(DataType::Date),
            ArrowDataType::Date64 => Some(DataType::DateTime),
            ArrowDataType::Timestamp(_, _) => Some(DataType::Timestamp),
            ArrowDataType::Time32(_) => Some(DataType::DateTime),
            ArrowDataType::Time64(_) => Some(DataType::DateTime),

            // Boolean
            ArrowDataType::Boolean => Some(DataType::Boolean),

            // Binary types
            ArrowDataType::Binary => Some(DataType::Binary),
            ArrowDataType::LargeBinary => Some(DataType::Binary),
            ArrowDataType::FixedSizeBinary(_) => Some(DataType::Binary),

            // Complex types
            ArrowDataType::List(field) | ArrowDataType::LargeList(field) => {
                Self::from_arrow_type(field.data_type()).map(|inner| DataType::Array(Box::new(inner)))
            }

            // Unsupported types - return None for now
            _ => None,
        }
    }
}
