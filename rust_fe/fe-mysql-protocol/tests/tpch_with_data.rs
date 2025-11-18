// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! TPC-H Test with Actual Data
//!
//! This test demonstrates inserting and querying real data using the in-memory datastore.

use fe_catalog::Catalog;
use fe_analysis::parser::DorisParser;
use fe_qe::executor::QueryExecutor;
use fe_qe::result::{QueryResult, Value, Row};
use std::sync::Arc;

/// Create lineitem table
fn create_lineitem_table(catalog: &Arc<Catalog>) {
    let sql = r#"
        CREATE TABLE tpch.lineitem (
            l_orderkey BIGINT,
            l_partkey BIGINT,
            l_suppkey BIGINT,
            l_linenumber INT,
            l_quantity DECIMAL(15,2),
            l_extendedprice DECIMAL(15,2),
            l_discount DECIMAL(15,2),
            l_tax DECIMAL(15,2),
            l_returnflag CHAR(1),
            l_linestatus CHAR(1),
            l_shipdate DATE,
            l_commitdate DATE,
            l_receiptdate DATE,
            l_shipinstruct CHAR(25),
            l_shipmode CHAR(10),
            l_comment VARCHAR(44)
        )
    "#;

    let stmt = DorisParser::parse_one(sql).expect("Failed to parse CREATE TABLE");
    let executor = QueryExecutor::new(catalog.clone());
    executor.execute(&stmt).expect("Failed to create lineitem table");
}

/// Insert sample TPC-H data manually into datastore
fn insert_sample_data(catalog: &Arc<Catalog>) {
    let datastore = catalog.datastore();

    // Sample TPC-H lineitem rows
    let rows = vec![
        vec![
            "1".to_string(),        // l_orderkey
            "155190".to_string(),   // l_partkey
            "7706".to_string(),     // l_suppkey
            "1".to_string(),        // l_linenumber
            "17.00".to_string(),    // l_quantity
            "21168.23".to_string(), // l_extendedprice
            "0.04".to_string(),     // l_discount
            "0.02".to_string(),     // l_tax
            "N".to_string(),        // l_returnflag
            "O".to_string(),        // l_linestatus
            "1996-03-13".to_string(), // l_shipdate
            "1996-02-12".to_string(), // l_commitdate
            "1996-03-22".to_string(), // l_receiptdate
            "DELIVER IN PERSON".to_string(), // l_shipinstruct
            "TRUCK".to_string(),    // l_shipmode
            "regular deposits".to_string(), // l_comment
        ],
        vec![
            "1".to_string(),
            "67310".to_string(),
            "7311".to_string(),
            "2".to_string(),
            "36.00".to_string(),
            "45983.16".to_string(),
            "0.09".to_string(),
            "0.06".to_string(),
            "N".to_string(),
            "O".to_string(),
            "1996-04-12".to_string(),
            "1996-02-28".to_string(),
            "1996-04-20".to_string(),
            "TAKE BACK RETURN".to_string(),
            "MAIL".to_string(),
            "quick packages".to_string(),
        ],
        vec![
            "2".to_string(),
            "106170".to_string(),
            "1191".to_string(),
            "1".to_string(),
            "38.00".to_string(),
            "44694.46".to_string(),
            "0.00".to_string(),
            "0.05".to_string(),
            "N".to_string(),
            "O".to_string(),
            "1997-01-28".to_string(),
            "1997-01-14".to_string(),
            "1997-02-02".to_string(),
            "TAKE BACK RETURN".to_string(),
            "RAIL".to_string(),
            "bold requests".to_string(),
        ],
        vec![
            "3".to_string(),
            "4297".to_string(),
            "1798".to_string(),
            "1".to_string(),
            "45.00".to_string(),
            "54058.05".to_string(),
            "0.06".to_string(),
            "0.00".to_string(),
            "R".to_string(),
            "F".to_string(),
            "1994-02-02".to_string(),
            "1994-01-04".to_string(),
            "1994-02-23".to_string(),
            "NONE".to_string(),
            "AIR".to_string(),
            "final deposits".to_string(),
        ],
    ];

    for row in rows {
        datastore.insert_row("tpch", "lineitem", row);
    }
}

#[tokio::test]
async fn test_insert_and_select_count() {
    println!("\n══════════════════════════════════════════════════════════");
    println!("  TPC-H: Insert and COUNT Test");
    println!("══════════════════════════════════════════════════════════\n");

    // Setup
    let catalog = Arc::new(Catalog::new());
    catalog.create_database("tpch".to_string(), "default_cluster".to_string())
        .expect("Failed to create tpch database");

    create_lineitem_table(&catalog);

    // Insert data
    println!("Step 1: Inserting sample data...");
    insert_sample_data(&catalog);

    let row_count = catalog.datastore().row_count("tpch", "lineitem");
    println!("✅ Inserted {} rows\n", row_count);
    assert_eq!(row_count, 4, "Should have 4 rows");

    // Query to verify data exists
    println!("Step 2: Querying data...");
    let rows = catalog.datastore().get_rows("tpch", "lineitem");

    println!("✅ Retrieved {} rows", rows.len());
    println!("   First row orderkey: {}", rows[0][0]);
    println!("   First row quantity: {}", rows[0][4]);
    println!("   First row returnflag: {}", rows[0][8]);

    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0][0], "1"); // orderkey
    assert_eq!(rows[0][8], "N"); // returnflag

    println!("\n══════════════════════════════════════════════════════════");
    println!("  Test Summary");
    println!("══════════════════════════════════════════════════════════");
    println!("✅ Data insertion: Working");
    println!("✅ Data retrieval: Working");
    println!("✅ Row count: Correct");
    println!("✅ Data integrity: Verified");
    println!("══════════════════════════════════════════════════════════\n");
}

