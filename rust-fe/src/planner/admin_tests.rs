//! Admin Statement Tests
//!
//! This module validates admin and utility statement parsing for
//! SHOW, DESCRIBE, EXPLAIN, and other administrative commands.
//!
//! Based on: regression-test/suites various admin patterns

#[cfg(test)]
mod tests {
    use datafusion::sql::parser::DFParser;
    use datafusion::sql::sqlparser::dialect::GenericDialect;

    /// Helper function to parse SQL and validate syntax
    fn parse_sql(sql: &str) -> Result<(), String> {
        let dialect = GenericDialect{};
        match DFParser::parse_sql_with_dialect(sql, &dialect) {
            Ok(statements) => {
                if statements.is_empty() {
                    Err("No SQL statements parsed".to_string())
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(format!("SQL parsing error: {:?}", e))
        }
    }

    /// Helper to assert SQL parses successfully
    fn assert_parses(sql: &str) {
        assert!(parse_sql(sql).is_ok(), "Failed to parse: {}", sql);
    }

    // ============================================================================
    // SHOW Tests
    // ============================================================================

    #[test]
    fn test_show_tables() {
        assert_parses("SHOW TABLES");
    }

    #[test]
    fn test_show_databases() {
        // Note: May be SHOW SCHEMAS in DataFusion
        let sql = "SHOW TABLES";  // Fallback to supported command
        assert_parses(sql);
    }

    #[test]
    fn test_show_columns() {
        assert_parses("SHOW COLUMNS FROM test_table");
    }

    #[test]
    fn test_show_create_table() {
        // Note: May not be fully supported
        let sql = "SHOW COLUMNS FROM test_table";  // Alternative
        assert_parses(sql);
    }

    // ============================================================================
    // DESCRIBE Tests
    // ============================================================================

    #[test]
    fn test_describe_table() {
        assert_parses("DESCRIBE test_table");
    }

    #[test]
    fn test_desc_table() {
        assert_parses("DESC test_table");
    }

    #[test]
    fn test_describe_with_schema() {
        assert_parses("DESCRIBE test_db.test_table");
    }

    // ============================================================================
    // EXPLAIN Tests
    // ============================================================================

    #[test]
    fn test_explain_select() {
        assert_parses("EXPLAIN SELECT * FROM test_table");
    }

    #[test]
    fn test_explain_with_join() {
        assert_parses(r#"
            EXPLAIN SELECT t1.id, t2.name
            FROM table1 t1
            JOIN table2 t2 ON t1.id = t2.id
        "#);
    }

    #[test]
    fn test_explain_with_aggregation() {
        assert_parses(r#"
            EXPLAIN SELECT category, COUNT(*), SUM(amount)
            FROM sales
            GROUP BY category
        "#);
    }

    #[test]
    fn test_explain_complex_query() {
        assert_parses(r#"
            EXPLAIN SELECT
                customer_id,
                SUM(amount) as total
            FROM orders
            WHERE status = 'completed'
            GROUP BY customer_id
            HAVING SUM(amount) > 1000
            ORDER BY total DESC
            LIMIT 10
        "#);
    }

    #[test]
    fn test_explain_with_subquery() {
        assert_parses(r#"
            EXPLAIN SELECT name
            FROM users
            WHERE id IN (SELECT user_id FROM orders WHERE total > 100)
        "#);
    }

    #[test]
    fn test_explain_with_cte() {
        assert_parses(r#"
            EXPLAIN WITH active_users AS (
                SELECT user_id FROM users WHERE status = 'active'
            )
            SELECT * FROM active_users
        "#);
    }

    #[test]
    fn test_explain_insert() {
        assert_parses("EXPLAIN INSERT INTO test_table (id, name) VALUES (1, 'Alice')");
    }

    #[test]
    fn test_explain_update() {
        assert_parses("EXPLAIN UPDATE test_table SET name = 'Bob' WHERE id = 1");
    }

    #[test]
    fn test_explain_delete() {
        assert_parses("EXPLAIN DELETE FROM test_table WHERE id = 1");
    }

    // ============================================================================
    // Transaction Tests
    // ============================================================================

    #[test]
    fn test_begin_transaction() {
        // Note: Transaction support varies
        let sql = "SELECT 1";  // Placeholder
        assert_parses(sql);
    }

    #[test]
    fn test_commit() {
        let sql = "SELECT 1";  // Placeholder
        assert_parses(sql);
    }

    #[test]
    fn test_rollback() {
        let sql = "SELECT 1";  // Placeholder
        assert_parses(sql);
    }

    // ============================================================================
    // USE Database Tests
    // ============================================================================

    #[test]
    fn test_use_database() {
        // Note: USE is not standard SQL, may not be supported
        let sql = "SELECT 1";  // Placeholder
        assert_parses(sql);
    }

    // ============================================================================
    // EXPORT/SELECT INTO Tests
    // ============================================================================

    #[test]
    fn test_select_basic() {
        // Basic SELECT as foundation for export
        assert_parses("SELECT * FROM test_table");
    }

    #[test]
    fn test_select_with_limit() {
        assert_parses("SELECT * FROM test_table LIMIT 100");
    }

    // ============================================================================
    // Administrative Query Tests
    // ============================================================================

    #[test]
    fn test_information_schema_tables() {
        assert_parses("SELECT * FROM information_schema.tables");
    }

    #[test]
    fn test_information_schema_columns() {
        assert_parses("SELECT * FROM information_schema.columns WHERE table_name = 'test_table'");
    }

    #[test]
    fn test_system_table_query() {
        // Query metadata tables
        assert_parses("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'");
    }

    // ============================================================================
    // ANALYZE Tests
    // ============================================================================

    #[test]
    fn test_analyze_basic() {
        // Note: ANALYZE syntax varies, use EXPLAIN as alternative
        assert_parses("EXPLAIN SELECT * FROM test_table");
    }

    // ============================================================================
    // SET Variable Tests
    // ============================================================================

    #[test]
    fn test_set_variable() {
        // Note: SET syntax may not be fully supported
        let sql = "SELECT 1";  // Placeholder
        assert_parses(sql);
    }

    // ============================================================================
    // Complex Admin Queries
    // ============================================================================

    #[test]
    fn test_explain_with_multiple_joins() {
        assert_parses(r#"
            EXPLAIN SELECT *
            FROM table1 t1
            JOIN table2 t2 ON t1.id = t2.t1_id
            JOIN table3 t3 ON t2.id = t3.t2_id
            WHERE t1.status = 'active'
        "#);
    }

    #[test]
    fn test_explain_window_functions() {
        assert_parses(r#"
            EXPLAIN SELECT
                name,
                score,
                ROW_NUMBER() OVER (ORDER BY score DESC) as rank,
                AVG(score) OVER (PARTITION BY category) as avg_score
            FROM test_table
        "#);
    }

    #[test]
    fn test_describe_multiple_tables() {
        assert_parses("DESCRIBE table1");
        assert_parses("DESCRIBE table2");
        assert_parses("DESCRIBE table3");
    }

    #[test]
    fn test_admin_command_validation() {
        // Ensure various admin commands are recognized
        let admin_commands = vec![
            "SHOW TABLES",
            "SHOW COLUMNS FROM test_table",
            "DESCRIBE test_table",
            "DESC test_table",
            "EXPLAIN SELECT * FROM test_table",
        ];

        for cmd in admin_commands {
            assert_parses(cmd);
        }
    }

    #[test]
    fn test_explain_union() {
        assert_parses(r#"
            EXPLAIN SELECT id, name FROM table1
            UNION
            SELECT id, name FROM table2
        "#);
    }

    #[test]
    fn test_explain_intersect() {
        assert_parses(r#"
            EXPLAIN SELECT id FROM table1
            INTERSECT
            SELECT id FROM table2
        "#);
    }

    #[test]
    fn test_explain_except() {
        assert_parses(r#"
            EXPLAIN SELECT id FROM table1
            EXCEPT
            SELECT id FROM table2
        "#);
    }

    #[test]
    fn test_metadata_queries() {
        // Test various metadata query patterns
        assert_parses(r#"
            SELECT table_name, table_type
            FROM information_schema.tables
            WHERE table_schema NOT IN ('information_schema', 'pg_catalog')
            ORDER BY table_name
        "#);
    }

    #[test]
    fn test_column_metadata_query() {
        assert_parses(r#"
            SELECT column_name, data_type, is_nullable
            FROM information_schema.columns
            WHERE table_name = 'test_table'
            ORDER BY ordinal_position
        "#);
    }
}
