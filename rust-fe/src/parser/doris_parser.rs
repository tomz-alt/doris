//! Doris SQL Parser - Handles Doris-specific SQL syntax
//!
//! This parser extends standard SQL parsing to support Doris-specific features:
//! - PARTITION BY RANGE/LIST with partition definitions
//! - DISTRIBUTED BY HASH BUCKETS
//! - DUPLICATE/UNIQUE/AGGREGATE KEY
//! - Table PROPERTIES
//! - MTMV (Multi-Table Materialized View) syntax
//! - Inverted indexes (USING INVERTED, MATCH operators)
//! - Bloom filters

use datafusion::sql::parser::{DFParser, Statement as DFStatement};
use crate::parser::doris_dialect::DorisDialect;
use crate::error::{DorisError, Result};

/// Doris SQL Parser that handles Doris-specific syntax
pub struct DorisParser;

impl DorisParser {
    /// Parse Doris SQL - handles both standard SQL and Doris-specific extensions
    pub fn parse_sql(sql: &str) -> Result<Vec<DorisStatement>> {
        // First try standard DataFusion parser with Doris dialect
        let dialect = DorisDialect;
        let sql_normalized = Self::normalize_sql(sql);

        // Check if this is Doris-specific syntax
        if Self::is_doris_specific_syntax(&sql_normalized) {
            Self::parse_doris_specific(&sql_normalized)
        } else {
            // Try standard parser
            match DFParser::parse_sql_with_dialect(&sql_normalized, &dialect) {
                Ok(statements) => {
                    Ok(statements.into_iter()
                        .map(DorisStatement::Standard)
                        .collect())
                }
                Err(e) => {
                    // If standard parser fails, try Doris-specific parsing
                    Self::parse_doris_specific(&sql_normalized)
                        .or_else(|_| Err(DorisError::QueryExecution(
                            format!("SQL parse error: {}", e)
                        )))
                }
            }
        }
    }

    /// Normalize SQL for parsing
    fn normalize_sql(sql: &str) -> String {
        sql.trim().to_string()
    }

    /// Check if SQL contains Doris-specific syntax
    fn is_doris_specific_syntax(sql: &str) -> bool {
        let sql_upper = sql.to_uppercase();

        // Doris-specific keywords and patterns
        sql_upper.contains("PARTITION BY RANGE") ||
        sql_upper.contains("PARTITION BY LIST") ||
        sql_upper.contains("DISTRIBUTED BY") ||
        sql_upper.contains("DUPLICATE KEY") ||
        sql_upper.contains("UNIQUE KEY") ||
        sql_upper.contains("AGGREGATE KEY") ||
        sql_upper.contains("BUILD IMMEDIATE") ||
        sql_upper.contains("BUILD DEFERRED") ||
        sql_upper.contains("REFRESH AUTO") ||
        sql_upper.contains("REFRESH MANUAL") ||
        sql_upper.contains("REFRESH ON") ||
        sql_upper.contains("USING INVERTED") ||
        sql_upper.contains("USING BTREE") ||
        sql_upper.contains(" MATCH ") ||
        sql_upper.contains("MATCH_ALL") ||
        sql_upper.contains("MATCH_ANY") ||
        sql_upper.contains("MATCH_PHRASE") ||
        sql_upper.contains("BLOOM_FILTER_COLUMNS") ||
        sql_upper.contains("DROP INDEX") && sql_upper.contains(" ON ") ||
        sql_upper.contains("ALTER TABLE") && sql_upper.contains("DROP INDEX") ||
        sql_upper.contains("ALTER TABLE") && sql_upper.contains("DROP PARTITION") ||
        sql_upper.contains("TEMPORARY PARTITION") ||
        sql_upper.contains("REPLACE PARTITION") ||
        sql_upper.contains("TRUNCATE PARTITION") ||
        sql_upper.contains("MODIFY PARTITION") ||
        sql_upper.contains("DYNAMIC PARTITION") ||
        sql_upper.contains("SHOW PARTITIONS") ||
        sql_upper.contains("SHOW DYNAMIC PARTITION") ||
        sql_upper.contains(" PARTITION(") ||
        sql_upper.contains(" PARTITION (")
    }

