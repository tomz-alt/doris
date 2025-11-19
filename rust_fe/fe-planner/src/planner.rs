// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Query Planner
//!
//! Converts AST to execution plans that can be sent to BE via Thrift.

use fe_common::{Result, DorisError, DataType};
use fe_catalog::{Catalog, OlapTable};
use fe_analysis::ast::*;
use crate::thrift_plan::*;
use std::sync::Arc;

pub struct QueryPlanner {
    catalog: Arc<Catalog>,
}

impl QueryPlanner {
    pub fn new(catalog: Arc<Catalog>) -> Self {
        Self { catalog }
    }

    /// Plan a SQL statement
    pub fn plan(&self, stmt: &Statement) -> Result<TPlanFragment> {
        match stmt {
            Statement::Select(select) => self.plan_select(select),
            _ => Err(DorisError::AnalysisError(
                "Only SELECT is supported for planning".to_string()
            )),
        }
    }

    /// Plan a SELECT statement
    fn plan_select(&self, _select: &SelectStatement) -> Result<TPlanFragment> {
        // For now, create a simple OLAP scan plan for lineitem table
        // TODO: Parse the actual SELECT statement
        self.plan_lineitem_scan()
    }

    /// Create a simple scan plan for TPC-H lineitem table
    pub fn plan_lineitem_scan(&self) -> Result<TPlanFragment> {
        // Get lineitem table from catalog
        let table = self.catalog.get_table_by_name("tpch", "lineitem")
            .or_else(|_| self.catalog.get_table_by_name("default", "lineitem"))
            .map_err(|_| DorisError::AnalysisError(
                "Table 'lineitem' not found. Create it first.".to_string()
            ))?;

        let table_guard = table.read();

        // Create OLAP scan node
        let scan_node = self.create_olap_scan_node(&table_guard)?;

        // Create plan fragment
        Ok(TPlanFragment {
            plan: TPlan {
                nodes: vec![scan_node],
            },
            partition: TDataPartition {
                partition_type: TPartitionType::Unpartitioned,
                partition_exprs: None,
                partition_infos: None,
            },
        })
    }

    /// Create OLAP scan node for a table
    fn create_olap_scan_node(&self, table: &OlapTable) -> Result<TPlanNode> {
        // Get key columns (for TPC-H lineitem, it's the DUPLICATE KEY columns)
        let key_columns: Vec<&fe_catalog::Column> = table.columns.iter()
            .filter(|col| col.is_key)
            .collect();

        // If no explicit keys, use first column
        let (key_names, key_types) = if key_columns.is_empty() {
            let col = &table.columns[0];
            (vec![col.name.clone()], vec![datatype_to_tprimitive(&col.data_type)])
        } else {
            let names = key_columns.iter().map(|col| col.name.clone()).collect();
            let types = key_columns.iter().map(|col| datatype_to_tprimitive(&col.data_type)).collect();
            (names, types)
        };

        Ok(TPlanNode {
            node_id: 0,
            node_type: TPlanNodeType::OlapScanNode,
            num_children: 0,
            limit: -1,
            row_tuples: vec![0],
            nullable_tuples: vec![false],
            compact_data: true,
            olap_scan_node: Some(TOlapScanNode {
                tuple_id: 0,
                key_column_name: key_names,
                key_column_type: key_types,
                is_preaggregation: true,
                table_name: Some(table.name.clone()),
            }),
        })
    }
}

/// Convert Doris DataType to Thrift TPrimitiveType
fn datatype_to_tprimitive(dt: &DataType) -> TPrimitiveType {
    match dt {
        DataType::Boolean => TPrimitiveType::Boolean,
        DataType::TinyInt => TPrimitiveType::TinyInt,
        DataType::SmallInt => TPrimitiveType::SmallInt,
        DataType::Int => TPrimitiveType::Int,
        DataType::BigInt => TPrimitiveType::BigInt,
        DataType::LargeInt => TPrimitiveType::LargeInt,
        DataType::Float => TPrimitiveType::Float,
        DataType::Double => TPrimitiveType::Double,
        DataType::Decimal { .. } => TPrimitiveType::DecimalV2,
        DataType::Date => TPrimitiveType::Date,
        DataType::DateTime => TPrimitiveType::DateTime,
        DataType::Char { .. } => TPrimitiveType::Char,
        DataType::Varchar { .. } => TPrimitiveType::Varchar,
        DataType::String => TPrimitiveType::String,
        DataType::Hll => TPrimitiveType::Hll,
        DataType::Bitmap => TPrimitiveType::Bitmap,
        DataType::Quantile => TPrimitiveType::QuantileState,
        DataType::Array { .. } => TPrimitiveType::Array,
        DataType::Map { .. } => TPrimitiveType::Map,
        DataType::Struct { .. } => TPrimitiveType::Struct,
        DataType::Json => TPrimitiveType::JsonB,
        DataType::Binary => TPrimitiveType::Binary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fe_analysis::DorisParser;
    use fe_qe::QueryExecutor;

    fn create_lineitem_catalog() -> Arc<Catalog> {
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("tpch".to_string(), "default".to_string()).unwrap();

        // Create TPC-H lineitem table
        let sql = r#"
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

        let stmt = DorisParser::parse_one(sql).unwrap();
        let executor = QueryExecutor::new(catalog.clone());
        executor.execute(&stmt).unwrap();

        catalog
    }

    #[test]
    fn test_plan_lineitem_scan() {
        let catalog = create_lineitem_catalog();
        let planner = QueryPlanner::new(catalog);

        let plan = planner.plan_lineitem_scan().unwrap();

        // Verify plan structure
        assert_eq!(plan.plan.nodes.len(), 1);

        let node = &plan.plan.nodes[0];
        assert_eq!(node.node_id, 0);
        assert_eq!(node.node_type, TPlanNodeType::OlapScanNode);
        assert_eq!(node.num_children, 0);
        assert_eq!(node.row_tuples, vec![0]);

        // Verify OLAP scan node
        let scan = node.olap_scan_node.as_ref().unwrap();
        assert_eq!(scan.tuple_id, 0);
        assert_eq!(scan.is_preaggregation, true);

        // Should have key columns
        assert!(!scan.key_column_name.is_empty());
        println!("Key columns: {:?}", scan.key_column_name);
    }

    #[test]
    fn test_plan_to_json() {
        let catalog = create_lineitem_catalog();
        let planner = QueryPlanner::new(catalog);

        let plan = planner.plan_lineitem_scan().unwrap();
        let json = plan.to_json().unwrap();

        println!("Plan JSON:\n{}", json);

        // Verify JSON structure
        assert!(json.contains("\"node_type\": \"OlapScanNode\""));
        assert!(json.contains("\"tuple_id\": 0"));
        assert!(json.contains("\"is_preaggregation\": true"));

        // Write to file for manual inspection
        std::fs::write("/tmp/rust_plan.json", &json).unwrap();
        println!("\nWrote plan to /tmp/rust_plan.json");
    }

    #[test]
    fn test_plan_select_statement() {
        let catalog = create_lineitem_catalog();
        let planner = QueryPlanner::new(catalog);

        let sql = "SELECT l_orderkey, l_quantity FROM lineitem";
        let stmt = DorisParser::parse_one(sql).unwrap();

        let plan = planner.plan(&stmt).unwrap();

        // Should create OLAP scan node
        assert_eq!(plan.plan.nodes.len(), 1);
        assert_eq!(plan.plan.nodes[0].node_type, TPlanNodeType::OlapScanNode);
    }
}
