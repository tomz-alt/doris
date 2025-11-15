// BE-backed table provider
// Routes queries to Doris BE instead of local DataFusion execution

use async_trait::async_trait;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::catalog::{Session, TableProvider};
use datafusion::datasource::TableType;
use datafusion::logical_expr::{Expr, TableProviderFilterPushDown};
use datafusion::physical_plan::ExecutionPlan;
use std::any::Any;
use std::sync::Arc;
use tracing::{info, warn};

use crate::be::BackendClientPool;
use crate::error::Result;

/// Table provider that delegates queries to Doris BE
#[derive(Debug)]
pub struct BETableProvider {
    schema: SchemaRef,
    database: String,
    table_name: String,
    be_client_pool: Arc<BackendClientPool>,
}

impl BETableProvider {
    pub fn new(
        schema: SchemaRef,
        database: String,
        table_name: String,
        be_client_pool: Arc<BackendClientPool>,
    ) -> Self {
        Self {
            schema,
            database,
            table_name,
            be_client_pool,
        }
    }

    /// Get fully qualified table name
    fn qualified_name(&self) -> String {
        format!("{}.{}", self.database, self.table_name)
    }
}

#[async_trait]
impl TableProvider for BETableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> datafusion::error::Result<Vec<TableProviderFilterPushDown>> {
        // We support filter pushdown by converting to SQL WHERE clause
        Ok(vec![TableProviderFilterPushDown::Inexact; filters.len()])
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        info!(
            "BE scan requested for table: {} (projection: {:?}, filters: {:?}, limit: {:?})",
            self.qualified_name(),
            projection,
            filters,
            limit
        );

        // For quick prototype: return error indicating need for BE execution
        // In full implementation, this would create a BEScanExec that queries BE

        warn!(
            "BE table scan not yet implemented. Table: {}",
            self.qualified_name()
        );
        warn!("Quick prototype: This table exists in BE but Rust FE can't query it yet");
        warn!("Next step: Implement BEScanExec to route query to BE via gRPC");

        Err(datafusion::error::DataFusionError::NotImplemented(
            format!(
                "BE scan execution not implemented yet for table {}. \
                 Table exists in BE with {} rows, but query routing needs to be implemented. \
                 See src/catalog/be_table.rs for TODO.",
                self.qualified_name(),
                match self.table_name.as_str() {
                    "lineitem" => "~6M",
                    "orders" => "~1.5M",
                    "customer" => "~150K",
                    "part" => "~200K",
                    "partsupp" => "~800K",
                    "supplier" => "~10K",
                    "nation" => "25",
                    "region" => "5",
                    _ => "unknown",
                }
            )
        ))
    }
}

// TODO: Implement BEScanExec for Phase 2
// This will:
// 1. Convert DataFusion logical plan to SQL
// 2. Send SQL to BE via gRPC
// 3. Stream results back as Arrow RecordBatches
// 4. Return as ExecutionPlan

/*
use datafusion::physical_plan::ExecutionPlan;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;

pub struct BEScanExec {
    schema: SchemaRef,
    qualified_name: String,
    projection: Option<Vec<usize>>,
    filters: Vec<Expr>,
    limit: Option<usize>,
    be_client: Arc<BEClient>,
}

impl BEScanExec {
    fn build_sql(&self) -> String {
        let mut sql = String::from("SELECT ");

        // Projection
        if let Some(proj) = &self.projection {
            let cols: Vec<_> = proj
                .iter()
                .map(|i| self.schema.field(*i).name())
                .collect();
            sql.push_str(&cols.join(", "));
        } else {
            sql.push('*');
        }

        // FROM
        sql.push_str(&format!(" FROM {}", self.qualified_name));

        // WHERE (convert DataFusion Expr to SQL)
        if !self.filters.is_empty() {
            sql.push_str(" WHERE ");
            // TODO: Convert filters to SQL
            sql.push_str("1=1");  // placeholder
        }

        // LIMIT
        if let Some(lim) = self.limit {
            sql.push_str(&format!(" LIMIT {}", lim));
        }

        sql
    }
}

impl ExecutionPlan for BEScanExec {
    // Implementation to execute query via BE
}
*/