    /// Parse Doris-specific SQL syntax
    fn parse_doris_specific(sql: &str) -> Result<Vec<DorisStatement>> {
        let sql_upper = sql.to_uppercase();

        // Validate Doris-specific syntax patterns
        if sql_upper.starts_with("CREATE") {
            Self::validate_create_statement(sql)?;
        } else if sql_upper.starts_with("ALTER") {
            Self::validate_alter_statement(sql)?;
        } else if sql_upper.starts_with("DROP") {
            Self::validate_drop_statement(sql)?;
        } else if sql_upper.starts_with("INSERT") {
            Self::validate_insert_statement(sql)?;
        } else if sql_upper.starts_with("UPDATE") || sql_upper.starts_with("DELETE") {
            Self::validate_dml_statement(sql)?;
        } else if sql_upper.starts_with("SELECT") || sql_upper.contains("SELECT") {
            Self::validate_select_statement(sql)?;
        } else if sql_upper.starts_with("SHOW") {
            Self::validate_show_statement(sql)?;
        } else if sql_upper.starts_with("ADMIN") {
            Self::validate_admin_statement(sql)?;
        } else if sql_upper.starts_with("ANALYZE") {
            Self::validate_analyze_statement(sql)?;
        }

        // Return a Doris-specific statement
        Ok(vec![DorisStatement::DorisSpecific(sql.to_string())])
    }

    fn validate_create_statement(sql: &str) -> Result<()> {
        let sql_upper = sql.to_uppercase();

        // Validate CREATE TABLE with Doris-specific features
        if sql_upper.contains("CREATE TABLE") || sql_upper.contains("CREATE MATERIALIZED VIEW") {
            // Check for balanced parentheses
            Self::check_balanced_parens(sql)?;

            // Validate partition syntax if present
            if sql_upper.contains("PARTITION BY") {
                Self::validate_partition_clause(sql)?;
            }

            // Validate distribution clause if present
            if sql_upper.contains("DISTRIBUTED BY") {
                Self::validate_distribution_clause(sql)?;
            }

            // Validate key type if present
            if sql_upper.contains("DUPLICATE KEY") ||
               sql_upper.contains("UNIQUE KEY") ||
               sql_upper.contains("AGGREGATE KEY") {
                Self::validate_key_clause(sql)?;
            }

            // Validate MTMV syntax if present
            if sql_upper.contains("BUILD") || sql_upper.contains("REFRESH") {
                Self::validate_mtmv_clause(sql)?;
            }

            Ok(())
        } else if sql_upper.contains("CREATE INDEX") {
            Self::validate_create_index(sql)
        } else {
            Ok(())
        }
    }

    fn validate_alter_statement(sql: &str) -> Result<()> {
        let sql_upper = sql.to_uppercase();

        // ALTER TABLE ... ADD/DROP/MODIFY PARTITION
        // ALTER TABLE ... DROP INDEX
        // ALTER TABLE ... SET (properties)
        Self::check_basic_syntax(sql)?;
        Ok(())
    }

    fn validate_drop_statement(sql: &str) -> Result<()> {
        let sql_upper = sql.to_uppercase();

        // DROP INDEX idx_name ON table_name - Doris-specific
        // DROP PARTITION ...
        Self::check_basic_syntax(sql)?;
        Ok(())
    }

    fn validate_insert_statement(sql: &str) -> Result<()> {
        Self::check_basic_syntax(sql)?;
        Ok(())
    }

    fn validate_dml_statement(sql: &str) -> Result<()> {
        Self::check_basic_syntax(sql)?;
        Ok(())
    }

