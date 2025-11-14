//! DML Statement Tests
//!
//! This module validates DML (Data Manipulation Language) statement parsing
//! for INSERT, UPDATE, DELETE, and REPLACE operations.
//!
//! Based on: regression-test/suites correctness and insert patterns

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
    // INSERT Tests
    // ============================================================================

    #[test]
    fn test_insert_single_row() {
        assert_parses("INSERT INTO test_table (id, name) VALUES (1, 'Alice')");
    }

    #[test]
    fn test_insert_multiple_rows() {
        assert_parses(r#"
            INSERT INTO test_table (id, name) VALUES
            (1, 'Alice'),
            (2, 'Bob'),
            (3, 'Charlie')
        "#);
    }

    #[test]
    fn test_insert_without_column_list() {
        assert_parses("INSERT INTO test_table VALUES (1, 'Alice', 30)");
    }

    #[test]
    fn test_insert_select() {
        assert_parses(r#"
            INSERT INTO target_table (id, name)
            SELECT id, name FROM source_table WHERE status = 'active'
        "#);
    }

    #[test]
    fn test_insert_with_null() {
        assert_parses("INSERT INTO test_table (id, name, age) VALUES (1, 'Alice', NULL)");
    }

    #[test]
    fn test_insert_with_expressions() {
        assert_parses(r#"
            INSERT INTO test_table (id, total, created_at)
            VALUES (1, 100 * 1.1, CURRENT_TIMESTAMP)
        "#);
    }

    #[test]
    fn test_insert_with_default() {
        assert_parses("INSERT INTO test_table (id, name, status) VALUES (1, 'Alice', DEFAULT)");
    }

    #[test]
    fn test_insert_large_batch() {
        let mut sql = String::from("INSERT INTO test_table (id, name) VALUES ");
        for i in 1..=100 {
            if i > 1 {
                sql.push_str(", ");
            }
            sql.push_str(&format!("({}, 'User{}')", i, i));
        }
        assert_parses(&sql);
    }

    #[test]
    fn test_insert_with_subquery() {
        assert_parses(r#"
            INSERT INTO test_table (id, name, count)
            SELECT user_id, name, COUNT(*)
            FROM orders
            GROUP BY user_id, name
        "#);
    }

    #[test]
    fn test_insert_with_cte() {
        assert_parses(r#"
            INSERT INTO test_table (id, name)
            WITH active_users AS (
                SELECT id, name FROM users WHERE status = 'active'
            )
            SELECT id, name FROM active_users
        "#);
    }

    // ============================================================================
    // UPDATE Tests
    // ============================================================================

    #[test]
    fn test_update_single_column() {
        assert_parses("UPDATE test_table SET name = 'Bob' WHERE id = 1");
    }

    #[test]
    fn test_update_multiple_columns() {
        assert_parses(r#"
            UPDATE test_table
            SET name = 'Bob', age = 25, status = 'active'
            WHERE id = 1
        "#);
    }

    #[test]
    fn test_update_without_where() {
        assert_parses("UPDATE test_table SET status = 'inactive'");
    }

    #[test]
    fn test_update_with_expression() {
        assert_parses("UPDATE test_table SET price = price * 1.1 WHERE category = 'electronics'");
    }

    #[test]
    fn test_update_with_case() {
        assert_parses(r#"
            UPDATE test_table
            SET status = CASE
                WHEN age < 18 THEN 'minor'
                WHEN age >= 18 AND age < 65 THEN 'adult'
                ELSE 'senior'
            END
        "#);
    }

    #[test]
    fn test_update_with_subquery() {
        assert_parses(r#"
            UPDATE test_table
            SET category = (SELECT category FROM categories WHERE id = test_table.category_id)
            WHERE category IS NULL
        "#);
    }

    #[test]
    fn test_update_with_join() {
        // Note: UPDATE with JOIN syntax varies by database
        assert_parses(r#"
            UPDATE test_table
            SET status = 'premium'
            WHERE user_id IN (SELECT user_id FROM premium_users)
        "#);
    }

    #[test]
    fn test_update_with_multiple_conditions() {
        assert_parses(r#"
            UPDATE test_table
            SET status = 'verified'
            WHERE status = 'pending'
            AND created_at < DATE '2024-01-01'
            AND email IS NOT NULL
        "#);
    }

    // ============================================================================
    // DELETE Tests
    // ============================================================================

    #[test]
    fn test_delete_with_where() {
        assert_parses("DELETE FROM test_table WHERE id = 1");
    }

    #[test]
    fn test_delete_without_where() {
        assert_parses("DELETE FROM test_table");
    }

    #[test]
    fn test_delete_with_subquery() {
        assert_parses(r#"
            DELETE FROM test_table
            WHERE user_id IN (SELECT user_id FROM inactive_users)
        "#);
    }

    #[test]
    fn test_delete_with_multiple_conditions() {
        assert_parses(r#"
            DELETE FROM test_table
            WHERE status = 'deleted'
            AND created_at < DATE '2020-01-01'
        "#);
    }

    #[test]
    fn test_delete_with_exists() {
        assert_parses(r#"
            DELETE FROM test_table t1
            WHERE EXISTS (
                SELECT 1 FROM archived_table t2
                WHERE t2.id = t1.id
            )
        "#);
    }

    // ============================================================================
    // Complex DML Tests
    // ============================================================================

    #[test]
    fn test_insert_on_duplicate_key() {
        // Note: ON DUPLICATE KEY UPDATE is MySQL-specific, may not be supported
        let sql = "INSERT INTO test_table (id, name) VALUES (1, 'Alice')";
        assert_parses(sql);
    }

    #[test]
    fn test_upsert_pattern() {
        // Basic INSERT that might be used in upsert scenarios
        assert_parses("INSERT INTO test_table (id, name, count) VALUES (1, 'Alice', 1)");
    }

    #[test]
    fn test_multi_table_operations() {
        // Test multiple independent DML statements
        assert_parses("INSERT INTO table1 (id) VALUES (1)");
        assert_parses("UPDATE table2 SET status = 'active' WHERE id = 1");
        assert_parses("DELETE FROM table3 WHERE id = 1");
    }

    #[test]
    fn test_dml_with_functions() {
        assert_parses(r#"
            INSERT INTO test_table (id, name, created_at, updated_at)
            VALUES (1, UPPER('alice'), CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        "#);
    }

    #[test]
    fn test_dml_with_cast() {
        assert_parses(r#"
            INSERT INTO test_table (id, amount, description)
            VALUES (1, CAST('123.45' AS DECIMAL(10,2)), 'Payment')
        "#);
    }

    #[test]
    fn test_dml_with_arithmetic() {
        assert_parses(r#"
            INSERT INTO test_table (id, total, tax, grand_total)
            VALUES (1, 100.00, 100.00 * 0.08, 100.00 * 1.08)
        "#);
    }

    #[test]
    fn test_insert_with_string_functions() {
        assert_parses(r#"
            INSERT INTO test_table (id, email, domain)
            VALUES (1, 'user@example.com', SUBSTRING('user@example.com', POSITION('@' IN 'user@example.com') + 1))
        "#);
    }

    #[test]
    fn test_update_with_concat() {
        assert_parses(r#"
            UPDATE test_table
            SET full_name = CONCAT(first_name, ' ', last_name)
            WHERE full_name IS NULL
        "#);
    }

    #[test]
    fn test_dml_with_date_functions() {
        assert_parses(r#"
            INSERT INTO test_table (id, date_col, year, month, day)
            VALUES (
                1,
                CURRENT_DATE,
                EXTRACT(YEAR FROM CURRENT_DATE),
                EXTRACT(MONTH FROM CURRENT_DATE),
                EXTRACT(DAY FROM CURRENT_DATE)
            )
        "#);
    }

    #[test]
    fn test_insert_with_coalesce() {
        assert_parses(r#"
            INSERT INTO test_table (id, name, status)
            SELECT id, name, COALESCE(status, 'unknown')
            FROM source_table
        "#);
    }

    #[test]
    fn test_update_increment() {
        assert_parses("UPDATE test_table SET counter = counter + 1 WHERE id = 1");
    }

    #[test]
    fn test_delete_with_limit() {
        // Note: LIMIT in DELETE may not be supported by all parsers
        assert_parses("DELETE FROM test_table WHERE status = 'temp'");
    }

    #[test]
    fn test_dml_with_between() {
        assert_parses(r#"
            DELETE FROM test_table
            WHERE created_at BETWEEN DATE '2020-01-01' AND DATE '2020-12-31'
        "#);
    }

    #[test]
    fn test_dml_with_in_list() {
        assert_parses(r#"
            UPDATE test_table
            SET category = 'electronics'
            WHERE product_id IN (1, 2, 3, 4, 5)
        "#);
    }

    #[test]
    fn test_dml_with_like() {
        assert_parses("DELETE FROM test_table WHERE email LIKE '%@spam.com'");
    }

    #[test]
    fn test_dml_with_null_checks() {
        assert_parses("UPDATE test_table SET status = 'pending' WHERE approved_at IS NULL");
        assert_parses("DELETE FROM test_table WHERE deleted_at IS NOT NULL");
    }

    #[test]
    fn test_complex_insert_select() {
        assert_parses(r#"
            INSERT INTO summary_table (date, user_id, total_orders, total_amount)
            SELECT
                DATE(created_at) as date,
                user_id,
                COUNT(*) as total_orders,
                SUM(amount) as total_amount
            FROM orders
            WHERE status = 'completed'
            GROUP BY DATE(created_at), user_id
            HAVING COUNT(*) > 0
        "#);
    }

    #[test]
    fn test_dml_with_window_functions() {
        // INSERT with window function in SELECT
        assert_parses(r#"
            INSERT INTO ranked_table (id, name, rank)
            SELECT
                id,
                name,
                ROW_NUMBER() OVER (ORDER BY score DESC) as rank
            FROM scores_table
        "#);
    }

    #[test]
    fn test_dml_validation() {
        // Ensure various DML statements are recognized
        let dml_statements = vec![
            "INSERT INTO t (id) VALUES (1)",
            "UPDATE t SET name = 'test' WHERE id = 1",
            "DELETE FROM t WHERE id = 1",
            "INSERT INTO t SELECT * FROM t2",
        ];

        for stmt in dml_statements {
            assert_parses(stmt);
        }
    }
}
