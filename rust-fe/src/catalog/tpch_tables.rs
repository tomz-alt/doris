// Hardcoded TPC-H table schemas for quick prototype
// This will be replaced with dynamic metadata sync in Phase 2

use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use datafusion::prelude::SessionContext;
use std::sync::Arc;
use tracing::{info, warn};

use super::be_table::BETableProvider;
use crate::be::BackendClientPool;
use crate::error::{DorisError, Result};
use crate::metadata::{catalog, schema::{ColumnDef, Table}, types::DataType as MetaDataType};

/// Helper function to register a table in the metadata catalog from Arrow schema
/// This ensures the Thrift encoder gets correct type mappings
fn register_in_metadata_catalog(database: &str, table_name: &str, arrow_schema: &Schema) -> Result<()> {
    // Create metadata table from Arrow schema
    let mut meta_table = Table::new(table_name.to_string());

    for field in arrow_schema.fields() {
        // Convert Arrow type to metadata type
        let meta_type = match MetaDataType::from_arrow_type(field.data_type()) {
            Some(t) => t,
            None => {
                warn!("Unsupported Arrow type for column '{}': {:?}, using String as fallback",
                      field.name(), field.data_type());
                MetaDataType::String
            }
        };

        let mut col = ColumnDef::new(field.name().clone(), meta_type);
        if !field.is_nullable() {
            col = col.not_null();
        }
        meta_table = meta_table.add_column(col);
    }

    // Get or create database in catalog
    let cat = catalog::catalog();
    if !cat.database_exists(database) {
        cat.add_database(crate::metadata::schema::Database::new(database.to_string()));
        info!("Created database '{}' in metadata catalog", database);
    }

    // Add table to catalog
    cat.add_table(database, meta_table)
        .map_err(|e| DorisError::QueryExecution(format!("Failed to register table in metadata catalog: {}", e)))?;

    info!("Registered table '{}.{}' in metadata catalog with {} columns",
          database, table_name, arrow_schema.fields().len());

    Ok(())
}

/// Register all 8 TPC-H tables with hardcoded schemas
pub async fn register_tpch_tables(
    ctx: &SessionContext,
    be_client_pool: Arc<BackendClientPool>,
    database: &str,
) -> Result<()> {
    info!("Registering hardcoded TPC-H tables for database: {}", database);

    // Register each TPC-H table directly to the session context
    // Table names will be qualified with database prefix (e.g., "tpch_sf1.lineitem")
    register_lineitem(ctx, be_client_pool.clone(), database).await?;
    register_orders(ctx, be_client_pool.clone(), database).await?;
    register_customer(ctx, be_client_pool.clone(), database).await?;
    register_part(ctx, be_client_pool.clone(), database).await?;
    register_partsupp(ctx, be_client_pool.clone(), database).await?;
    register_supplier(ctx, be_client_pool.clone(), database).await?;
    register_nation(ctx, be_client_pool.clone(), database).await?;
    register_region(ctx, be_client_pool.clone(), database).await?;

    info!("Successfully registered all 8 TPC-H tables");
    Ok(())
}

/// LINEITEM table (16 columns, ~6M rows for SF1)
async fn register_lineitem(
    ctx: &SessionContext,
    be_client_pool: Arc<BackendClientPool>,
    database: &str,
) -> Result<()> {
    let schema = Schema::new(vec![
        Field::new("l_orderkey", DataType::Int64, false),
        Field::new("l_partkey", DataType::Int64, false),
        Field::new("l_suppkey", DataType::Int64, false),
        Field::new("l_linenumber", DataType::Int32, false),
        Field::new("l_quantity", DataType::Decimal128(15, 2), false),
        Field::new("l_extendedprice", DataType::Decimal128(15, 2), false),
        Field::new("l_discount", DataType::Decimal128(15, 2), false),
        Field::new("l_tax", DataType::Decimal128(15, 2), false),
        Field::new("l_returnflag", DataType::Utf8, false),
        Field::new("l_linestatus", DataType::Utf8, false),
        Field::new("l_shipdate", DataType::Date32, false),
        Field::new("l_commitdate", DataType::Date32, false),
        Field::new("l_receiptdate", DataType::Date32, false),
        Field::new("l_shipinstruct", DataType::Utf8, false),
        Field::new("l_shipmode", DataType::Utf8, false),
        Field::new("l_comment", DataType::Utf8, false),
    ]);

    let schema_ref = Arc::new(schema.clone());

    // Register in metadata catalog FIRST
    register_in_metadata_catalog(database, "lineitem", &schema)?;

    let table = Arc::new(BETableProvider::new(
        schema_ref,
        database.to_string(),
        "lineitem".to_string(),
        be_client_pool,
    ));

    // Register with simple table name - DataFusion will make it available in default schema
    ctx.register_table("lineitem", table)
        .map_err(|e| DorisError::QueryExecution(format!("Failed to register lineitem table: {}", e)))?;

    info!("Registered table: lineitem (for database: {})", database);
    Ok(())
}

