//! JOIN Regression Tests - 40 tests covering various JOIN operations
//! Inspired by Doris regression-test/suites/query_p0/join/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // Basic JOIN Tests (10 tests)

    #[test]
    fn test_inner_join_basic() {
        let sql = r#"
            SELECT t1.id, t1.name, t2.value
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_left_join_basic() {
        let sql = r#"
            SELECT t1.id, t1.name, t2.value
            FROM table1 t1
            LEFT JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_right_join_basic() {
        let sql = r#"
            SELECT t1.id, t1.name, t2.value
            FROM table1 t1
            RIGHT JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_full_outer_join() {
        let sql = r#"
            SELECT t1.id, t1.name, t2.value
            FROM table1 t1
            FULL OUTER JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cross_join() {
        let sql = r#"
            SELECT t1.id, t2.id
            FROM table1 t1
            CROSS JOIN table2 t2
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_self_join() {
        let sql = r#"
            SELECT t1.id, t1.name, t2.name
            FROM employees t1
            INNER JOIN employees t2 ON t1.manager_id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_using_clause() {
        let sql = r#"
            SELECT t1.id, t1.name, t2.value
            FROM table1 t1
            INNER JOIN table2 t2 USING (id)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_natural_join() {
        let sql = r#"
            SELECT *
            FROM table1
            NATURAL JOIN table2
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_where() {
        let sql = r#"
            SELECT t1.id, t1.name, t2.value
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            WHERE t1.active = true
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_multiple_conditions() {
        let sql = r#"
            SELECT t1.id, t1.name, t2.value
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id AND t1.type = t2.type
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Multiple JOIN Tests (10 tests)

    #[test]
    fn test_three_table_join() {
        let sql = r#"
            SELECT t1.id, t2.name, t3.value
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            INNER JOIN table3 t3 ON t2.id = t3.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_mixed_join_types() {
        let sql = r#"
            SELECT t1.id, t2.name, t3.value
            FROM table1 t1
            LEFT JOIN table2 t2 ON t1.id = t2.id
            INNER JOIN table3 t3 ON t2.id = t3.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_four_table_join() {
        let sql = r#"
            SELECT t1.id, t2.name, t3.value, t4.status
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            LEFT JOIN table3 t3 ON t2.id = t3.id
            RIGHT JOIN table4 t4 ON t3.id = t4.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_chain() {
        let sql = r#"
            SELECT *
            FROM t1
            JOIN t2 ON t1.a = t2.a
            JOIN t3 ON t2.b = t3.b
            JOIN t4 ON t3.c = t4.c
            JOIN t5 ON t4.d = t5.d
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_aggregation() {
        let sql = r#"
            SELECT t1.dept_id, COUNT(*)
            FROM employees t1
            INNER JOIN departments t2 ON t1.dept_id = t2.id
            GROUP BY t1.dept_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_order_by() {
        let sql = r#"
            SELECT t1.id, t2.name
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            ORDER BY t1.id DESC, t2.name ASC
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_limit() {
        let sql = r#"
            SELECT t1.id, t2.name
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            LIMIT 100
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_offset() {
        let sql = r#"
            SELECT t1.id, t2.name
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            LIMIT 100 OFFSET 50
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_distinct() {
        let sql = r#"
            SELECT DISTINCT t1.id, t2.name
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_aliases() {
        let sql = r#"
            SELECT t1.id AS user_id, t2.name AS user_name
            FROM users t1
            INNER JOIN profiles t2 ON t1.id = t2.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // JOIN with Subqueries (10 tests)

    #[test]
    fn test_join_with_subquery_in_from() {
        let sql = r#"
            SELECT t1.id, sub.total
            FROM table1 t1
            INNER JOIN (SELECT id, SUM(value) AS total FROM table2 GROUP BY id) sub
            ON t1.id = sub.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_multiple_subqueries() {
        let sql = r#"
            SELECT s1.id, s2.value
            FROM (SELECT id, COUNT(*) AS cnt FROM t1 GROUP BY id) s1
            INNER JOIN (SELECT id, SUM(val) AS value FROM t2 GROUP BY id) s2
            ON s1.id = s2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_correlated_subquery() {
        let sql = r#"
            SELECT t1.id, t1.name
            FROM table1 t1
            WHERE EXISTS (
                SELECT 1 FROM table2 t2
                WHERE t2.id = t1.id AND t2.active = true
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_derived_table() {
        let sql = r#"
            SELECT dt.id, dt.total
            FROM (
                SELECT t1.id, SUM(t2.value) AS total
                FROM table1 t1
                INNER JOIN table2 t2 ON t1.id = t2.id
                GROUP BY t1.id
            ) dt
            WHERE dt.total > 1000
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_cte() {
        let sql = r#"
            WITH active_users AS (
                SELECT id, name FROM users WHERE active = true
            )
            SELECT au.name, o.total
            FROM active_users au
            INNER JOIN orders o ON au.id = o.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_multiple_ctes() {
        let sql = r#"
            WITH
            users_cte AS (SELECT id, name FROM users WHERE active = true),
            orders_cte AS (SELECT user_id, SUM(amount) AS total FROM orders GROUP BY user_id)
            SELECT u.name, o.total
            FROM users_cte u
            INNER JOIN orders_cte o ON u.id = o.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_nested_subquery() {
        let sql = r#"
            SELECT t1.id, t1.name
            FROM table1 t1
            WHERE t1.id IN (
                SELECT t2.id FROM table2 t2
                WHERE t2.value IN (SELECT t3.value FROM table3 t3)
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_scalar_subquery() {
        let sql = r#"
            SELECT t1.id,
                   t1.name,
                   (SELECT COUNT(*) FROM orders WHERE user_id = t1.id) AS order_count
            FROM users t1
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_lateral() {
        let sql = r#"
            SELECT t1.id, t1.name, sub.value
            FROM table1 t1
            CROSS JOIN LATERAL (
                SELECT value FROM table2 WHERE id = t1.id LIMIT 1
            ) sub
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_union_subquery() {
        let sql = r#"
            SELECT t1.id, sub.name
            FROM table1 t1
            INNER JOIN (
                SELECT id, name FROM table2
                UNION
                SELECT id, name FROM table3
            ) sub ON t1.id = sub.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Advanced JOIN Tests (10 tests)

    #[test]
    fn test_join_with_case_expression() {
        let sql = r#"
            SELECT t1.id,
                   CASE WHEN t2.value > 100 THEN 'High' ELSE 'Low' END AS category
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_coalesce() {
        let sql = r#"
            SELECT t1.id, COALESCE(t2.value, 0) AS value
            FROM table1 t1
            LEFT JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_cast() {
        let sql = r#"
            SELECT t1.id, CAST(t2.value AS VARCHAR) AS value_str
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_date_functions() {
        let sql = r#"
            SELECT t1.id, t2.created_date
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            WHERE t2.created_date > DATE '2024-01-01'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_string_functions() {
        let sql = r#"
            SELECT t1.id, CONCAT(t1.first_name, ' ', t2.last_name) AS full_name
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_nullif() {
        let sql = r#"
            SELECT t1.id, NULLIF(t2.value, 0) AS safe_value
            FROM table1 t1
            LEFT JOIN table2 t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_having() {
        let sql = r#"
            SELECT t1.dept_id, COUNT(*) AS emp_count
            FROM employees t1
            INNER JOIN departments t2 ON t1.dept_id = t2.id
            GROUP BY t1.dept_id
            HAVING COUNT(*) > 10
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_between() {
        let sql = r#"
            SELECT t1.id, t2.value
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            WHERE t2.value BETWEEN 100 AND 500
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_in_list() {
        let sql = r#"
            SELECT t1.id, t2.status
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            WHERE t2.status IN ('active', 'pending', 'approved')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_with_like() {
        let sql = r#"
            SELECT t1.id, t2.name
            FROM table1 t1
            INNER JOIN table2 t2 ON t1.id = t2.id
            WHERE t2.name LIKE 'A%'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
