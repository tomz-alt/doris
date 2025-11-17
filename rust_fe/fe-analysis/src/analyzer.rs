// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Semantic Analyzer
//!
//! Performs semantic analysis on parsed SQL statements:
//! - Type checking
//! - Name resolution
//! - Permission checking
//! - Validity checks

use crate::ast::*;
use fe_common::{Result, DorisError};

pub struct Analyzer;

impl Analyzer {
    /// Analyze a statement
    pub fn analyze(stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::CreateTable(create) => Self::analyze_create_table(create),
            Statement::Select(select) => Self::analyze_select(select),
            _ => Ok(()),
        }
    }

    fn analyze_create_table(stmt: &CreateTableStatement) -> Result<()> {
        // Validate table name
        if stmt.table_name.is_empty() {
            return Err(DorisError::AnalysisError("Table name cannot be empty".to_string()));
        }

        // Validate columns
        if stmt.columns.is_empty() {
            return Err(DorisError::AnalysisError("Table must have at least one column".to_string()));
        }

        // Check for duplicate column names
        let mut names = std::collections::HashSet::new();
        for col in &stmt.columns {
            if !names.insert(col.name.clone()) {
                return Err(DorisError::AnalysisError(
                    format!("Duplicate column name: {}", col.name)
                ));
            }
        }

        Ok(())
    }

    fn analyze_select(_stmt: &SelectStatement) -> Result<()> {
        // TODO: Implement SELECT validation
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::DorisParser;

    #[test]
    fn test_analyze_valid_create_table() {
        let sql = "CREATE TABLE users (id INT, name VARCHAR(50))";
        let stmt = DorisParser::parse_one(sql).unwrap();
        let result = Analyzer::analyze(&stmt);
        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_duplicate_columns() {
        let sql = "CREATE TABLE users (id INT, id VARCHAR(50))";
        let stmt = DorisParser::parse_one(sql).unwrap();
        let result = Analyzer::analyze(&stmt);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Duplicate column"));
    }
}
