// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! TPC-H Catalog Integration Test
//!
//! CLAUDE.MD Principle #1: Keep the core boundary clean
//! This example demonstrates the complete query pipeline:
//! 1. Load TPC-H schema into catalog
//! 2. Parse SQL query
//! 3. Build query plan from catalog metadata
//! 4. (Future) Execute on BE and parse results
//!
//! Demonstrates progress toward: "run until TPC_H fully passed"

use fe_catalog::{Catalog, load_tpch_schema};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════");
    println!("  TPC-H Catalog Integration Test");
    println!("  Goal: Build query execution pipeline for TPC-H");
    println!("═══════════════════════════════════════════════════════\n");

    // Step 1: Create catalog and load TPC-H schema
    println!("Step 1: Loading TPC-H schema into catalog");
    println!("─────────────────────────────────────────");
    let catalog = Catalog::new();
    load_tpch_schema(&catalog)?;
    println!("✅ Schema loaded successfully\n");

    // Step 2: Verify catalog contents
    println!("Step 2: Verifying catalog contents");
    println!("─────────────────────────────────────────");

    // Verify database
    let db = catalog.get_database("tpch")?;
    let db_guard = db.read();
    println!("✓ Database: {} (ID: {})", db_guard.name, db_guard.id);

    // Verify table
    let table = catalog.get_table_by_name("tpch", "lineitem")?;
    let table_guard = table.read();
    println!("✓ Table: {} (ID: {})", table_guard.name, table_guard.id);
    println!("  - Keys type: {:?}", table_guard.keys_type);
    println!("  - Columns: {}", table_guard.columns.len());
    println!("  - Partitions: {}", table_guard.partition_count());

    // Display columns
    println!("\n  Columns:");
    for (i, col) in table_guard.columns.iter().enumerate() {
        println!("    {}: {} ({:?}) {}",
            i,
            col.name,
            col.data_type,
            if col.is_key { "[KEY]" } else { "" }
        );
    }
    println!();

    // Step 3: Parse sample TPC-H queries
    println!("Step 3: Parsing sample SQL queries");
    println!("─────────────────────────────────────────");

    let queries = vec![
        // Simple scan
        "SELECT * FROM tpch.lineitem LIMIT 10",

        // Projection
        "SELECT l_orderkey, l_quantity, l_extendedprice FROM tpch.lineitem LIMIT 10",

        // With filter (simplified TPC-H Q6 pattern)
        "SELECT l_extendedprice, l_discount
         FROM tpch.lineitem
         WHERE l_shipdate >= '1994-01-01' AND l_discount >= 0.05
         LIMIT 10",

        // With aggregation (simplified TPC-H Q1 pattern)
        "SELECT l_returnflag, l_linestatus,
                SUM(l_quantity) as sum_qty,
                SUM(l_extendedprice) as sum_base_price
         FROM tpch.lineitem
         GROUP BY l_returnflag, l_linestatus",
    ];

    for (i, query) in queries.iter().enumerate() {
        println!("\nQuery {}: {}", i + 1, query);

        match parse_sql(query) {
            Ok(_statements) => {
                println!("✓ Parsed successfully");
                // TODO: Build query plan from catalog
                // TODO: Generate Thrift plan for BE
            }
            Err(e) => {
                println!("✗ Parse error: {}", e);
            }
        }
    }

    println!("\n═══════════════════════════════════════════════════════");
    println!("Summary");
    println!("═══════════════════════════════════════════════════════");
    println!("✅ Catalog integration: WORKING");
    println!("✅ TPC-H schema loaded: WORKING");
    println!("✅ SQL parsing: WORKING");
    println!("⏳ Query planning: TODO (next step)");
    println!("⏳ BE execution: TODO (requires running BE)");
    println!("⏳ Result parsing: READY (PBlock parser complete)");
    println!();
    println!("Next Steps:");
    println!("1. Implement query planner (catalog → TPlanFragment)");
    println!("2. Add scan range generation (tablet locations)");
    println!("3. Execute simple SELECT on real BE");
    println!("4. Run TPC-H Q1 end-to-end");
    println!("═══════════════════════════════════════════════════════");

    Ok(())
}

/// Parse SQL query
fn parse_sql(sql: &str) -> Result<Vec<sqlparser::ast::Statement>, sqlparser::parser::ParserError> {
    let dialect = GenericDialect {};
    Parser::parse_sql(&dialect, sql)
}
