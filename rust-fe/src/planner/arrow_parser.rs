//! Arrow Result Data Parser
//!
//! Parses Apache Arrow IPC format from BE for efficient zero-copy data transfer

use arrow::ipc::reader::StreamReader;
use arrow::record_batch::RecordBatch;
use std::io::Cursor;
use tracing::{debug, error};

use crate::error::{DorisError, Result};
use crate::mysql::packet::{ColumnDefinition, ResultRow};
use crate::mysql::ColumnType;
use crate::query::QueryResult;

/// Arrow result parser
pub struct ArrowResultParser;

impl ArrowResultParser {
    /// Parse Arrow IPC stream from BE response
    pub fn parse_arrow_stream(data: &[u8]) -> Result<QueryResult> {
        if data.is_empty() {
            return Ok(QueryResult::empty());
        }

        debug!("Parsing Arrow IPC stream ({} bytes)", data.len());

        let cursor = Cursor::new(data);
        let reader = StreamReader::try_new(cursor, None).map_err(|e| {
            error!("Failed to create Arrow stream reader: {}", e);
            DorisError::QueryExecution(format!("Failed to parse Arrow stream: {}", e))
        })?;

        let mut all_batches = Vec::new();
        let mut schema = None;

        for batch_result in reader {
            match batch_result {
                Ok(batch) => {
                    if schema.is_none() {
                        schema = Some(batch.schema());
                    }
                    all_batches.push(batch);
                }
                Err(e) => {
                    error!("Failed to read Arrow batch: {}", e);
                    return Err(DorisError::QueryExecution(format!(
                        "Failed to read Arrow batch: {}",
                        e
                    )));
                }
            }
        }

        if all_batches.is_empty() {
            return Ok(QueryResult::empty());
        }

        debug!("Parsed {} Arrow record batches", all_batches.len());

        Self::convert_to_query_result(&all_batches)
    }

    /// Convert Arrow RecordBatches to QueryResult
    fn convert_to_query_result(batches: &[RecordBatch]) -> Result<QueryResult> {
        if batches.is_empty() {
            return Ok(QueryResult::empty());
        }

        let schema = batches[0].schema();
        let columns = Self::convert_schema_to_columns(&schema);

        let mut rows = Vec::new();

        for batch in batches {
            let num_rows = batch.num_rows();
            let num_cols = batch.num_columns();

            for row_idx in 0..num_rows {
                let mut row_values = Vec::with_capacity(num_cols);

                for col_idx in 0..num_cols {
                    let column = batch.column(col_idx);
                    let value = Self::extract_value_at(column, row_idx)?;
                    row_values.push(value);
                }

                rows.push(ResultRow::new(row_values));
            }
        }

        debug!("Converted {} rows from Arrow format", rows.len());

        Ok(QueryResult::new_select(columns, rows))
    }

    /// Convert Arrow schema to MySQL column definitions
    fn convert_schema_to_columns(schema: &arrow::datatypes::SchemaRef) -> Vec<ColumnDefinition> {
        schema
            .fields()
            .iter()
            .map(|field| {
                let column_type = Self::arrow_type_to_mysql_type(field.data_type());
                ColumnDefinition::new(field.name().clone(), column_type)
            })
            .collect()
    }

    /// Map Arrow data type to MySQL column type
    fn arrow_type_to_mysql_type(arrow_type: &arrow::datatypes::DataType) -> ColumnType {
        use arrow::datatypes::DataType;

        match arrow_type {
            DataType::Boolean => ColumnType::Tiny,
            DataType::Int8 => ColumnType::Tiny,
            DataType::Int16 => ColumnType::Short,
            DataType::Int32 => ColumnType::Long,
            DataType::Int64 => ColumnType::LongLong,
            DataType::UInt8 => ColumnType::Tiny,
            DataType::UInt16 => ColumnType::Short,
            DataType::UInt32 => ColumnType::Long,
            DataType::UInt64 => ColumnType::LongLong,
            DataType::Float32 => ColumnType::Float,
            DataType::Float64 => ColumnType::Double,
            DataType::Utf8 => ColumnType::VarString,
            DataType::LargeUtf8 => ColumnType::VarString,
            DataType::Date32 => ColumnType::Date,
            DataType::Date64 => ColumnType::DateTime,
            DataType::Timestamp(_, _) => ColumnType::Timestamp,
            DataType::Decimal128(_, _) => ColumnType::NewDecimal,
            DataType::Decimal256(_, _) => ColumnType::NewDecimal,
            _ => ColumnType::VarString, // Default to string
        }
    }

