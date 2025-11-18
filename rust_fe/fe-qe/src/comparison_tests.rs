// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Comparison Tests - Rust FE vs Java FE Behavior
//!
//! These tests verify that Rust FE produces identical behavior to Java FE
//! for operations that don't require C++ BE.
//!
//! Test methodology:
//! 1. Document expected behavior from Java FE
//! 2. Test Rust FE produces same behavior
//! 3. Mark any differences for investigation

use fe_analysis::DorisParser;
use fe_catalog::{Catalog, OlapTable, Column};
use crate::{QueryExecutor, QueryResult};
use fe_common::{DataType, KeysType};
use std::sync::Arc;

/// Test SQL parsing matches Java FE behavior
mod sql_parsing {
    use super::*;

    #[test]
    fn test_simple_select_parsing() {
        // Java FE: Accepts this query
        let sql = "SELECT id, name FROM users WHERE age > 18";
        let result = DorisParser::parse_one(sql);

        // Rust FE should also accept
        assert!(result.is_ok(), "Should parse simple SELECT like Java FE");
    }

    #[test]
    fn test_join_parsing() {
        // Java FE: Accepts JOIN queries
        let sql = "SELECT u.id, o.order_id FROM users u JOIN orders o ON u.id = o.user_id";
        let result = DorisParser::parse_one(sql);

        assert!(result.is_ok(), "Should parse JOIN like Java FE");
    }

    #[test]
    fn test_group_by_parsing() {
        // Java FE: Accepts GROUP BY
        let sql = "SELECT department, COUNT(*) FROM employees GROUP BY department";
        let result = DorisParser::parse_one(sql);

        assert!(result.is_ok(), "Should parse GROUP BY like Java FE");
    }

    #[test]
    fn test_tpch_q1_parsing() {
        // Java FE: Successfully parses TPC-H Q1
        let sql = r#"
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
            FROM lineitem
            WHERE l_shipdate <= date '1998-12-01' - interval '90' day
            GROUP BY l_returnflag, l_linestatus
            ORDER BY l_returnflag, l_linestatus
        "#;

        let result = DorisParser::parse_one(sql);

        assert!(result.is_ok(), "Should parse TPC-H Q1 like Java FE");
    }

    #[test]
    fn test_invalid_sql_rejected() {
        // Java FE: Rejects invalid syntax
        let sql = "SELECT FROM WHERE";
        let result = DorisParser::parse_one(sql);

        assert!(result.is_err(), "Should reject invalid SQL like Java FE");
    }

    #[test]
    fn test_create_table_parsing() {
        // Java FE: Parses CREATE TABLE
        let sql = r#"
            CREATE TABLE users (
                id INT,
                name VARCHAR(100),
                age INT,
                created_at DATETIME
            )
        "#;

        let result = DorisParser::parse_one(sql);

        assert!(result.is_ok(), "Should parse CREATE TABLE like Java FE");
    }

    #[test]
    fn test_tpch_lineitem_parsing() {
        // Java FE: Parses full TPC-H lineitem table
        let sql = r#"
            CREATE TABLE lineitem (
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

        let result = DorisParser::parse_one(sql);

        assert!(result.is_ok(), "Should parse TPC-H lineitem like Java FE");
    }
}

/// Test catalog operations match Java FE behavior
mod catalog_operations {
    use super::*;

    #[test]
    fn test_create_database() {
        // Java FE: Creates database successfully
        let catalog = Arc::new(Catalog::new());

        let result = catalog.create_database(
            "test_db".to_string(),
            "default_cluster".to_string()
        );

        assert!(result.is_ok(), "Should create database like Java FE");
    }

    #[test]
    fn test_create_table() {
        // Java FE: Creates table in database
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("test_db".to_string(), "default_cluster".to_string()).unwrap();

        let columns = vec![
            Column::new(1, "id".to_string(), DataType::Int),
            Column::new(2, "name".to_string(), DataType::Varchar { len: 100 }),
        ];

        let table = OlapTable::new(
            1,
            "users".to_string(),
            1,
            KeysType::DupKeys,
            columns,
        );

        let result = catalog.create_table("test_db", table);

        assert!(result.is_ok(), "Should create table like Java FE");
    }

