// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Tests for SELECT execution

#[cfg(test)]
mod tests {
    use crate::executor::QueryExecutor;
    use crate::result::QueryResult;
    use fe_analysis::DorisParser;
    use fe_catalog::Catalog;
    use std::sync::Arc;

    #[test]
    fn test_execute_select_simple() {
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("test_db".to_string(), "default".to_string()).unwrap();

        let executor = QueryExecutor::new(catalog.clone());

        // Execute SELECT with explicit columns
        let sql = "SELECT id, name FROM users";
        let stmt = DorisParser::parse_one(sql).unwrap();
        let result = executor.execute(&stmt).unwrap();

        match result {
            QueryResult::ResultSet(rs) => {
                assert_eq!(rs.columns.len(), 2);
                assert_eq!(rs.columns[0].name, "id");
                assert_eq!(rs.columns[1].name, "name");
                assert_eq!(rs.rows.len(), 0); // No data yet
            }
            _ => panic!("Expected ResultSet, got: {:?}", result),
        }
    }

    #[test]
    fn test_execute_select_tpch_q1() {
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("tpch".to_string(), "default".to_string()).unwrap();

        let executor = QueryExecutor::new(catalog.clone());

        // Create lineitem table first
        let create_sql = r#"
            CREATE TABLE tpch.lineitem (
                L_ORDERKEY INTEGER,
                L_PARTKEY INTEGER,
                L_SUPPKEY INTEGER,
                L_LINENUMBER INTEGER,
                L_QUANTITY DECIMAL(15,2),
                L_EXTENDEDPRICE DECIMAL(15,2),
                L_DISCOUNT DECIMAL(15,2),
                L_TAX DECIMAL(15,2),
                L_RETURNFLAG CHAR(1),
                L_LINESTATUS CHAR(1),
                L_SHIPDATE DATE,
                L_COMMITDATE DATE,
                L_RECEIPTDATE DATE,
                L_SHIPINSTRUCT CHAR(25),
                L_SHIPMODE CHAR(10),
                L_COMMENT VARCHAR(44)
            )
        "#;
        let create_stmt = DorisParser::parse_one(create_sql).unwrap();
        executor.execute(&create_stmt).unwrap();

        // Execute TPC-H Q1 query
        let q1_sql = r#"
            select
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
            from
                lineitem
            where
                l_shipdate <= date '1998-12-01' - interval '90' day
            group by
                l_returnflag,
                l_linestatus
            order by
                l_returnflag,
                l_linestatus
        "#;
        let stmt = DorisParser::parse_one(q1_sql).unwrap();
        let result = executor.execute(&stmt).unwrap();

        match result {
            QueryResult::ResultSet(rs) => {
                // Should have 10 columns as per Q1 SELECT list
                assert_eq!(rs.columns.len(), 10, "TPC-H Q1 should return 10 columns");

                // Verify column aliases
                assert_eq!(rs.columns[2].name, "sum_qty");
                assert_eq!(rs.columns[3].name, "sum_base_price");
                assert_eq!(rs.columns[4].name, "sum_disc_price");
                assert_eq!(rs.columns[5].name, "sum_charge");
                assert_eq!(rs.columns[6].name, "avg_qty");
                assert_eq!(rs.columns[7].name, "avg_price");
                assert_eq!(rs.columns[8].name, "avg_disc");
                assert_eq!(rs.columns[9].name, "count_order");

                assert_eq!(rs.rows.len(), 0); // No data execution yet
            }
            _ => panic!("Expected ResultSet, got: {:?}", result),
        }
    }
}
