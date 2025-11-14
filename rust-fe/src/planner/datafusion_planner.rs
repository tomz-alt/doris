use datafusion::prelude::*;
use datafusion::error::Result as DFResult;
use datafusion::datasource::empty::EmptyTable;
use datafusion::arrow::datatypes::{DataType as ArrowDataType, Field, Schema as ArrowSchema};
use datafusion::arrow::record_batch::RecordBatch;
use std::sync::Arc;
use tracing::{info, debug};

use crate::metadata::catalog::catalog;
use crate::metadata::schema::Table;
use crate::error::{DorisError, Result};

pub struct DataFusionPlanner {
    ctx: SessionContext,
}

impl DataFusionPlanner {
    pub async fn new() -> Self {
        info!("Initializing DataFusion query planner");

        let config = SessionConfig::new()
            .with_information_schema(true)
            .with_default_catalog_and_schema("tpch", "public");

        let ctx = SessionContext::new_with_config(config);

        // Register TPC-H tables from our metadata catalog
        Self::register_catalog_tables(&ctx).await;

        Self { ctx }
    }

    async fn register_catalog_tables(ctx: &SessionContext) {
        let catalog = catalog();

        info!("Registering tables from metadata catalog");

        // Register tpch database tables
        if let Some(db) = catalog.get_database("tpch") {
            for table_name in db.list_tables() {
                if let Some(table) = db.get_table(&table_name) {
                    debug!("Registering table: {}", table_name);

                    // Convert our table schema to Arrow schema
                    let arrow_schema = table_to_arrow_schema(&table);

                    // Register as empty table (will be populated from CSV or BE)
                    let empty_table = EmptyTable::new(Arc::new(arrow_schema));

                    if let Err(e) = ctx.register_table(&table_name, Arc::new(empty_table)) {
                        tracing::error!("Failed to register table {}: {}", table_name, e);
                    }
                }
            }
        }

        info!("Registered {} tables", catalog.list_tables("tpch").unwrap_or_default().len());
    }

    /// Register TPC-H data from CSV files (for testing without BE)
    pub async fn register_tpch_csv_files(&self, data_dir: &str) -> DFResult<()> {
        info!("Registering TPC-H CSV files from: {}", data_dir);

        let csv_options = CsvReadOptions::new()
            .delimiter(b'|')          // TPC-H uses | delimiter
            .has_header(false)         // .tbl files have no header
            .file_extension(".tbl");   // Look for .tbl files

        let tables = vec![
            "nation", "region", "part", "supplier",
            "partsupp", "customer", "orders", "lineitem"
        ];

        for table in tables {
            let path = format!("{}/{}.tbl", data_dir, table);
            debug!("Registering CSV: {}", path);

            match self.ctx.register_csv(table, &path, csv_options.clone()).await {
                Ok(_) => debug!("Registered table: {}", table),
                Err(e) => tracing::warn!("Failed to register {}: {}", table, e),
            }
        }

        info!("CSV file registration complete");
        Ok(())
    }

    /// Execute a SQL query using DataFusion (Option A - direct execution)
    pub async fn execute_query(&self, sql: &str) -> Result<Vec<RecordBatch>> {
        debug!("Executing query via DataFusion: {}", sql.trim());

        let df = self.ctx.sql(sql).await
            .map_err(|e| DorisError::QueryExecution(format!("DataFusion SQL error: {}", e)))?;

        let batches = df.collect().await
            .map_err(|e| DorisError::QueryExecution(format!("DataFusion execution error: {}", e)))?;

        debug!("Query returned {} batches", batches.len());

        Ok(batches)
    }

    /// Create physical plan from SQL (Option B - for plan conversion to Doris BE)
    pub async fn create_physical_plan(&self, sql: &str) -> Result<std::sync::Arc<dyn datafusion::physical_plan::ExecutionPlan>> {
        debug!("Creating physical plan for: {}", sql);

        let df = self.ctx.sql(sql).await
            .map_err(|e| DorisError::QueryExecution(format!("DataFusion SQL error: {}", e)))?;

        let physical_plan = df.create_physical_plan().await
            .map_err(|e| DorisError::QueryExecution(format!("Failed to create physical plan: {}", e)))?;

        debug!("Physical plan created: {}", physical_plan.name());
        Ok(physical_plan)
    }

    /// Create logical plan from SQL (for inspection/debugging)
    pub async fn create_logical_plan(&self, sql: &str) -> Result<datafusion::logical_expr::LogicalPlan> {
        debug!("Creating logical plan for: {}", sql);

        let df = self.ctx.sql(sql).await
            .map_err(|e| DorisError::QueryExecution(format!("DataFusion SQL error: {}", e)))?;

        Ok(df.logical_plan().clone())
    }

    /// Get the DataFusion context (for advanced usage)
    pub fn context(&self) -> &SessionContext {
        &self.ctx
    }
}

/// Convert our Table schema to Arrow Schema
fn table_to_arrow_schema(table: &Table) -> ArrowSchema {
    let fields: Vec<Field> = table.columns.iter()
        .map(|col| {
            let arrow_type = match &col.data_type {
                crate::metadata::types::DataType::TinyInt => ArrowDataType::Int8,
                crate::metadata::types::DataType::SmallInt => ArrowDataType::Int16,
                crate::metadata::types::DataType::Int => ArrowDataType::Int32,
                crate::metadata::types::DataType::BigInt => ArrowDataType::Int64,
                crate::metadata::types::DataType::Float => ArrowDataType::Float32,
                crate::metadata::types::DataType::Double => ArrowDataType::Float64,
                crate::metadata::types::DataType::Decimal { precision, scale } => {
                    ArrowDataType::Decimal128(*precision, (*scale) as i8)
                }
                crate::metadata::types::DataType::Varchar { .. }
                | crate::metadata::types::DataType::Char { .. }
                | crate::metadata::types::DataType::String
                | crate::metadata::types::DataType::Text => ArrowDataType::Utf8,
                crate::metadata::types::DataType::Date => ArrowDataType::Date32,
                crate::metadata::types::DataType::DateTime => {
                    ArrowDataType::Timestamp(datafusion::arrow::datatypes::TimeUnit::Millisecond, None)
                }
                crate::metadata::types::DataType::Timestamp => {
                    ArrowDataType::Timestamp(datafusion::arrow::datatypes::TimeUnit::Microsecond, None)
                }
                crate::metadata::types::DataType::Boolean => ArrowDataType::Boolean,
                crate::metadata::types::DataType::Binary => ArrowDataType::Binary,
                crate::metadata::types::DataType::Varbinary { .. } => ArrowDataType::Binary,
                crate::metadata::types::DataType::Json => ArrowDataType::Utf8, // JSON as string
                crate::metadata::types::DataType::Array(inner) => {
                    // Simplified: treat arrays as strings for now
                    ArrowDataType::Utf8
                }
            };

            Field::new(&col.name, arrow_type, col.nullable)
        })
        .collect();

    ArrowSchema::new(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_datafusion_init() {
        let planner = DataFusionPlanner::new().await;
        assert!(planner.context().catalog("tpch").is_some());
    }

    #[tokio::test]
    async fn test_simple_query() {
        let planner = DataFusionPlanner::new().await;

        // This will fail because tables are empty, but should parse
        let result = planner.execute_query("SELECT 1 as test").await;
        assert!(result.is_ok());
    }
}
