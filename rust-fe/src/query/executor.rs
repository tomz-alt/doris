use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, info, error};

use crate::error::{DorisError, Result};
use crate::be::BackendClientPool;
use crate::mysql::packet::{ColumnDefinition, ResultRow};
use crate::mysql::ColumnType;
use crate::planner::DataFusionPlanner;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::datatypes::DataType as ArrowDataType;
use datafusion::arrow::array::*;
use super::{QueryQueue, QueryResult, QueuedQuery};

pub struct QueryExecutor {
    queue: Arc<QueryQueue>,
    datafusion: Option<Arc<DataFusionPlanner>>,
}

impl QueryExecutor {
    pub fn new(max_queue_size: usize, max_concurrent: usize) -> Self {
        Self {
            queue: Arc::new(QueryQueue::new(max_queue_size, max_concurrent)),
            datafusion: None,
        }
    }

    /// Initialize with DataFusion (async)
    pub async fn with_datafusion(max_queue_size: usize, max_concurrent: usize) -> Self {
        let datafusion = DataFusionPlanner::new().await;

        Self {
            queue: Arc::new(QueryQueue::new(max_queue_size, max_concurrent)),
            datafusion: Some(Arc::new(datafusion)),
        }
    }

    /// Register TPC-H CSV files (for testing without BE)
    pub async fn register_tpch_csv(&self, data_dir: &str) -> Result<()> {
        if let Some(ref df) = self.datafusion {
            df.register_tpch_csv_files(data_dir).await
                .map_err(|e| DorisError::QueryExecution(format!("Failed to register CSV: {}", e)))?;
            info!("TPC-H CSV data registered from: {}", data_dir);
        }
        Ok(())
    }

    pub async fn queue_query(
        &self,
        query_id: Uuid,
        query: String,
        database: Option<String>,
    ) -> Result<()> {
        let queued_query = QueuedQuery {
            query_id,
            query,
            database,
        };

        self.queue.enqueue(queued_query)
    }

    pub async fn execute_query(
        &self,
        query_id: Uuid,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        // Acquire execution slot (implements queuing)
        let _permit = self.queue.acquire_slot().await;

        info!("Executing query: {} (available slots: {})",
              query_id, self.queue.available_slots());

        // Dequeue the query
        let queued_query = self.queue.dequeue()
            .ok_or_else(|| DorisError::QueryExecution("Query not found in queue".to_string()))?;

        // Execute the query
        self.execute_internal(queued_query, be_client_pool).await
    }

    async fn execute_internal(
        &self,
        query: QueuedQuery,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        debug!("Executing query: {}", query.query.trim());

        let query_lower = query.query.trim().to_lowercase();

        // Parse query type
        if query_lower.starts_with("select") || query_lower.starts_with("with") {
            self.execute_select(query, be_client_pool).await
        } else if query_lower.starts_with("insert")
            || query_lower.starts_with("update")
            || query_lower.starts_with("delete")
        {
            self.execute_dml(query, be_client_pool).await
        } else if query_lower.starts_with("create")
            || query_lower.starts_with("drop")
            || query_lower.starts_with("alter")
        {
            self.execute_ddl(query, be_client_pool).await
        } else {
            // Unknown query type, return empty result
            Ok(QueryResult::empty())
        }
    }

    async fn execute_select(
        &self,
        query: QueuedQuery,
        _be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        info!("Executing SELECT query via DataFusion: {}", query.query_id);

        // Use DataFusion if available
        if let Some(ref df) = self.datafusion {
            match df.execute_query(&query.query).await {
                Ok(batches) => {
                    // Convert Arrow batches to MySQL result format
                    self.arrow_to_mysql_result(batches)
                }
                Err(e) => {
                    error!("DataFusion execution failed: {}", e);
                    Err(e)
                }
            }
        } else {
            // Fallback: try BE execution (if protoc is working)
            error!("DataFusion not available, returning mock data");
            self.mock_select_result(&query.query)
        }
    }

    fn arrow_to_mysql_result(&self, batches: Vec<RecordBatch>) -> Result<QueryResult> {
        if batches.is_empty() {
            return Ok(QueryResult::empty());
        }

        // Get schema from first batch
        let schema = batches[0].schema();

        // Convert to MySQL column definitions
        let columns: Vec<ColumnDefinition> = schema.fields().iter()
            .map(|field| {
                let mysql_type = arrow_to_mysql_type(field.data_type());
                let length = match field.data_type() {
                    ArrowDataType::Utf8 | ArrowDataType::LargeUtf8 => 65535,
                    ArrowDataType::Int32 => 11,
                    ArrowDataType::Int64 => 20,
                    ArrowDataType::Float32 => 12,
                    ArrowDataType::Float64 => 22,
                    _ => 255,
                };

                ColumnDefinition {
                    catalog: "def".to_string(),
                    schema: "".to_string(),
                    table: "".to_string(),
                    org_table: "".to_string(),
                    name: field.name().clone(),
                    org_name: field.name().clone(),
                    character_set: 33, // UTF-8
                    column_length: length,
                    column_type: mysql_type as u8,
                    flags: 0,
                    decimals: 0,
                }
            })
            .collect();

        // Convert rows
        let mut rows = Vec::new();
        for batch in batches {
            for row_idx in 0..batch.num_rows() {
                let mut values = Vec::new();

                for col_idx in 0..batch.num_columns() {
                    let array = batch.column(col_idx);
                    let value = if array.is_null(row_idx) {
                        None
                    } else {
                        Some(array_value_to_string(array.as_ref(), row_idx))
                    };
                    values.push(value);
                }

                rows.push(ResultRow::new(values));
            }
        }

        info!("Converted {} rows from Arrow to MySQL format", rows.len());

        Ok(QueryResult::new_select(columns, rows))
    }