#[tokio::test]
async fn test_select_all_rows() {
    println!("\n══════════════════════════════════════════════════════════");
    println!("  TPC-H: SELECT * Test");
    println!("══════════════════════════════════════════════════════════\n");

    let catalog = Arc::new(Catalog::new());
    catalog.create_database("tpch".to_string(), "default_cluster".to_string())
        .expect("Failed to create tpch database");

    create_lineitem_table(&catalog);
    insert_sample_data(&catalog);

    println!("Querying all rows from lineitem...");
    let rows = catalog.datastore().get_rows("tpch", "lineitem");

    println!("✅ Retrieved {} rows\n", rows.len());

    // Display first few rows
    for (i, row) in rows.iter().take(3).enumerate() {
        println!("Row {}:", i + 1);
        println!("  orderkey={}, quantity={}, extendedprice={}, discount={}",
            row[0], row[4], row[5], row[6]);
        println!("  returnflag={}, linestatus={}, shipdate={}",
            row[8], row[9], row[10]);
    }

    assert_eq!(rows.len(), 4);

    println!("\n══════════════════════════════════════════════════════════");
    println!("  SELECT * Test Complete");
    println!("══════════════════════════════════════════════════════════\n");
}

#[tokio::test]
async fn test_tpch_aggregation_sim() {
    println!("\n══════════════════════════════════════════════════════════");
    println!("  TPC-H: Aggregation Simulation Test");
    println!("══════════════════════════════════════════════════════════\n");

    let catalog = Arc::new(Catalog::new());
    catalog.create_database("tpch".to_string(), "default_cluster".to_string())
        .expect("Failed to create tpch database");

    create_lineitem_table(&catalog);
    insert_sample_data(&catalog);

    println!("Simulating TPC-H Q6 aggregation...");
    println!("Query: SELECT SUM(l_extendedprice * l_discount) AS revenue");
    println!("       FROM lineitem");
    println!("       WHERE l_discount BETWEEN 0.05 AND 0.07\n");

    let rows = catalog.datastore().get_rows("tpch", "lineitem");

    // Manual aggregation simulation
    let mut total_revenue = 0.0;
    let mut qualifying_rows = 0;

    for row in &rows {
        let extendedprice: f64 = row[5].parse().unwrap_or(0.0);
        let discount: f64 = row[6].parse().unwrap_or(0.0);

        if discount >= 0.05 && discount <= 0.07 {
            qualifying_rows += 1;
            total_revenue += extendedprice * discount;
            println!("  ✓ Row qualifies: extendedprice={}, discount={}, contribution={}",
                extendedprice, discount, extendedprice * discount);
        }
    }

    println!("\n✅ Aggregation complete:");
    println!("   Qualifying rows: {}", qualifying_rows);
    println!("   Total revenue: {:.2}", total_revenue);

    assert!(qualifying_rows > 0, "Should have qualifying rows");
    assert!(total_revenue > 0.0, "Revenue should be positive");

    println!("\n══════════════════════════════════════════════════════════");
    println!("  Aggregation Test Complete");
    println!("══════════════════════════════════════════════════════════\n");
}

#[tokio::test]
async fn test_tpch_q1_simulation() {
    println!("\n══════════════════════════════════════════════════════════");
    println!("  TPC-H Q1: GROUP BY Simulation");
    println!("══════════════════════════════════════════════════════════\n");

    let catalog = Arc::new(Catalog::new());
    catalog.create_database("tpch".to_string(), "default_cluster".to_string())
        .expect("Failed to create tpch database");

    create_lineitem_table(&catalog);
    insert_sample_data(&catalog);

    println!("Simulating TPC-H Q1...");
    println!("Query: SELECT l_returnflag, l_linestatus, ");
    println!("              SUM(l_quantity), SUM(l_extendedprice), COUNT(*)");
    println!("       FROM lineitem");
    println!("       GROUP BY l_returnflag, l_linestatus\n");

    let rows = catalog.datastore().get_rows("tpch", "lineitem");

    // Group by returnflag and linestatus
    use std::collections::HashMap;
    let mut groups: HashMap<(String, String), (f64, f64, usize)> = HashMap::new();

    for row in &rows {
        let returnflag = row[8].clone();
        let linestatus = row[9].clone();
        let quantity: f64 = row[4].parse().unwrap_or(0.0);
        let extendedprice: f64 = row[5].parse().unwrap_or(0.0);

        let entry = groups.entry((returnflag, linestatus))
            .or_insert((0.0, 0.0, 0));

        entry.0 += quantity;
        entry.1 += extendedprice;
        entry.2 += 1;
    }

    println!("✅ Aggregation results:\n");
    println!("  RETURNFLAG | LINESTATUS | SUM_QTY | SUM_PRICE | COUNT");
    println!("  -----------|------------|---------|-----------|-------");

    for ((returnflag, linestatus), (sum_qty, sum_price, count)) in &groups {
        println!("  {:^11}| {:^10} | {:>7.2} | {:>9.2} | {:>5}",
            returnflag, linestatus, sum_qty, sum_price, count);
    }

    assert!(groups.len() > 0, "Should have grouped results");

    println!("\n══════════════════════════════════════════════════════════");
    println!("  TPC-H Q1 Simulation Complete");
    println!("══════════════════════════════════════════════════════════\n");
}
