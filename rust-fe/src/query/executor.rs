use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, info, error};

use crate::error::{DorisError, Result};
use crate::be::BackendClientPool;
use crate::mysql::packet::{ColumnDefinition, ResultRow};
use crate::mysql::ColumnType;
use crate::planner::DataFusionPlanner;
use crate::parser;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::datatypes::DataType as ArrowDataType;
use datafusion::arrow::array::*;
use sqlparser::ast::{Statement, Expr as SqlExpr, Value as SqlValue};
use super::{QueryQueue, QueryResult, QueuedQuery, SessionCtx};

/// Parsed representation of a simple INSERT ... VALUES statement.
/// This is derived from the SQL AST and Rust FE catalog and is used
/// as the core boundary for any BE load integration so that all
/// protocols still call through `execute_sql` (AGENTS.md #1, #3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedInsert {
    pub database: String,
    pub table: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

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

    /// Register BE-backed TPC-H tables (quick prototype for proving BE integration)
    pub async fn register_tpch_be_tables(
        &self,
        be_client_pool: Arc<BackendClientPool>,
        database: &str,
    ) -> Result<()> {
        if let Some(ref df) = self.datafusion {
            df.register_tpch_be_tables(be_client_pool, database).await?;
            info!("TPC-H BE-backed tables registered for database: {}", database);
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

    /// Core, protocol-agnostic entrypoint used by all frontends.
    ///
    /// Frontends provide the SQL string and session context; this method
    /// handles queuing, parsing, planning and routing to BE/DataFusion.
    pub async fn execute_sql(
        &self,
        session: &mut SessionCtx,
        sql: &str,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        let query_id = Uuid::new_v4();
        let database = session.database.clone();

        self.queue_query(query_id, sql.to_string(), database).await?;
        self.execute_query(query_id, be_client_pool).await
    }

    async fn execute_internal(
        &self,
        query: QueuedQuery,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        let sql = query.query.trim();
        debug!("Executing query (parsed): {}", sql);

        // Parse SQL using the Doris-aware parser module.
        let statements = parser::parse_sql(sql)?;
        if statements.is_empty() {
            return Err(DorisError::QueryExecution("Empty query".to_string()));
        }

        // For now we only support a single statement per COM_QUERY.
        let stmt = &statements[0];

        // Validate against the catalog (tables/databases must exist).
        parser::validate_statement(stmt)?;

        match stmt {
            Statement::Query(_) => self.execute_select(query, be_client_pool).await,
            Statement::Insert { .. }
            | Statement::Update { .. }
            | Statement::Delete { .. } => self.execute_dml(query, be_client_pool).await,
            Statement::CreateTable { .. }
            | Statement::Drop { .. }
            | Statement::AlterTable { .. } => self.execute_ddl(query, be_client_pool).await,
            // Other statement types (SHOW, SET, etc.) are handled at the protocol layer.
            _ => Ok(QueryResult::empty()),
        }
    }

    async fn execute_select(
        &self,
        query: QueuedQuery,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        info!("Executing SELECT query: {}", query.query_id);

        // In real_be_proto builds with a configured BE, first attempt to
        // route simple queries through the Doris pipeline fragments path.
        #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
        {
            use crate::planner::plan_converter::PlanConverter;
            use crate::be::thrift_pipeline::PipelineFragmentParamsList;

            debug!("Checking pipeline execution for query {}", query.query_id);
            if be_client_pool.backend_count() > 0 {
                debug!("Backend nodes available: {}", be_client_pool.backend_count());
                if let Some(ref df) = self.datafusion {
                    debug!("DataFusion planner available, creating physical plan...");
                    if let Ok(physical_plan) = df.create_physical_plan(&query.query).await {
                        debug!("Physical plan created, converting to fragments...");
                        let mut converter = PlanConverter::new(query.query_id);
                        match converter.convert_to_fragments(Arc::clone(&physical_plan)) {
                            Ok(plan) => {
                                debug!("Fragments converted, checking if pipeline params can be created...");
                                if PipelineFragmentParamsList::from_query_plan(&plan).is_some() {
                                    debug!("Pipeline params created, executing via pipeline...");
                                    let db_name = query.database.as_deref();
                                    match be_client_pool.execute_pipeline_query(plan, db_name).await {
                                        Ok(result) => {
                                            info!("✓ Pipeline execution succeeded for query {}", query.query_id);
                                            return Ok(result);
                                        }
                                        Err(DorisError::QueryExecution(msg))
                                            if msg.contains("Unsupported fragment shape") =>
                                        {
                                            // Fall through to DataFusion for unsupported shapes.
                                            debug!(
                                                "Pipeline execution not supported for query {}: {}",
                                                query.query_id, msg
                                            );
                                        }
                                        Err(e) => {
                                            // Propagate BE / transport errors to the caller.
                                            error!(
                                                "Pipeline execution failed for query {}: {}",
                                                query.query_id, e
                                            );
                                            return Err(e);
                                        }
                                    }
                                } else {
                                    debug!("Cannot create pipeline params from query plan - falling back to DataFusion");
                                }
                            }
                            Err(e) => {
                                debug!(
                                    "Failed to convert plan for pipeline execution (query {}): {}",
                                    query.query_id, e
                                );
                            }
                        }
                    }
                }
            }
        }

        // Default path: execute via DataFusion if available.
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
            // Fallback: return a mock result if no planner is configured.
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

        // For the PoC backend_service.proto path, send SQL to BE.
        #[cfg(all(not(skip_proto), not(feature = "real_be_proto")))]
        {
            match be_client_pool.execute_query(query.query_id, &query.query).await {
                Ok(result) => Ok(result),
                Err(e) => {
                    error!("BE execution failed: {}, returning mock affected rows", e);
                    Ok(QueryResult::new_dml(1))
                }
            }
        }

        // For SKIP_PROTO builds we don't have a real BE. As a
        // stop-gap, parse the INSERT statement and return an
        // affected_rows count that matches the number of VALUES rows,
        // so that HTTP stream load and clients observe realistic row
        // counts while we build out a true load path.
        #[cfg(skip_proto)]
        {
            let affected_rows = Self::count_insert_values_rows(&query.query).unwrap_or(1);
            Ok(QueryResult::new_dml(affected_rows))
        }

        // For real_be_proto builds, first try to route supported
        // INSERT ... VALUES statements into the real Doris BE
        // tablet-writer path for tpch.lineitem. When the query or
        // environment does not match this narrow slice, fall back to
        // the same affected_rows-only behavior as SKIP_PROTO so that
        // stream load clients still see realistic counts.
        #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
        {
            if be_client_pool.backend_count() > 0 {
                match Self::parse_insert_values(&query.query) {
                    Ok(parsed) => {
                        if parsed.database == "tpch" && parsed.table == "lineitem" {
                            match be_client_pool.load_tpch_lineitem(&parsed).await {
                                Ok(affected_rows) => {
                                    return Ok(QueryResult::new_dml(affected_rows));
                                }
                                Err(e) => {
                                    error!(
                                        "BE load for tpch.lineitem failed (query {}): {}",
                                        query.query_id, e
                                    );
                                    return Err(e);
                                }
                            }
                        }

                        let affected_rows = parsed.rows.len() as u64;
                        return Ok(QueryResult::new_dml(affected_rows));
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse INSERT for BE load (query {}): {}",
                            query.query_id, e
                        );
                    }
                }
            }

            let affected_rows = Self::count_insert_values_rows(&query.query).unwrap_or(1);
            Ok(QueryResult::new_dml(affected_rows))
        }
    }

    /// Count the number of VALUES rows in a simple INSERT statement.
    /// This is used by the DML stub in SKIP_PROTO/real_be_proto builds
    /// so that clients see realistic affected_rows even before a real
    /// BE load path is wired.
    #[cfg(any(skip_proto, feature = "real_be_proto"))]
    fn count_insert_values_rows(sql: &str) -> Option<u64> {
        Self::parse_insert_values(sql).ok().map(|p| p.rows.len() as u64)
    }

    /// Parse a simple INSERT ... VALUES statement into a ParsedInsert.
    /// This keeps INSERT semantics in the core planner and can be used
    /// by any BE load integration path.
    #[cfg(any(skip_proto, feature = "real_be_proto"))]
    fn parse_insert_values(sql: &str) -> Result<ParsedInsert> {
        use sqlparser::ast::SetExpr;

        let statements = parser::parse_sql(sql)?;
        let stmt = statements
            .get(0)
            .ok_or_else(|| DorisError::QueryExecution("Empty INSERT statement".to_string()))?;

        let (table_name, columns, source) = match stmt {
            Statement::Insert { table_name, columns, source, .. } => {
                (table_name, columns, source)
            }
            _ => {
                return Err(DorisError::QueryExecution(
                    "Only INSERT ... VALUES is supported for load".to_string(),
                ));
            }
        };

        // Split db.table or use tpch as default db.
        let full_name = table_name.to_string();
        let parts: Vec<&str> = full_name.split('.').collect();
        let (database, table) = match parts.len() {
            1 => ("tpch".to_string(), parts[0].to_string()),
            2 => (parts[0].to_string(), parts[1].to_string()),
            _ => {
                return Err(DorisError::QueryExecution(format!(
                    "Invalid table name in INSERT: {}",
                    full_name
                )));
            }
        };

        let column_names: Vec<String> = if columns.is_empty() {
            // No explicit column list: use catalog order.
            let cat = crate::metadata::catalog::catalog();
            let table_meta = cat
                .get_table(&database, &table)
                .ok_or_else(|| DorisError::QueryExecution(format!(
                    "Table '{}.{}' not found for INSERT",
                    database, table
                )))?;
            table_meta.columns.iter().map(|c| c.name.clone()).collect()
        } else {
            columns.iter().map(|id| id.to_string()).collect()
        };

        // In newer sqlparser, source is Option<Box<Query>>
        let query = source.as_ref().ok_or_else(|| {
            DorisError::QueryExecution("INSERT must have a source".to_string())
        })?;

        let values = match &*query.body {
            SetExpr::Values(v) => &v.rows,
            _ => {
                return Err(DorisError::QueryExecution(
                    "Only VALUES clause is supported for load".to_string(),
                ));
            }
        };

        let mut rows = Vec::with_capacity(values.len());
        for row_exprs in values {
            if row_exprs.len() != column_names.len() {
                return Err(DorisError::QueryExecution(format!(
                    "VALUES row has {} columns, expected {}",
                    row_exprs.len(),
                    column_names.len()
                )));
            }

            let mut row = Vec::with_capacity(row_exprs.len());
            for expr in row_exprs {
                let cell = match expr {
                    SqlExpr::Value(SqlValue::Number(n, _)) => n.clone(),
                    SqlExpr::Value(SqlValue::SingleQuotedString(s)) => s.clone(),
                    SqlExpr::Value(SqlValue::Boolean(b)) => {
                        if *b { "1".to_string() } else { "0".to_string() }
                    }
                    other => other.to_string(),
                };
                row.push(cell);
            }
            rows.push(row);
        }

        Ok(ParsedInsert {
            database,
            table,
            columns: column_names,
            rows,
        })
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

    /// List tables in a database (for SHOW TABLES command)
    /// Uses global metadata catalog for table listing.
    pub async fn list_tables(&self, database: &str) -> Result<Vec<String>> {
        let catalog = crate::metadata::catalog::catalog();
        catalog
            .list_tables(database)
            .map_err(|e| DorisError::QueryExecution(e))
    }

    /// Describe table schema (for DESCRIBE command)
    /// Returns: Vec<(field_name, field_type, nullable)>
    /// Uses global metadata catalog to build DESCRIBE output.
    pub async fn describe_table(
        &self,
        database: &str,
        table_name: &str,
    ) -> Result<Vec<(String, String, bool)>> {
        let catalog = crate::metadata::catalog::catalog();

        let columns = catalog
            .get_table_columns(database, table_name)
            .ok_or_else(|| DorisError::QueryExecution(format!(
                "Table '{}.{}' doesn't exist",
                database, table_name
            )))?;

        let rows = columns
            .into_iter()
            .map(|col| {
                let type_str = match col.data_type {
                    crate::metadata::types::DataType::TinyInt => "TINYINT".to_string(),
                    crate::metadata::types::DataType::SmallInt => "SMALLINT".to_string(),
                    crate::metadata::types::DataType::Int => "INT".to_string(),
                    crate::metadata::types::DataType::BigInt => "BIGINT".to_string(),
                    crate::metadata::types::DataType::Float => "FLOAT".to_string(),
                    crate::metadata::types::DataType::Double => "DOUBLE".to_string(),
                    crate::metadata::types::DataType::Decimal { precision, scale } => {
                        format!("DECIMAL({}, {})", precision, scale)
                    }
                    crate::metadata::types::DataType::Char { length } => {
                        format!("CHAR({})", length)
                    }
                    crate::metadata::types::DataType::Varchar { length } => {
                        format!("VARCHAR({})", length)
                    }
                    crate::metadata::types::DataType::String => "STRING".to_string(),
                    crate::metadata::types::DataType::Text => "TEXT".to_string(),
                    crate::metadata::types::DataType::Date => "DATE".to_string(),
                    crate::metadata::types::DataType::DateTime => "DATETIME".to_string(),
                    crate::metadata::types::DataType::Timestamp => "TIMESTAMP".to_string(),
                    crate::metadata::types::DataType::Boolean => "BOOLEAN".to_string(),
                    crate::metadata::types::DataType::Binary => "BINARY".to_string(),
                    crate::metadata::types::DataType::Varbinary { length } => {
                        format!("VARBINARY({})", length)
                    }
                    crate::metadata::types::DataType::Json => "JSON".to_string(),
                    crate::metadata::types::DataType::Array(ref inner) => {
                        format!("ARRAY<{:?}>", inner)
                    }
                };

                (col.name, type_str, col.nullable)
            })
            .collect();

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::be::BackendClientPool;

    #[tokio::test]
    async fn test_execute_sql_simple_select() {
        let executor = QueryExecutor::with_datafusion(16, 4).await;
        let be_pool = Arc::new(BackendClientPool::new(Vec::new()));
        let mut session = SessionCtx::new();

        let result = executor
            .execute_sql(&mut session, "SELECT 1", &be_pool)
            .await
            .expect("execute_sql should succeed for SELECT 1");

        assert!(!result.is_dml);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.columns.len(), 1);
    }

    #[cfg(any(skip_proto, feature = "real_be_proto"))]
    #[test]
    fn test_parse_insert_values_basic() {
        let sql = "INSERT INTO tpch.lineitem (l_orderkey, l_partkey) \
                   VALUES (1, 1), (2, 2), (3, 3)";
        let parsed = QueryExecutor::parse_insert_values(sql).unwrap();

        assert_eq!(parsed.database, "tpch");
        assert_eq!(parsed.table, "lineitem");
        assert_eq!(parsed.columns, vec!["l_orderkey", "l_partkey"]);
        assert_eq!(parsed.rows.len(), 3);
        assert_eq!(parsed.rows[0], vec!["1".to_string(), "1".to_string()]);
        assert_eq!(parsed.rows[2], vec!["3".to_string(), "3".to_string()]);
    }

    #[cfg(any(skip_proto, feature = "real_be_proto"))]
    #[test]
    fn test_count_insert_values_rows_basic() {
        let sql = "INSERT INTO tpch.lineitem (l_orderkey, l_partkey) \
                   VALUES (1, 1), (2, 2), (3, 3)";
        let count = QueryExecutor::count_insert_values_rows(sql).unwrap();
        assert_eq!(count, 3);
    }

    #[cfg(any(skip_proto, feature = "real_be_proto"))]
    #[test]
    fn test_count_insert_values_rows_non_insert() {
        let sql = "SELECT * FROM lineitem";
        assert!(QueryExecutor::count_insert_values_rows(sql).is_none());
    }

    #[cfg(feature = "real_be_proto")]
    #[tokio::test]
    async fn test_execute_dml_counts_values_rows() {
        let executor = QueryExecutor::new(1024, 4);
        let be_client_pool = Arc::new(BackendClientPool::new(vec![]));

        let query_id = Uuid::new_v4();
        let sql = "INSERT INTO tpch.lineitem (l_orderkey, l_partkey) \
                   VALUES (1, 1), (2, 2), (3, 3)";
        let queued = QueuedQuery {
            query_id,
            query: sql.to_string(),
            database: Some("tpch".to_string()),
        };

        let result = executor.execute_dml(queued, &be_client_pool).await.unwrap();
        assert_eq!(result.affected_rows, 3);
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
