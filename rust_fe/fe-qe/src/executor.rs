// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Query Executor
//!
//! Executes SQL statements and returns results.

use fe_common::{Result, DorisError, DataType};
use fe_catalog::{Catalog, OlapTable, Column};
use fe_analysis::ast::*;
use crate::result::{QueryResult, ResultSet};
use std::sync::Arc;

pub struct QueryExecutor {
    catalog: Arc<Catalog>,
}

impl QueryExecutor {
    pub fn new(catalog: Arc<Catalog>) -> Self {
        Self { catalog }
    }

    /// Execute a SQL statement
    pub fn execute(&self, stmt: &Statement) -> Result<QueryResult> {
        match stmt {
            Statement::CreateTable(create) => self.execute_create_table(create),
            Statement::DropTable(drop) => self.execute_drop_table(drop),
            Statement::Select(select) => self.execute_select(select),
            Statement::Insert(_) => Ok(QueryResult::ok("INSERT not yet implemented")),
            Statement::Update(_) => Ok(QueryResult::ok("UPDATE not yet implemented")),
            Statement::Delete(_) => Ok(QueryResult::ok("DELETE not yet implemented")),
            Statement::CreateDatabase(create) => self.execute_create_database(create),
            Statement::DropDatabase(drop) => self.execute_drop_database(drop),
            Statement::Use(use_stmt) => self.execute_use(use_stmt),
            Statement::Show(_) => Ok(QueryResult::ok("SHOW not yet implemented")),
            Statement::Set(_) => Ok(QueryResult::ok("SET not yet implemented")),
        }
    }

    fn execute_create_table(&self, stmt: &CreateTableStatement) -> Result<QueryResult> {
        // Parse table name (may include database)
        let parts: Vec<&str> = stmt.table_name.split('.').collect();
        let (db_name, table_name) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            ("default", parts[0])
        };

        // Convert parser columns to catalog columns
        let mut columns = Vec::new();
        for (i, col_def) in stmt.columns.iter().enumerate() {
            let data_type = Self::parse_data_type(&col_def.data_type)?;
            let column = Column::new((i + 1) as i32, col_def.name.clone(), data_type);
            columns.push(column);
        }

        if columns.is_empty() {
            return Err(DorisError::AnalysisError("Table must have at least one column".to_string()));
        }

        // Create table in catalog
        // For now, use DUPLICATE KEY by default
        let table = OlapTable::new(
            self.catalog.list_databases().len() as i64 + 1, // Simple ID allocation
            table_name.to_string(),
            1, // db_id
            fe_common::KeysType::DupKeys,
            columns,
        );

        self.catalog.create_table(db_name, table)?;

