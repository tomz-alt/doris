/// Direct DataFusion query test - bypasses MySQL protocol
/// Tests that DataFusion can execute TPC-H queries on loaded data
/// Usage: TPCH_DATA_DIR=/home/user/doris/tpch-data cargo run --example datafusion_test

use datafusion::prelude::*;
use datafusion::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== DataFusion TPC-H Query Test ===\n");

    // Create DataFusion session
    let ctx = SessionContext::new();

    // Get TPC-H data directory from env
    let data_dir = std::env::var("TPCH_DATA_DIR")
        .unwrap_or_else(|_| "/home/user/doris/tpch-data".to_string());

    println!("Loading TPC-H data from: {}\n", data_dir);

    // Register TPC-H tables
    let tables = vec![
        ("nation", vec!["n_nationkey", "n_name", "n_regionkey", "n_comment"]),
        ("region", vec!["r_regionkey", "r_name", "r_comment"]),
        ("customer", vec!["c_custkey", "c_name", "c_address", "c_nationkey", "c_phone", "c_acctbal", "c_mktsegment", "c_comment"]),
        ("orders", vec!["o_orderkey", "o_custkey", "o_orderstatus", "o_totalprice", "o_orderdate", "o_orderpriority", "o_clerk", "o_shippriority", "o_comment"]),
        ("lineitem", vec![
            "l_orderkey", "l_partkey", "l_suppkey", "l_linenumber",
            "l_quantity", "l_extendedprice", "l_discount", "l_tax",
            "l_returnflag", "l_linestatus", "l_shipdate", "l_commitdate",
            "l_receiptdate", "l_shipinstruct", "l_shipmode", "l_comment"
        ]),
    ];

    for (table_name, _columns) in &tables {
        let file_path = format!("{}/{}.tbl", data_dir, table_name);

        // Register CSV with | delimiter (TPC-H format)
        let csv_options = CsvReadOptions::new()
            .delimiter(b'|')
            .has_header(false)
            .file_extension(".tbl");

        match ctx.register_csv(*table_name, &file_path, csv_options).await {
            Ok(_) => println!("✓ Registered table: {}", table_name),
            Err(e) => println!("✗ Failed to register {}: {}", table_name, e),
        }
    }

    println!("\n=== Running Test Queries ===\n");

    // Test 1: Count rows in nation table
    println!("Query 1: SELECT COUNT(*) FROM nation;");
    let df = ctx.sql("SELECT COUNT(*) as count FROM nation").await?;
    df.show().await?;

    // Test 2: Sample nation data
    println!("\nQuery 2: SELECT * FROM nation LIMIT 5;");
    let df = ctx.sql("SELECT * FROM nation LIMIT 5").await?;
    df.show().await?;

    // Test 3: Count lineitem rows
    println!("\nQuery 3: SELECT COUNT(*) FROM lineitem;");
    let df = ctx.sql("SELECT COUNT(*) as count FROM lineitem").await?;
    df.show().await?;

    // Test 4: Simple aggregation (DataFusion auto-names columns as column_1, column_2, etc)
    println!("\nQuery 4: SELECT l_returnflag, COUNT(*) as count FROM lineitem GROUP BY l_returnflag;");
    let df = ctx.sql(
        "SELECT column_9 as l_returnflag, COUNT(*) as count FROM lineitem GROUP BY column_9"
    ).await?;
    df.show().await?;

    // Test 5: TPC-H Query 1 (simplified)
    println!("\nQuery 5: TPC-H Q1 (simplified) - Pricing Summary Report");
    let sql = r#"
        SELECT
            column_9 as l_returnflag,
            column_10 as l_linestatus,
            COUNT(*) as count_order,
            SUM(CAST(column_5 AS DOUBLE)) as sum_qty,
            SUM(CAST(column_6 AS DOUBLE)) as sum_base_price,
            AVG(CAST(column_6 AS DOUBLE)) as avg_price
        FROM lineitem
        WHERE column_11 <= '1998-12-01'
        GROUP BY column_9, column_10
        ORDER BY column_9, column_10
        LIMIT 10
    "#;

    let df = ctx.sql(sql).await?;
    df.show().await?;

    println!("\n=== All Tests Complete! ===");
    println!("✓ DataFusion can execute TPC-H queries successfully");
    println!("✓ CSV data loaded and queryable");

    Ok(())
}
