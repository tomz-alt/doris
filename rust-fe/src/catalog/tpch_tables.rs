// Hardcoded TPC-H table schemas for quick prototype
// This will be replaced with dynamic metadata sync in Phase 2

use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use datafusion::prelude::SessionContext;
use std::sync::Arc;
use tracing::info;

use super::be_table::BETableProvider;
use crate::be::BackendClientPool;
use crate::error::{DorisError, Result};

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

    let table = Arc::new(BETableProvider::new(
        Arc::new(schema),
        database.to_string(),
        "lineitem".to_string(),
        be_client_pool,
    ));

    ctx.register_table(
        &format!("{}.lineitem", database),
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register lineitem table: {}", e)))?;

    info!("Registered table: {}.lineitem", database);
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
        &format!("{}.orders", database),
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: {}.orders", database);
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
        &format!("{}.customer", database),
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: {}.customer", database);
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
        &format!("{}.part", database),
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: {}.part", database);
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
        &format!("{}.partsupp", database),
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: {}.partsupp", database);
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
        &format!("{}.supplier", database),
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: {}.supplier", database);
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
        &format!("{}.nation", database),
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: {}.nation", database);
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
        &format!("{}.region", database),
        table,
    ).map_err(|e| DorisError::QueryExecution(format!("Failed to register table: {}", e)))?;

    info!("Registered table: {}.region", database);
    Ok(())
}
