// BE-backed table provider
// Routes queries to Doris BE instead of local DataFusion execution

use async_trait::async_trait;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::{Session, TableProvider};
use datafusion::datasource::TableType;
use datafusion::error::{DataFusionError, Result as DFResult};
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::logical_expr::{Expr, TableProviderFilterPushDown};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs,
    ExecutionMode,
    ExecutionPlan,
    Partitioning,
    PlanProperties,
};
use futures::Stream;
use std::any::Any;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{info, warn};

use crate::be::BackendClientPool;
use crate::error::{DorisError, Result};

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

/// Execution plan that triggers a BE pipeline fragment request for a
/// simple scan and currently returns an empty batch. This is intended
/// for early validation of the Thrift encoder and BE integration.
#[derive(Debug)]
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
pub struct BEScanExec {
    schema: SchemaRef,
    plan_properties: PlanProperties,
    be_client_pool: Arc<BackendClientPool>,
    database: String,
    table_name: String,
    projection: Option<Vec<usize>>,
    filters: Vec<Expr>,
    limit: Option<usize>,
}

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
impl BEScanExec {
    fn new(
        schema: SchemaRef,
        be_client_pool: Arc<BackendClientPool>,
        database: String,
        table_name: String,
        projection: Option<Vec<usize>>,
        filters: Vec<Expr>,
        limit: Option<usize>,
    ) -> Self {
        // Apply simple projection at the schema level
        let output_schema = match projection.as_ref() {
            None => schema.clone(),
            Some(cols) => Arc::new(schema.project(cols).expect("valid projection")),
        };

        let plan_properties = PlanProperties::new(
            EquivalenceProperties::new(output_schema.clone()),
            Partitioning::UnknownPartitioning(1),
            ExecutionMode::Bounded,
        );

        Self {
            schema: output_schema,
            plan_properties,
            be_client_pool,
            database,
            table_name,
            projection,
            filters,
            limit,
        }
    }

    pub fn table_name(&self) -> &str {
        &self.table_name
    }

    pub fn database(&self) -> &str {
        &self.database
    }
}

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
impl ExecutionPlan for BEScanExec {
    fn name(&self) -> &str {
        "BEScanExec"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }

    fn with_new_children(
        self: Arc<Self>,
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> DFResult<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }

    fn properties(&self) -> &PlanProperties {
        &self.plan_properties
    }

    fn execute(
        &self,
        _partition: usize,
        _context: Arc<TaskContext>,
    ) -> DFResult<SendableRecordBatchStream> {
        // Lazily call BE when the stream is polled; currently we only
        // support a simple TPCH lineitem scan and return an empty
        // batch after the BE call completes.
        let schema = self.schema.clone();
        let pool = self.be_client_pool.clone();
        let database = self.database.clone();
        let table_name = self.table_name.clone();
        let projection = self.projection.clone();
        let filters = self.filters.clone();
        let limit = self.limit;

        let fut = get_be_batch(schema.clone(), pool, database, table_name, projection, filters, limit);
        let stream = futures::stream::once(fut);

        Ok(Box::pin(RecordBatchStreamAdapter::new(
            schema,
            stream,
        )))
    }
}

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
impl DisplayAs for BEScanExec {
    fn fmt_as(
        &self,
        _t: datafusion::physical_plan::DisplayFormatType,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        write!(
            f,
            "BEScanExec table: {}.{} projection: {:?} limit: {:?}",
            self.database, self.table_name, self.projection, self.limit
        )
    }
}

/// Simple BE call for early validation: for now we only support
/// `tpch.lineitem` and ignore filters. We send a pipeline fragment
/// request to BE and, on success, return an empty batch so the query
/// can complete.
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
async fn get_be_batch(
    schema: SchemaRef,
    be_client_pool: Arc<BackendClientPool>,
    database: String,
    table_name: String,
    _projection: Option<Vec<usize>>,
    _filters: Vec<Expr>,
    _limit: Option<usize>,
) -> DFResult<RecordBatch> {
    if database == "tpch" && table_name == "lineitem" {
        match be_client_pool.execute_tpch_lineitem_scan_fragments().await {
            Ok(_res) => {
                // BE accepted the pipeline fragments request. Result
                // fetching is not yet wired, so return an empty batch.
                Ok(RecordBatch::new_empty(schema))
            }
            Err(e) => Err(DataFusionError::External(Box::new(e))),
        }
    } else {
        Err(DataFusionError::NotImplemented(format!(
            "BE scan execution not implemented yet for table {}.{}",
            database, table_name
        )))
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

        #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
        {
            let proj_vec = projection.cloned();
            let exec = BEScanExec::new(
                self.schema.clone(),
                self.be_client_pool.clone(),
                self.database.clone(),
                self.table_name.clone(),
                proj_vec,
                filters.to_vec(),
                limit,
            );
            return Ok(Arc::new(exec));
        }

        #[cfg(not(all(not(skip_proto), feature = "real_be_proto")))]
        {
            // For non-real_be_proto builds, keep prior NotImplemented behavior.
            warn!(
                "BE table scan not yet implemented. Table: {}",
                self.qualified_name()
            );
            warn!("Quick prototype: This table exists in BE but Rust FE can't query it yet");
            warn!("Next step: Enable real_be_proto and implement BEScanExec to route query to BE via gRPC");

            return Err(datafusion::error::DataFusionError::NotImplemented(
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
            ));
        }
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