    async fn execute_dml(
        &self,
        query: QueuedQuery,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        info!("Executing DML query: {}", query.query_id);

        // For PoC: Send query to BE via gRPC
        match be_client_pool.execute_query(query.query_id, &query.query).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("BE execution failed: {}, returning mock affected rows", e);
                // Return mock result for PoC
                Ok(QueryResult::new_dml(1))
            }
        }
    }

    async fn execute_ddl(
        &self,
        query: QueuedQuery,
        _be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        info!("Executing DDL query: {}", query.query_id);

        // For PoC: Just return success
        Ok(QueryResult::new_dml(0))
    }

    fn mock_select_result(&self, _query: &str) -> Result<QueryResult> {
        // Simple mock result for demonstration
        let columns = vec![
            ColumnDefinition::new("message".to_string(), ColumnType::VarString),
        ];

        let rows = vec![
            ResultRow::new(vec![
                Some("DataFusion not initialized - use with_datafusion()".to_string()),
            ]),
        ];

        Ok(QueryResult::new_select(columns, rows))
    }

    pub fn queue_stats(&self) -> (usize, usize, usize) {
        (
            self.queue.queue_size(),
            self.queue.available_slots(),
            self.queue.max_concurrent(),
        )
    }
}

/// Convert Arrow DataType to MySQL ColumnType
fn arrow_to_mysql_type(arrow_type: &ArrowDataType) -> ColumnType {
    match arrow_type {
        ArrowDataType::Int8 | ArrowDataType::UInt8 => ColumnType::Tiny,
        ArrowDataType::Int16 | ArrowDataType::UInt16 => ColumnType::Short,
        ArrowDataType::Int32 | ArrowDataType::UInt32 => ColumnType::Long,
        ArrowDataType::Int64 | ArrowDataType::UInt64 => ColumnType::LongLong,
        ArrowDataType::Float32 => ColumnType::Float,
        ArrowDataType::Float64 => ColumnType::Double,
        ArrowDataType::Decimal128(_, _) | ArrowDataType::Decimal256(_, _) => ColumnType::NewDecimal,
        ArrowDataType::Utf8 | ArrowDataType::LargeUtf8 => ColumnType::VarString,
        ArrowDataType::Binary | ArrowDataType::LargeBinary => ColumnType::Blob,
        ArrowDataType::Date32 | ArrowDataType::Date64 => ColumnType::Date,
        ArrowDataType::Timestamp(_, _) => ColumnType::DateTime,
        ArrowDataType::Time32(_) | ArrowDataType::Time64(_) => ColumnType::Time,
        ArrowDataType::Boolean => ColumnType::Tiny,
        _ => ColumnType::VarString, // Default
    }
}

/// Convert Arrow array value to String for MySQL protocol
fn array_value_to_string(array: &dyn Array, idx: usize) -> String {
    match array.data_type() {
        ArrowDataType::Int8 => {
            array.as_any().downcast_ref::<Int8Array>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Int16 => {
            array.as_any().downcast_ref::<Int16Array>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Int32 => {
            array.as_any().downcast_ref::<Int32Array>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Int64 => {
            array.as_any().downcast_ref::<Int64Array>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Float32 => {
            array.as_any().downcast_ref::<Float32Array>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Float64 => {
            array.as_any().downcast_ref::<Float64Array>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Utf8 => {
            array.as_any().downcast_ref::<StringArray>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Boolean => {
            let val = array.as_any().downcast_ref::<BooleanArray>().unwrap().value(idx);
            if val { "1" } else { "0" }.to_string()
        }
        ArrowDataType::Date32 => {
            let days = array.as_any().downcast_ref::<Date32Array>().unwrap().value(idx);
            // Convert days since epoch to date string
            let date = chrono::NaiveDate::from_num_days_from_ce_opt(days + 719163).unwrap();
            date.format("%Y-%m-%d").to_string()
        }
        ArrowDataType::Decimal128(_, scale) => {
            let val = array.as_any().downcast_ref::<Decimal128Array>().unwrap().value(idx);
            let scale = *scale as u32;
            let divisor = 10_i128.pow(scale);
            format!("{}.{:0width$}", val / divisor, (val % divisor).abs(), width = scale as usize)
        }
        _ => {
            // Fallback: try to format as debug
            format!("{:?}", array.slice(idx, 1))
        }
    }
}
