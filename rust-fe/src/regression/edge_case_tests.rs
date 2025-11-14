//! Edge Case Regression Tests - 28 tests covering edge cases and error handling
//! Inspired by Doris regression-test/suites/edge_cases/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // Empty Result Sets (5 tests)

    #[test]
    fn test_empty_where_clause() {
        let sql = "SELECT * FROM users WHERE 1 = 0";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_empty_join_result() {
        let sql = r#"
            SELECT *
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            WHERE 1 = 0
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_empty_group_by() {
        let sql = r#"
            SELECT category, COUNT(*)
            FROM products
            WHERE 1 = 0
            GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_empty_union() {
        let sql = r#"
            SELECT id FROM table1 WHERE 1 = 0
            UNION
            SELECT id FROM table2 WHERE 1 = 0
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_empty_subquery() {
        let sql = r#"
            SELECT *
            FROM users
            WHERE id IN (SELECT id FROM orders WHERE 1 = 0)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // NULL Handling (8 tests)

    #[test]
    fn test_null_in_select() {
        let sql = "SELECT NULL AS null_col, 1 AS num_col, 'text' AS text_col";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_in_where() {
        let sql = "SELECT * FROM table1 WHERE nullable_column IS NULL";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_in_join() {
        let sql = r#"
            SELECT *
            FROM table1 t1
            LEFT JOIN table2 t2 ON t1.id = t2.id
            WHERE t2.id IS NULL
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_in_aggregate() {
        let sql = r#"
            SELECT
                COUNT(*) AS total_count,
                COUNT(nullable_column) AS non_null_count,
                SUM(nullable_column) AS sum_non_null
            FROM table1
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_in_case() {
        let sql = r#"
            SELECT
                CASE
                    WHEN value IS NULL THEN 'null'
                    WHEN value = 0 THEN 'zero'
                    ELSE 'other'
                END AS result
            FROM table1
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_comparison() {
        let sql = r#"
            SELECT
                NULL = NULL AS null_equals_null,
                NULL <> NULL AS null_not_equals_null,
                NULL > 5 AS null_gt_five,
                5 < NULL AS five_lt_null
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_in_order_by() {
        let sql = "SELECT id, nullable_column FROM table1 ORDER BY nullable_column NULLS FIRST";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_in_group_by() {
        let sql = r#"
            SELECT nullable_column, COUNT(*)
            FROM table1
            GROUP BY nullable_column
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Extreme Values (5 tests)

    #[test]
    fn test_max_bigint() {
        let sql = "SELECT 9223372036854775807 AS max_bigint";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_min_bigint() {
        let sql = "SELECT -9223372036854775808 AS min_bigint";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_very_large_decimal() {
        let sql = "SELECT CAST(99999999999999.999999 AS DECIMAL(20, 6)) AS large_decimal";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_very_small_decimal() {
        let sql = "SELECT CAST(0.000001 AS DECIMAL(10, 6)) AS small_decimal";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_extreme_date_range() {
        let sql = r#"
            SELECT
                DATE '1000-01-01' AS old_date,
                DATE '9999-12-31' AS future_date
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Special Characters and Edge Cases (5 tests)

    #[test]
    fn test_empty_string() {
        let sql = "SELECT '' AS empty_string, LENGTH('') AS empty_length";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_special_characters() {
        let sql = "SELECT 'Hello\nWorld' AS newline, 'Tab\there' AS tab, 'Quote''s' AS quote";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_unicode_characters() {
        let sql = "SELECT 'Hello 世界' AS unicode, '🚀' AS emoji";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_long_string() {
        let sql = "SELECT REPEAT('a', 1000) AS long_string, LENGTH(REPEAT('a', 1000)) AS len";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_whitespace_only() {
        let sql = "SELECT '   ' AS spaces, TRIM('   ') AS trimmed";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Complex Queries (5 tests)

    #[test]
    fn test_deeply_nested_subquery() {
        let sql = r#"
            SELECT *
            FROM table1
            WHERE id IN (
                SELECT id FROM table2
                WHERE value IN (
                    SELECT value FROM table3
                    WHERE type IN (
                        SELECT type FROM table4
                    )
                )
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_multiple_unions() {
        let sql = r#"
            SELECT id, 'A' AS source FROM table1
            UNION
            SELECT id, 'B' AS source FROM table2
            UNION
            SELECT id, 'C' AS source FROM table3
            UNION
            SELECT id, 'D' AS source FROM table4
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_complex_expression() {
        let sql = r#"
            SELECT
                ((a + b) * c) / NULLIF(d, 0) AS complex1,
                CASE
                    WHEN x > 0 THEN LOG(x)
                    WHEN x < 0 THEN -LOG(-x)
                    ELSE 0
                END AS complex2
            FROM table1
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_many_joins() {
        let sql = r#"
            SELECT t1.id
            FROM t1
            JOIN t2 ON t1.id = t2.id
            JOIN t3 ON t2.id = t3.id
            JOIN t4 ON t3.id = t4.id
            JOIN t5 ON t4.id = t5.id
            JOIN t6 ON t5.id = t6.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_complex_window_function() {
        let sql = r#"
            SELECT
                id,
                value,
                ROW_NUMBER() OVER (PARTITION BY category ORDER BY value DESC) AS rank,
                LAG(value, 1, 0) OVER (PARTITION BY category ORDER BY date) AS prev_value,
                SUM(value) OVER (PARTITION BY category ORDER BY date ROWS BETWEEN 6 PRECEDING AND CURRENT ROW) AS moving_sum
            FROM table1
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