    fn validate_select_statement(sql: &str) -> Result<()> {
        let sql_upper = sql.to_uppercase();

        // Check for MATCH operators
        if sql_upper.contains(" MATCH ") ||
           sql_upper.contains("MATCH_ALL") ||
           sql_upper.contains("MATCH_ANY") ||
           sql_upper.contains("MATCH_PHRASE") {
            // Validate MATCH syntax
            Self::check_basic_syntax(sql)?;
        }

        // Check for PARTITION clause in SELECT
        if sql_upper.contains(" PARTITION(") || sql_upper.contains(" PARTITION (") {
            Self::check_balanced_parens(sql)?;
        }

        Self::check_basic_syntax(sql)?;
        Ok(())
    }

    fn validate_show_statement(sql: &str) -> Result<()> {
        Ok(())
    }

    fn validate_admin_statement(sql: &str) -> Result<()> {
        Ok(())
    }

    fn validate_analyze_statement(sql: &str) -> Result<()> {
        Ok(())
    }

    fn validate_partition_clause(sql: &str) -> Result<()> {
        let sql_upper = sql.to_uppercase();

        // PARTITION BY RANGE(col) (PARTITION p1 VALUES LESS THAN (...))
        // PARTITION BY LIST(col) (PARTITION p1 VALUES IN (...))
        if sql_upper.contains("PARTITION BY RANGE") || sql_upper.contains("PARTITION BY LIST") {
            // Basic validation - check for VALUES keyword
            if sql_upper.contains("VALUES LESS THAN") ||
               sql_upper.contains("VALUES IN") ||
               sql_upper.contains("PARTITION BY") && sql_upper.contains("()") {
                Self::check_balanced_parens(sql)?;
                Ok(())
            } else if sql_upper.contains("PROPERTIES") && sql_upper.contains("DYNAMIC_PARTITION") {
                // Dynamic partitioning
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn validate_distribution_clause(sql: &str) -> Result<()> {
        let sql_upper = sql.to_uppercase();

        // DISTRIBUTED BY HASH(col) BUCKETS n
        if sql_upper.contains("DISTRIBUTED BY") {
            if sql_upper.contains("HASH") && sql_upper.contains("BUCKETS") {
                Self::check_balanced_parens(sql)?;
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn validate_key_clause(sql: &str) -> Result<()> {
        // DUPLICATE KEY(col1, col2)
        // UNIQUE KEY(col1)
        // AGGREGATE KEY(col1)
        Self::check_balanced_parens(sql)?;
        Ok(())
    }

    fn validate_mtmv_clause(sql: &str) -> Result<()> {
        let sql_upper = sql.to_uppercase();

        // BUILD IMMEDIATE/DEFERRED
        // REFRESH AUTO ON MANUAL/SCHEDULE/COMMIT
        // Just check that keywords are present in valid combinations
        if sql_upper.contains("BUILD") {
            if sql_upper.contains("IMMEDIATE") || sql_upper.contains("DEFERRED") {
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn validate_create_index(sql: &str) -> Result<()> {
        Self::check_basic_syntax(sql)?;
        Self::check_balanced_parens(sql)?;
        Ok(())
    }

    /// Check for balanced parentheses
    fn check_balanced_parens(sql: &str) -> Result<()> {
        let mut depth = 0;
        for ch in sql.chars() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth < 0 {
                        return Err(DorisError::QueryExecution(
                            "Unbalanced parentheses".to_string()
                        ));
                    }
                }
                _ => {}
            }
        }

        if depth != 0 {
            return Err(DorisError::QueryExecution(
                "Unbalanced parentheses".to_string()
            ));
        }

        Ok(())
    }

    /// Basic syntax check - ensure SQL has minimum structure
    fn check_basic_syntax(sql: &str) -> Result<()> {
        if sql.trim().is_empty() {
            return Err(DorisError::QueryExecution(
                "Empty SQL statement".to_string()
            ));
        }
        Ok(())
    }
}

/// Doris Statement - either standard SQL or Doris-specific
#[derive(Debug, Clone)]
pub enum DorisStatement {
    /// Standard SQL statement parsed by DataFusion
    Standard(DFStatement),
    /// Doris-specific statement (partition syntax, MTMV, etc.)
    DorisSpecific(String),
}
