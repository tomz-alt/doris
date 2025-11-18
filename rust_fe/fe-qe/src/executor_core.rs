// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Core SQL Execution Engine
//!
//! CLAUDE.MD Principle #1: "Treat the core FE as: execute_sql + Doris-aware parser
//! + catalog + planner + BE RPC layer"
//!
//! This module provides the main execute_sql interface that orchestrates all components.

use fe_common::{DorisError, Result};
use fe_catalog::Catalog;
use crate::result::QueryResult;
use sqlparser::ast::Statement;
use std::sync::Arc;
use std::time::Instant;

/// Core SQL execution engine
///
/// This is the main entry point following CLAUDE.md architecture.
/// Clean separation: execute_sql → parse → execute → results
pub struct SqlExecutor {
    /// Catalog for metadata
    catalog: Arc<Catalog>,
    /// Query execution metrics
    metrics: ExecutionMetrics,
}

/// Execution metrics for observability (CLAUDE.md Principle #3)
#[derive(Debug, Default, Clone)]
pub struct ExecutionMetrics {
    /// Total queries executed
    pub queries_executed: u64,
    /// Total parse time (microseconds)
    pub total_parse_time_us: u64,
    /// Total execution time (microseconds)
    pub total_exec_time_us: u64,
    /// Parse errors
    pub parse_errors: u64,
    /// Execution errors
    pub exec_errors: u64,
}

impl SqlExecutor {
    /// Create new SQL executor with given catalog
    pub fn new(catalog: Arc<Catalog>) -> Self {
        Self {
            catalog,
            metrics: ExecutionMetrics::default(),
        }
    }

    /// Execute SQL statement - main entry point
    ///
    /// CLAUDE.md Principle #1: Clean core boundary
    /// Flow: SQL → parse → execute → results
    ///
    /// CLAUDE.md Principle #3: Observability - metrics and logging
    pub fn execute_sql(&mut self, sql: &str) -> Result<QueryResult> {
        let query_start = Instant::now();
        self.metrics.queries_executed += 1;

        log::info!("🔍 Executing SQL: {}", sql.trim());

        // Stage 1: Parse SQL (CLAUDE.md: Doris-aware parser)
        let parse_start = Instant::now();
        let ast = self.parse_sql(sql)?;
        let parse_time = parse_start.elapsed();
        self.metrics.total_parse_time_us += parse_time.as_micros() as u64;
        log::debug!("✓ Parsed in {:?}", parse_time);

        // Stage 2: Execute (CLAUDE.md: BE RPC layer abstracted)
        let exec_result = self.execute_statement(&ast)?;
        let total_time = query_start.elapsed();
        self.metrics.total_exec_time_us += total_time.as_micros() as u64;

        log::info!("✅ Query completed in {:?}", total_time);

        Ok(exec_result)
    }

    /// Parse SQL into AST
    ///
    /// CLAUDE.md Principle #3: Graceful error handling
    fn parse_sql(&mut self, sql: &str) -> Result<Statement> {
        use sqlparser::dialect::GenericDialect;
        use sqlparser::parser::Parser;

        let dialect = GenericDialect {};
        let parse_result = Parser::parse_sql(&dialect, sql);

        match parse_result {
            Ok(mut statements) => {
                if statements.is_empty() {
                    self.metrics.parse_errors += 1;
                    return Err(DorisError::ParseError("Empty SQL statement".to_string()));
                }
                if statements.len() > 1 {
                    log::warn!("Multiple statements detected, executing first only");
                }
                Ok(statements.remove(0))
            }
            Err(e) => {
                self.metrics.parse_errors += 1;
                log::error!("Parse error: {}", e);
                Err(DorisError::ParseError(format!("Parse error: {}", e)))
            }
        }
    }

    /// Execute statement
    ///
    /// CLAUDE.md Principle #4: Transport details hidden in QueryExecutor
    fn execute_statement(&mut self, ast: &Statement) -> Result<QueryResult> {
        use crate::executor::QueryExecutor;

        let executor = QueryExecutor::new(Arc::clone(&self.catalog));

        match executor.execute(ast) {
            Ok(result) => Ok(result),
            Err(e) => {
                self.metrics.exec_errors += 1;
                log::error!("Execution error: {}", e);
                Err(e)
            }
        }
    }

    /// Get execution metrics (CLAUDE.md Principle #3: Observability)
    pub fn metrics(&self) -> &ExecutionMetrics {
        &self.metrics
    }
}

impl ExecutionMetrics {
    /// Print metrics summary
    pub fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║  Query Execution Metrics (CLAUDE.md Principle #3)     ║");
        println!("╚════════════════════════════════════════════════════════╝");
        println!("Total queries: {}", self.queries_executed);
        println!("Parse errors:  {}", self.parse_errors);
        println!("Exec errors:   {}", self.exec_errors);
        if self.queries_executed > 0 {
            println!("Avg parse time: {} μs", self.total_parse_time_us / self.queries_executed);
            println!("Avg total time: {} μs", self.total_exec_time_us / self.queries_executed);
        }
        println!("════════════════════════════════════════════════════════\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sql() {
        let catalog = Arc::new(Catalog::new());
        let mut executor = SqlExecutor::new(catalog);

        let result = executor.parse_sql("SELECT 1");
        assert!(result.is_ok());

        let result = executor.parse_sql("");
        assert!(result.is_err());
    }
}