/// ORDERS table (9 columns, ~1.5M rows for SF1)
async fn register_orders(
    ctx: &SessionContext,
    be_client_pool: Arc<BackendClientPool>,
    database: &str,
) -> Result<()> {
    let schema = Schema::new(vec![
        Field::new("o_orderkey", DataType::Int64, false),
        Field::new("o_custkey", DataType::Int64, false),
        Field::new("o_orderstatus", DataType::Utf8, false),
        Field::new("o_totalprice", DataType::Decimal128(15, 2), false),
        Field::new("o_orderdate", DataType::Date32, false),
        Field::new("o_orderpriority", DataType::Utf8, false),
        Field::new("o_clerk", DataType::Utf8, false),
        Field::new("o_shippriority", DataType::Int32, false),
        Field::new("o_comment", DataType::Utf8, false),
    ]);

    let table = Arc::new(BETableProvider::new(
        Arc::new(schema),
        database.to_string(),
        "orders".to_string(),
        be_client_pool,
    ));

    ctx.register_table(
        "orders",
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: orders (for database: {})", database);
    Ok(())
}

/// CUSTOMER table (8 columns, 150K rows for SF1)
async fn register_customer(
    ctx: &SessionContext,
    be_client_pool: Arc<BackendClientPool>,
    database: &str,
) -> Result<()> {
    let schema = Schema::new(vec![
        Field::new("c_custkey", DataType::Int64, false),
        Field::new("c_name", DataType::Utf8, false),
        Field::new("c_address", DataType::Utf8, false),
        Field::new("c_nationkey", DataType::Int32, false),
        Field::new("c_phone", DataType::Utf8, false),
        Field::new("c_acctbal", DataType::Decimal128(15, 2), false),
        Field::new("c_mktsegment", DataType::Utf8, false),
        Field::new("c_comment", DataType::Utf8, false),
    ]);

    let table = Arc::new(BETableProvider::new(
        Arc::new(schema),
        database.to_string(),
        "customer".to_string(),
        be_client_pool,
    ));

    ctx.register_table(
        "customer",
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: customer (for database: {})", database);
    Ok(())
}

/// PART table (9 columns, 200K rows for SF1)
async fn register_part(
    ctx: &SessionContext,
    be_client_pool: Arc<BackendClientPool>,
    database: &str,
) -> Result<()> {
    let schema = Schema::new(vec![
        Field::new("p_partkey", DataType::Int64, false),
        Field::new("p_name", DataType::Utf8, false),
        Field::new("p_mfgr", DataType::Utf8, false),
        Field::new("p_brand", DataType::Utf8, false),
        Field::new("p_type", DataType::Utf8, false),
        Field::new("p_size", DataType::Int32, false),
        Field::new("p_container", DataType::Utf8, false),
        Field::new("p_retailprice", DataType::Decimal128(15, 2), false),
        Field::new("p_comment", DataType::Utf8, false),
    ]);

    let table = Arc::new(BETableProvider::new(
        Arc::new(schema),
        database.to_string(),
        "part".to_string(),
        be_client_pool,
    ));

    ctx.register_table(
        "part",
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: part (for database: {})", database);
    Ok(())
}

/// PARTSUPP table (5 columns, 800K rows for SF1)
async fn register_partsupp(
    ctx: &SessionContext,
    be_client_pool: Arc<BackendClientPool>,
    database: &str,
) -> Result<()> {
    let schema = Schema::new(vec![
        Field::new("ps_partkey", DataType::Int64, false),
        Field::new("ps_suppkey", DataType::Int64, false),
        Field::new("ps_availqty", DataType::Int32, false),
        Field::new("ps_supplycost", DataType::Decimal128(15, 2), false),
        Field::new("ps_comment", DataType::Utf8, false),
    ]);

    let table = Arc::new(BETableProvider::new(
        Arc::new(schema),
        database.to_string(),
        "partsupp".to_string(),
        be_client_pool,
    ));

    ctx.register_table(
        "partsupp",
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: partsupp (for database: {})", database);
    Ok(())
}

/// SUPPLIER table (7 columns, 10K rows for SF1)
async fn register_supplier(
    ctx: &SessionContext,
    be_client_pool: Arc<BackendClientPool>,
    database: &str,
) -> Result<()> {
    let schema = Schema::new(vec![
        Field::new("s_suppkey", DataType::Int64, false),
        Field::new("s_name", DataType::Utf8, false),
        Field::new("s_address", DataType::Utf8, false),
        Field::new("s_nationkey", DataType::Int32, false),
        Field::new("s_phone", DataType::Utf8, false),
        Field::new("s_acctbal", DataType::Decimal128(15, 2), false),
        Field::new("s_comment", DataType::Utf8, false),
    ]);

    let table = Arc::new(BETableProvider::new(
        Arc::new(schema),
        database.to_string(),
        "supplier".to_string(),
        be_client_pool,
    ));

    ctx.register_table(
        "supplier",
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: supplier (for database: {})", database);
    Ok(())
}

/// NATION table (4 columns, 25 rows)
async fn register_nation(
    ctx: &SessionContext,
    be_client_pool: Arc<BackendClientPool>,
    database: &str,
) -> Result<()> {
    let schema = Schema::new(vec![
        Field::new("n_nationkey", DataType::Int32, false),
        Field::new("n_name", DataType::Utf8, false),
        Field::new("n_regionkey", DataType::Int32, false),
        Field::new("n_comment", DataType::Utf8, false),
    ]);

    let table = Arc::new(BETableProvider::new(
        Arc::new(schema),
        database.to_string(),
        "nation".to_string(),
        be_client_pool,
    ));

    ctx.register_table(
        "nation",
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: nation (for database: {})", database);
    Ok(())
}

/// REGION table (3 columns, 5 rows)
async fn register_region(
    ctx: &SessionContext,
    be_client_pool: Arc<BackendClientPool>,
    database: &str,
) -> Result<()> {
    let schema = Schema::new(vec![
        Field::new("r_regionkey", DataType::Int32, false),
        Field::new("r_name", DataType::Utf8, false),
        Field::new("r_comment", DataType::Utf8, false),
    ]);

    let table = Arc::new(BETableProvider::new(
        Arc::new(schema),
        database.to_string(),
        "region".to_string(),
        be_client_pool,
    ));

    ctx.register_table(
        "region",
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: region (for database: {})", database);
    Ok(())
}