        Ok(QueryResult::ok(format!("Table {} created", stmt.table_name)))
    }

    fn execute_drop_table(&self, stmt: &DropTableStatement) -> Result<QueryResult> {
        let parts: Vec<&str> = stmt.table_name.split('.').collect();
        let (db_name, table_name) = if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            ("default", parts[0])
        };

        self.catalog.drop_table(db_name, table_name)?;
        Ok(QueryResult::ok(format!("Table {} dropped", stmt.table_name)))
    }

    fn execute_create_database(&self, stmt: &CreateDatabaseStatement) -> Result<QueryResult> {
        self.catalog.create_database(stmt.database_name.clone(), "default_cluster".to_string())?;
        Ok(QueryResult::ok(format!("Database {} created", stmt.database_name)))
    }

    fn execute_drop_database(&self, stmt: &DropDatabaseStatement) -> Result<QueryResult> {
        self.catalog.drop_database(&stmt.database_name)?;
        Ok(QueryResult::ok(format!("Database {} dropped", stmt.database_name)))
    }

    fn execute_use(&self, stmt: &UseStatement) -> Result<QueryResult> {
        // Verify database exists
        self.catalog.get_database(&stmt.database_name)?;
        Ok(QueryResult::ok(format!("Database changed to {}", stmt.database_name)))
    }

    fn execute_select(&self, stmt: &SelectStatement) -> Result<QueryResult> {
        // Parse the SQL query to extract table and columns
        // For now, we return proper column structure but no data
        use sqlparser::parser::Parser;
        use sqlparser::dialect::GenericDialect;

        let dialect = GenericDialect {};
        let ast = Parser::parse_sql(&dialect, &stmt.query)
            .map_err(|e| DorisError::AnalysisError(format!("Failed to parse query: {}", e)))?;

        if ast.is_empty() {
            return Err(DorisError::AnalysisError("Empty query".to_string()));
        }

        let query = match &ast[0] {
            sqlparser::ast::Statement::Query(q) => q,
            _ => return Err(DorisError::AnalysisError("Not a SELECT query".to_string())),
        };

        // Extract columns from SELECT list
        let select = match query.body.as_ref() {
            sqlparser::ast::SetExpr::Select(s) => s,
            _ => return Err(DorisError::AnalysisError("Unsupported query type".to_string())),
        };

        let mut columns = Vec::new();

        for item in &select.projection {
            match item {
                sqlparser::ast::SelectItem::UnnamedExpr(expr) => {
                    columns.push(crate::result::ColumnInfo {
                        name: format!("{}", expr),
                        data_type: "VARCHAR".to_string(), // Default type
                    });
                }
                sqlparser::ast::SelectItem::ExprWithAlias { expr, alias } => {
                    columns.push(crate::result::ColumnInfo {
                        name: alias.to_string(),
                        data_type: "VARCHAR".to_string(),
                    });
                }
                sqlparser::ast::SelectItem::Wildcard(_) => {
                    // For *, we would need to fetch table schema
                    // For now, just add a placeholder
                    columns.push(crate::result::ColumnInfo {
                        name: "*".to_string(),
                        data_type: "VARCHAR".to_string(),
                    });
                }
                _ => {}
            }
        }

        Ok(QueryResult::ResultSet(ResultSet {
            columns,
            rows: vec![],
        }))
    }

    /// Parse data type string to DataType enum
    fn parse_data_type(type_str: &str) -> Result<DataType> {
        let upper = type_str.to_uppercase();

        if upper.starts_with("VARCHAR") {
            // Extract length: VARCHAR(100)
            if let Some(start) = upper.find('(') {
                if let Some(end) = upper.find(')') {
                    let len_str = &upper[start+1..end];
                    let len: u32 = len_str.parse()
                        .map_err(|_| DorisError::AnalysisError(format!("Invalid VARCHAR length: {}", len_str)))?;
                    return Ok(DataType::Varchar { len });
                }
            }
            return Ok(DataType::Varchar { len: 65535 }); // Default
        }

        if upper.starts_with("CHAR") {
            if let Some(start) = upper.find('(') {
                if let Some(end) = upper.find(')') {
                    let len_str = &upper[start+1..end];
                    let len: u32 = len_str.parse()
                        .map_err(|_| DorisError::AnalysisError(format!("Invalid CHAR length: {}", len_str)))?;
                    return Ok(DataType::Char { len });
                }
            }
            return Ok(DataType::Char { len: 1 }); // Default
        }

        if upper.starts_with("DECIMAL") {
            // Extract precision and scale: DECIMAL(15,2)
            if let Some(start) = upper.find('(') {
                if let Some(end) = upper.find(')') {
                    let params = &upper[start+1..end];
                    let parts: Vec<&str> = params.split(',').collect();
                    if parts.len() == 2 {
                        let precision: u8 = parts[0].trim().parse()
                            .map_err(|_| DorisError::AnalysisError("Invalid DECIMAL precision".to_string()))?;
                        let scale: u8 = parts[1].trim().parse()
                            .map_err(|_| DorisError::AnalysisError("Invalid DECIMAL scale".to_string()))?;
                        return Ok(DataType::Decimal { precision, scale });
                    }
                }
            }
            return Ok(DataType::Decimal { precision: 10, scale: 0 }); // Default
        }

        // Simple types
        match upper.as_str() {
            "BOOLEAN" | "BOOL" => Ok(DataType::Boolean),
            "TINYINT" => Ok(DataType::TinyInt),
            "SMALLINT" => Ok(DataType::SmallInt),
            "INT" | "INTEGER" => Ok(DataType::Int),
            "BIGINT" => Ok(DataType::BigInt),
            "LARGEINT" => Ok(DataType::LargeInt),
            "FLOAT" => Ok(DataType::Float),
            "DOUBLE" => Ok(DataType::Double),
            "DATE" => Ok(DataType::Date),
            "DATETIME" | "TIMESTAMP" => Ok(DataType::DateTime),
            "STRING" | "TEXT" => Ok(DataType::String),
            "JSON" => Ok(DataType::Json),
            "HLL" => Ok(DataType::Hll),
            "BITMAP" => Ok(DataType::Bitmap),
            _ => Err(DorisError::AnalysisError(format!("Unknown data type: {}", type_str))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fe_analysis::DorisParser;

    #[test]
    fn test_execute_create_table() {
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("test_db".to_string(), "default".to_string()).unwrap();

        let executor = QueryExecutor::new(catalog.clone());

        let sql = "CREATE TABLE test_db.users (id INT, name VARCHAR(50))";
        let stmt = DorisParser::parse_one(sql).unwrap();

        let result = executor.execute(&stmt);
        assert!(result.is_ok(), "Failed to execute CREATE TABLE: {:?}", result.err());

        // Verify table was created
        let table = catalog.get_table_by_name("test_db", "users");
        assert!(table.is_ok(), "Table not found after creation");
    }

    #[test]
    fn test_execute_tpch_lineitem() {
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("tpch".to_string(), "default".to_string()).unwrap();

        let executor = QueryExecutor::new(catalog.clone());

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

        let result = executor.execute(&stmt);
        assert!(result.is_ok(), "Failed to create TPC-H lineitem table: {:?}", result.err());

        // Verify table structure
        let table = catalog.get_table_by_name("tpch", "lineitem").unwrap();
        let table_guard = table.read();
        assert_eq!(table_guard.columns.len(), 16, "Expected 16 columns in lineitem table");
        // Column names are lowercased by SQL parser
        assert_eq!(table_guard.columns[0].name.to_uppercase(), "L_ORDERKEY");
        assert_eq!(table_guard.columns[4].name.to_uppercase(), "L_QUANTITY");
    }

    #[test]
    fn test_parse_data_types() {
        assert_eq!(QueryExecutor::parse_data_type("INT").unwrap(), DataType::Int);
        assert_eq!(QueryExecutor::parse_data_type("VARCHAR(100)").unwrap(), DataType::Varchar { len: 100 });
        assert_eq!(QueryExecutor::parse_data_type("DECIMAL(15,2)").unwrap(), DataType::Decimal { precision: 15, scale: 2 });
        assert_eq!(QueryExecutor::parse_data_type("CHAR(25)").unwrap(), DataType::Char { len: 25 });
        assert_eq!(QueryExecutor::parse_data_type("DATE").unwrap(), DataType::Date);
    }

    #[test]
    fn test_execute_drop_table() {
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("test_db".to_string(), "default".to_string()).unwrap();

        let executor = QueryExecutor::new(catalog.clone());

        // Create table
        let create_sql = "CREATE TABLE test_db.users (id INT)";
        let create_stmt = DorisParser::parse_one(create_sql).unwrap();
        executor.execute(&create_stmt).unwrap();

        // Drop table
        let drop_sql = "DROP TABLE test_db.users";
        let drop_stmt = DorisParser::parse_one(drop_sql).unwrap();
        let result = executor.execute(&drop_stmt);

        assert!(result.is_ok(), "Failed to drop table: {:?}", result.err());

        // Verify table is gone
        let table = catalog.get_table_by_name("test_db", "users");
        assert!(table.is_err(), "Table should not exist after drop");
    }
}