    #[test]
    fn test_get_table_by_name() {
        // Java FE: Retrieves table by name
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("test_db".to_string(), "default_cluster".to_string()).unwrap();

        let columns = vec![
            Column::new(1, "id".to_string(), DataType::Int),
        ];

        let table = OlapTable::new(1, "users".to_string(), 1, KeysType::DupKeys, columns);
        catalog.create_table("test_db", table).unwrap();

        let result = catalog.get_table_by_name("test_db", "users");

        assert!(result.is_ok(), "Should retrieve table like Java FE");

        let table_arc = result.unwrap();
        let table = table_arc.read();
        assert_eq!(table.name, "users", "Table name should match");
        assert_eq!(table.columns.len(), 1, "Column count should match");
    }

    #[test]
    fn test_drop_table() {
        // Java FE: Drops table successfully
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("test_db".to_string(), "default_cluster".to_string()).unwrap();

        let columns = vec![Column::new(1, "id".to_string(), DataType::Int)];
        let table = OlapTable::new(1, "users".to_string(), 1, KeysType::DupKeys, columns);
        catalog.create_table("test_db", table).unwrap();

        let result = catalog.drop_table("test_db", "users");

        assert!(result.is_ok(), "Should drop table like Java FE");

        // Verify table is gone
        let get_result = catalog.get_table_by_name("test_db", "users");
        assert!(get_result.is_err(), "Table should not exist after drop");
    }

    #[test]
    fn test_tpch_lineitem_table_structure() {
        // Java FE: Creates TPC-H lineitem with 16 columns
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("tpch".to_string(), "default_cluster".to_string()).unwrap();

        let executor = QueryExecutor::new(catalog.clone());

        let sql = r#"
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

        let stmt = DorisParser::parse_one(sql).unwrap();
        let result = executor.execute(&stmt);

        assert!(result.is_ok(), "Should create lineitem table like Java FE");

        // Verify table structure
        let table_arc = catalog.get_table_by_name("tpch", "lineitem").unwrap();
        let table = table_arc.read();

        assert_eq!(table.columns.len(), 16, "Should have 16 columns like Java FE");
        assert_eq!(table.name, "lineitem", "Table name should match");
    }
}

/// Test query executor behavior matches Java FE
mod query_execution {
    use super::*;

    #[test]
    fn test_tpch_q1_schema_extraction() {
        // Java FE: Returns 10 columns for TPC-H Q1
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("tpch".to_string(), "default_cluster".to_string()).unwrap();

        let executor = QueryExecutor::new(catalog.clone());

        // First create lineitem table
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

        let create_stmt = DorisParser::parse_one(create_sql).unwrap();
        executor.execute(&create_stmt).unwrap();

        // Now execute TPC-H Q1
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
            FROM lineitem
            WHERE l_shipdate <= date '1998-12-01' - interval '90' day
            GROUP BY l_returnflag, l_linestatus
            ORDER BY l_returnflag, l_linestatus
        "#;

        let q1_stmt = DorisParser::parse_one(q1_sql).unwrap();
        let result = executor.execute(&q1_stmt).unwrap();

        match result {
            QueryResult::ResultSet(rs) => {
                // Java FE returns 10 columns
                assert_eq!(rs.columns.len(), 10, "Should have 10 columns like Java FE");

                // Verify column names match Java FE
                assert_eq!(rs.columns[0].name, "l_returnflag");
                assert_eq!(rs.columns[1].name, "l_linestatus");
                assert_eq!(rs.columns[2].name, "sum_qty");
                assert_eq!(rs.columns[3].name, "sum_base_price");
                assert_eq!(rs.columns[4].name, "sum_disc_price");
                assert_eq!(rs.columns[5].name, "sum_charge");
                assert_eq!(rs.columns[6].name, "avg_qty");
                assert_eq!(rs.columns[7].name, "avg_price");
                assert_eq!(rs.columns[8].name, "avg_disc");
                assert_eq!(rs.columns[9].name, "count_order");
            }
            _ => panic!("Should return ResultSet like Java FE"),
        }
    }

