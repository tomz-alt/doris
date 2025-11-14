use sqlparser::ast::{Statement, Select, SelectItem, TableFactor, Expr};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use tracing::{debug, warn};

use crate::error::{DorisError, Result};
use crate::metadata::catalog::catalog;

pub fn parse_sql(sql: &str) -> Result<Vec<Statement>> {
    debug!("Parsing SQL: {}", sql.trim());

    let dialect = GenericDialect {};

    Parser::parse_sql(&dialect, sql)
        .map_err(|e| DorisError::QueryExecution(format!("SQL parse error: {}", e)))
}

pub fn validate_statement(stmt: &Statement) -> Result<()> {
    match stmt {
        Statement::Query(query) => validate_query(query.as_ref()),
        Statement::Insert { table_name, .. } => validate_table_reference(&table_name.to_string()),
        Statement::CreateTable { name, .. } => validate_create_table(&name.to_string()),
        Statement::Drop { names, .. } => {
            for name in names {
                validate_table_reference(&name.to_string())?;
            }
            Ok(())
        }
        _ => {
            warn!("Statement type not validated: {:?}", stmt);
            Ok(()) // Allow other statement types for now
        }
    }
}

fn validate_query(query: &sqlparser::ast::Query) -> Result<()> {
    if let sqlparser::ast::SetExpr::Select(ref select) = *query.body {
        validate_select(select)?;
    }

    Ok(())
}

fn validate_select(select: &Select) -> Result<()> {
    debug!("Validating SELECT statement");

    // Validate FROM clause
    for table_with_joins in &select.from {
        validate_table_factor(&table_with_joins.relation)?;

        for join in &table_with_joins.joins {
            validate_table_factor(&join.relation)?;
        }
    }

    // Validate SELECT items
    for item in &select.projection {
        match item {
            SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => {
                validate_expr(expr)?;
            }
            SelectItem::QualifiedWildcard(name, _) => {
                // Validate table exists
                validate_table_reference(&name.to_string())?;
            }
            SelectItem::Wildcard(_) => {
                // Wildcard is always valid if FROM clause is valid
            }
        }
    }

    // Validate WHERE clause
    if let Some(ref where_clause) = select.selection {
        validate_expr(where_clause)?;
    }

    Ok(())
}

fn validate_table_factor(factor: &TableFactor) -> Result<()> {
    match factor {
        TableFactor::Table { name, .. } => {
            let table_name = name.to_string();
            validate_table_reference(&table_name)?;
        }
        TableFactor::Derived { .. } => {
            // Subqueries are allowed
        }
        TableFactor::TableFunction { .. } => {
            // Table functions are allowed
        }
        _ => {}
    }

    Ok(())
}

fn validate_table_reference(full_name: &str) -> Result<()> {
    debug!("Validating table reference: {}", full_name);

    // Parse table name (might be db.table or just table)
    let parts: Vec<&str> = full_name.split('.').collect();

    let (db_name, table_name) = match parts.len() {
        1 => {
            // Just table name, assume current database (tpch for now)
            ("tpch", parts[0])
        }
        2 => {
            // db.table
            (parts[0], parts[1])
        }
        _ => {
            return Err(DorisError::QueryExecution(
                format!("Invalid table name: {}", full_name)
            ));
        }
    };

    // Check if table exists in catalog
    let catalog = catalog();

    if !catalog.table_exists(db_name, table_name) {
        return Err(DorisError::QueryExecution(
            format!("Table '{}' does not exist in database '{}'", table_name, db_name)
        ));
    }

    Ok(())
}

fn validate_create_table(table_name: &str) -> Result<()> {
    debug!("Validating CREATE TABLE: {}", table_name);

    // Parse table name
    let parts: Vec<&str> = table_name.split('.').collect();
    let (db_name, _) = match parts.len() {
        1 => ("tpch", parts[0]),
        2 => (parts[0], parts[1]),
        _ => {
            return Err(DorisError::QueryExecution(
                format!("Invalid table name: {}", table_name)
            ));
        }
    };

    // Check if database exists
    let catalog = catalog();

    if !catalog.database_exists(db_name) {
        return Err(DorisError::QueryExecution(
            format!("Database '{}' does not exist", db_name)
        ));
    }

    Ok(())
}

fn validate_expr(_expr: &Expr) -> Result<()> {
    // For now, we'll allow all expressions
    // In a full implementation, we'd validate:
    // - Column references exist in tables
    // - Function calls are valid
    // - Type compatibility
    Ok(())
}

/// Extract table names from a query
pub fn extract_table_names(stmt: &Statement) -> Vec<String> {
    let mut tables = Vec::new();

    if let Statement::Query(query) = stmt {
        if let sqlparser::ast::SetExpr::Select(ref select) = *query.body {
            for table_with_joins in &select.from {
                if let TableFactor::Table { name, .. } = &table_with_joins.relation {
                    tables.push(name.to_string());
                }

                for join in &table_with_joins.joins {
                    if let TableFactor::Table { name, .. } = &join.relation {
                        tables.push(name.to_string());
                    }
                }
            }
        }
    }

    tables
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let sql = "SELECT * FROM lineitem LIMIT 10";
        let statements = parse_sql(sql).unwrap();
        assert_eq!(statements.len(), 1);
    }

    #[test]
    fn test_parse_tpch_q1() {
        let sql = r#"
            SELECT
                l_returnflag,
                l_linestatus,
                SUM(l_quantity) AS sum_qty,
                COUNT(*) AS count_order
            FROM
                lineitem
            WHERE
                l_shipdate <= DATE '1998-12-01'
            GROUP BY
                l_returnflag,
                l_linestatus
            ORDER BY
                l_returnflag,
                l_linestatus
        "#;

        let statements = parse_sql(sql).unwrap();
        assert_eq!(statements.len(), 1);

        // Validate against catalog
        validate_statement(&statements[0]).unwrap();
    }

    #[test]
    fn test_extract_table_names() {
        let sql = "SELECT * FROM lineitem WHERE l_quantity > 10";
        let statements = parse_sql(sql).unwrap();
        let tables = extract_table_names(&statements[0]);

        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0], "lineitem");
    }
}