    /// Extract value from Arrow array at specific index
    fn extract_value_at(
        column: &arrow::array::ArrayRef,
        row_idx: usize,
    ) -> Result<Option<String>> {
        use arrow::array::*;
        use arrow::datatypes::DataType;

        if column.is_null(row_idx) {
            return Ok(None);
        }

        let value_str = match column.data_type() {
            DataType::Boolean => {
                let array = column.as_any().downcast_ref::<BooleanArray>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::Int8 => {
                let array = column.as_any().downcast_ref::<Int8Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::Int16 => {
                let array = column.as_any().downcast_ref::<Int16Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::Int32 => {
                let array = column.as_any().downcast_ref::<Int32Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::Int64 => {
                let array = column.as_any().downcast_ref::<Int64Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::UInt8 => {
                let array = column.as_any().downcast_ref::<UInt8Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::UInt16 => {
                let array = column.as_any().downcast_ref::<UInt16Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::UInt32 => {
                let array = column.as_any().downcast_ref::<UInt32Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::UInt64 => {
                let array = column.as_any().downcast_ref::<UInt64Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::Float32 => {
                let array = column.as_any().downcast_ref::<Float32Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::Float64 => {
                let array = column.as_any().downcast_ref::<Float64Array>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::Utf8 => {
                let array = column.as_any().downcast_ref::<StringArray>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::LargeUtf8 => {
                let array = column.as_any().downcast_ref::<LargeStringArray>().unwrap();
                array.value(row_idx).to_string()
            }
            DataType::Date32 => {
                let array = column.as_any().downcast_ref::<Date32Array>().unwrap();
                let days = array.value(row_idx);
                // Convert days since epoch to date string
                format!("Date({})", days)
            }
            DataType::Date64 => {
                let array = column.as_any().downcast_ref::<Date64Array>().unwrap();
                let millis = array.value(row_idx);
                format!("DateTime({})", millis)
            }
            _ => {
                // Fallback: use debug representation
                format!("{:?}", column.slice(row_idx, 1))
            }
        };

        Ok(Some(value_str))
    }

    /// Parse Arrow Flight data (for future Flight SQL support)
    pub fn parse_arrow_flight(data: &[u8]) -> Result<QueryResult> {
        // For now, delegate to stream parser
        // In the future, this could handle Flight-specific encoding
        Self::parse_arrow_stream(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int32Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::ipc::writer::StreamWriter;
    use std::sync::Arc;

    #[test]
    fn test_parse_arrow_stream() {
        // Create a simple Arrow record batch
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, false),
        ]));

        let id_array = Int32Array::from(vec![1, 2, 3]);
        let name_array = StringArray::from(vec!["Alice", "Bob", "Charlie"]);

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(id_array), Arc::new(name_array)],
        )
        .unwrap();

        // Serialize to Arrow IPC stream
        let mut buffer = Vec::new();
        {
            let mut writer = StreamWriter::try_new(&mut buffer, &schema).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }

        // Parse the stream
        let result = ArrowResultParser::parse_arrow_stream(&buffer).unwrap();

        // Verify
        assert_eq!(result.rows.len(), 3);
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.columns[0].name, "id");
        assert_eq!(result.columns[1].name, "name");
    }

    #[test]
    fn test_arrow_type_mapping() {
        use arrow::datatypes::DataType;

        // Test that mapping function doesn't panic
        let _ = ArrowResultParser::arrow_type_to_mysql_type(&DataType::Int32);
        let _ = ArrowResultParser::arrow_type_to_mysql_type(&DataType::Utf8);
        let _ = ArrowResultParser::arrow_type_to_mysql_type(&DataType::Float64);
        let _ = ArrowResultParser::arrow_type_to_mysql_type(&DataType::Boolean);
        let _ = ArrowResultParser::arrow_type_to_mysql_type(&DataType::Date32);
    }
}