    #[test]
    fn test_simple_select_schema() {
        // Java FE: Extracts column names from SELECT
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("test_db".to_string(), "default_cluster".to_string()).unwrap();

        let executor = QueryExecutor::new(catalog);

        let sql = "SELECT id, name, age FROM users";
        let stmt = DorisParser::parse_one(sql).unwrap();
        let result = executor.execute(&stmt).unwrap();

        match result {
            QueryResult::ResultSet(rs) => {
                assert_eq!(rs.columns.len(), 3, "Should have 3 columns like Java FE");
                assert_eq!(rs.columns[0].name, "id");
                assert_eq!(rs.columns[1].name, "name");
                assert_eq!(rs.columns[2].name, "age");
            }
            _ => panic!("Should return ResultSet like Java FE"),
        }
    }
}

/// Test error handling matches Java FE
mod error_handling {
    use super::*;

    #[test]
    fn test_drop_nonexistent_table() {
        // Java FE: Returns error when dropping non-existent table
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("test_db".to_string(), "default_cluster".to_string()).unwrap();

        let result = catalog.drop_table("test_db", "nonexistent");

        assert!(result.is_err(), "Should error like Java FE");
    }

    #[test]
    fn test_get_nonexistent_database() {
        // Java FE: Returns error for non-existent database
        let catalog = Arc::new(Catalog::new());

        let result = catalog.get_database("nonexistent");

        assert!(result.is_err(), "Should error like Java FE");
    }

    #[test]
    fn test_create_table_in_nonexistent_database() {
        // Java FE: Returns error when creating table in non-existent database
        let catalog = Arc::new(Catalog::new());

        let columns = vec![Column::new(1, "id".to_string(), DataType::Int)];
        let table = OlapTable::new(1, "users".to_string(), 1, KeysType::DupKeys, columns);

        let result = catalog.create_table("nonexistent_db", table);

        assert!(result.is_err(), "Should error like Java FE");
    }

    #[test]
    fn test_invalid_sql_parse_error() {
        // Java FE: Returns parse error for invalid SQL
        let sql = "INVALID SQL SYNTAX";
        let result = DorisParser::parse_one(sql);

        assert!(result.is_err(), "Should return parse error like Java FE");
    }
}

/// Test data type parsing matches Java FE
mod data_type_parsing {
    use super::*;

    #[test]
    fn test_int_types() {
        // Java FE: Supports INT, TINYINT, SMALLINT, BIGINT
        let sql = r#"
            CREATE TABLE test (
                a TINYINT,
                b SMALLINT,
                c INT,
                d BIGINT
            )
        "#;

        let result = DorisParser::parse_one(sql);

        assert!(result.is_ok(), "Should parse integer types like Java FE");
    }

    #[test]
    fn test_decimal_type() {
        // Java FE: Supports DECIMAL(precision, scale)
        let sql = "CREATE TABLE test (price DECIMAL(15,2))";
        let result = DorisParser::parse_one(sql);

        assert!(result.is_ok(), "Should parse DECIMAL like Java FE");
    }

    #[test]
    fn test_string_types() {
        // Java FE: Supports VARCHAR, CHAR, STRING
        let sql = r#"
            CREATE TABLE test (
                a VARCHAR(100),
                b CHAR(10),
                c STRING
            )
        "#;

        let result = DorisParser::parse_one(sql);

        assert!(result.is_ok(), "Should parse string types like Java FE");
    }

    #[test]
    fn test_datetime_types() {
        // Java FE: Supports DATE, DATETIME
        let sql = "CREATE TABLE test (a DATE, b DATETIME)";
        let result = DorisParser::parse_one(sql);

        assert!(result.is_ok(), "Should parse datetime types like Java FE");
    }
}
