// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! MySQL Server Integration Tests
//!
//! These tests start a real MySQL server and connect with mysql_async client
//! to verify end-to-end functionality.

use fe_mysql_protocol::MysqlServer;
use fe_catalog::Catalog;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use mysql_async::prelude::*;

/// Helper to start server on a unique port for each test
async fn start_test_server(port: u16) -> Arc<Catalog> {
    let catalog = Arc::new(Catalog::new());

    // Create test databases
    catalog.create_database("default".to_string(), "default_cluster".to_string()).unwrap();
    catalog.create_database("test".to_string(), "default_cluster".to_string()).unwrap();
    catalog.create_database("tpch".to_string(), "default_cluster".to_string()).unwrap();

    let server = MysqlServer::new(catalog.clone(), port);

    tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("Server error: {:?}", e);
        }
    });

    // Wait for server to start
    sleep(Duration::from_millis(200)).await;

    catalog
}

#[tokio::test]
async fn test_server_connection() {
    let port = 19030;
    let _catalog = start_test_server(port).await;

    let url = format!("mysql://root@127.0.0.1:{}/test", port);
    let pool = mysql_async::Pool::new(url.as_str());

    let mut conn = pool.get_conn().await.expect("Failed to connect");

    // Simple ping
    conn.ping().await.expect("Ping failed");

    drop(conn);
    pool.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
async fn test_create_table() {
    let port = 19031;
    let catalog = start_test_server(port).await;

    let url = format!("mysql://root@127.0.0.1:{}/test", port);
    let pool = mysql_async::Pool::new(url.as_str());

    let mut conn = pool.get_conn().await.expect("Failed to connect");

    // Create table (explicitly specify database since executor defaults to 'default')
    conn.query_drop("CREATE TABLE test.users (id INT, name VARCHAR(100))")
        .await
        .expect("Failed to create table");

    // Verify table exists in catalog
    let table_arc = catalog.get_table_by_name("test", "users").expect("Table not found");
    let table = table_arc.read();

    assert_eq!(table.name, "users");
    assert_eq!(table.columns.len(), 2);

    drop(conn);
    pool.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
async fn test_switch_database() {
    let port = 19032;
    let _catalog = start_test_server(port).await;

    let url = format!("mysql://root@127.0.0.1:{}/test", port);
    let pool = mysql_async::Pool::new(url.as_str());

    let mut conn = pool.get_conn().await.expect("Failed to connect");

    // Switch to tpch database (COM_INIT_DB)
    conn.query_drop("USE tpch")
        .await
        .expect("Failed to switch database");

    drop(conn);
    pool.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
async fn test_create_tpch_lineitem() {
    let port = 19033;
    let catalog = start_test_server(port).await;

    let url = format!("mysql://root@127.0.0.1:{}/tpch", port);
    let pool = mysql_async::Pool::new(url.as_str());

    let mut conn = pool.get_conn().await.expect("Failed to connect");

    // Create TPC-H lineitem table (explicitly specify database)
    let create_sql = r#"
        CREATE TABLE tpch.lineitem (
            l_orderkey INT,
            l_partkey INT,
            l_suppkey INT,
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

    conn.query_drop(create_sql)
        .await
        .expect("Failed to create lineitem table");

    // Verify table exists
    let table_arc = catalog.get_table_by_name("tpch", "lineitem").expect("Table not found");
    let table = table_arc.read();

    assert_eq!(table.name, "lineitem");
    assert_eq!(table.columns.len(), 16);

    drop(conn);
    pool.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
async fn test_drop_nonexistent_table() {
    let port = 19034;
    let _catalog = start_test_server(port).await;

    let url = format!("mysql://root@127.0.0.1:{}/test", port);
    let pool = mysql_async::Pool::new(url.as_str());

    let mut conn = pool.get_conn().await.expect("Failed to connect");

    // Try to drop non-existent table - should fail
    let result = conn.query_drop("DROP TABLE test.nonexistent").await;

    assert!(result.is_err(), "Should fail for non-existent table");

    drop(conn);
    pool.disconnect().await.expect("Failed to disconnect");
}

#[tokio::test]
async fn test_invalid_sql() {
    let port = 19035;
    let _catalog = start_test_server(port).await;

    let url = format!("mysql://root@127.0.0.1:{}/test", port);
    let pool = mysql_async::Pool::new(url.as_str());

    let mut conn = pool.get_conn().await.expect("Failed to connect");

    // Invalid SQL syntax
    let result = conn.query_drop("INVALID SQL SYNTAX").await;

    assert!(result.is_err(), "Should fail for invalid SQL");

    drop(conn);
    pool.disconnect().await.expect("Failed to disconnect");
}
#[tokio::test]
async fn test_tpch_q1_query() {
    let port = 19036;
    let catalog = start_test_server(port).await;

    let url = format!("mysql://root@127.0.0.1:{}/tpch", port);
    let pool = mysql_async::Pool::new(url.as_str());

    let mut conn = pool.get_conn().await.expect("Failed to connect");

    // Create TPC-H lineitem table
    let create_sql = r#"
        CREATE TABLE tpch.lineitem (
            l_orderkey INT,
            l_partkey INT,
            l_suppkey INT,
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

    conn.query_drop(create_sql)
        .await
        .expect("Failed to create lineitem table");

    // Execute TPC-H Q1 - full query
    let q1_sql = r#"
        SELECT
            l_returnflag,
            l_linestatus,
            sum(l_quantity) as sum_qty,
            sum(l_extendedprice) as sum_base_price,
            sum(l_extendedprice * (1 - l_discount)) as sum_disc_price,
            sum(l_extendedprice * (1 - l_discount) * (1 + l_tax)) as sum_charge,
            avg(l_quantity) as avg_qty,
            avg(l_extendedprice) as avg_price,
            avg(l_discount) as avg_disc,
            count(*) as count_order
        FROM
            lineitem
        WHERE
            l_shipdate <= date '1998-12-01' - interval '90' day
        GROUP BY
            l_returnflag,
            l_linestatus
        ORDER BY
            l_returnflag,
            l_linestatus
    "#;

    // Execute query - should return proper column structure
    let result: Vec<mysql_async::Row> = conn.query(q1_sql)
        .await
        .expect("Failed to execute TPC-H Q1");

    // Should return 0 rows (no data) but proper column structure
    assert_eq!(result.len(), 0, "Should return 0 rows (no data loaded)");

    // Verify table exists
    let table_arc = catalog.get_table_by_name("tpch", "lineitem").expect("Table not found");
    let table = table_arc.read();
    assert_eq!(table.columns.len(), 16);

    drop(conn);
    pool.disconnect().await.expect("Failed to disconnect");
}
