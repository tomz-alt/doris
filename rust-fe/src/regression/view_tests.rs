//! View and CTE (Common Table Expression) Tests
//! Inspired by Doris regression-test/suites/view_p0/ and with_clause_p0/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // Basic View Tests (20 tests)

    #[test]
    fn test_create_view_basic() {
        let sql = r#"
            CREATE VIEW user_view AS
            SELECT id, name, email FROM users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_view_with_where() {
        let sql = r#"
            CREATE VIEW active_users AS
            SELECT * FROM users WHERE active = true
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_view_with_join() {
        let sql = r#"
            CREATE VIEW user_orders AS
            SELECT u.name, o.order_id, o.total
            FROM users u
            INNER JOIN orders o ON u.id = o.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_view_with_aggregation() {
        let sql = r#"
            CREATE VIEW user_summary AS
            SELECT
                user_id,
                COUNT(*) as order_count,
                SUM(total) as total_spent
            FROM orders
            GROUP BY user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_or_replace_view() {
        let sql = r#"
            CREATE OR REPLACE VIEW user_view AS
            SELECT id, name, email, phone FROM users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_view_if_not_exists() {
        let sql = r#"
            CREATE VIEW IF NOT EXISTS user_view AS
            SELECT * FROM users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_view_with_subquery() {
        let sql = r#"
            CREATE VIEW high_value_users AS
            SELECT * FROM users
            WHERE id IN (
                SELECT user_id FROM orders
                GROUP BY user_id
                HAVING SUM(total) > 10000
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_view_with_union() {
        let sql = r#"
            CREATE VIEW all_users AS
            SELECT id, name FROM active_users
            UNION
            SELECT id, name FROM inactive_users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_view_with_case() {
        let sql = r#"
            CREATE VIEW user_tiers AS
            SELECT
                id,
                name,
                CASE
                    WHEN total_spent > 10000 THEN 'Gold'
                    WHEN total_spent > 1000 THEN 'Silver'
                    ELSE 'Bronze'
                END as tier
            FROM users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_view_with_window_function() {
        let sql = r#"
            CREATE VIEW ranked_products AS
            SELECT
                id,
                name,
                category,
                price,
                ROW_NUMBER() OVER (PARTITION BY category ORDER BY price DESC) as rank
            FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_query_from_view() {
        let sql = "SELECT * FROM user_view WHERE name LIKE 'A%'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_join_views() {
        let sql = r#"
            SELECT
                uv.name,
                us.order_count,
                us.total_spent
            FROM user_view uv
            INNER JOIN user_summary us ON uv.id = us.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_aggregate_from_view() {
        let sql = r#"
            SELECT
                tier,
                COUNT(*) as user_count,
                AVG(total_spent) as avg_spent
            FROM user_tiers
            GROUP BY tier
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_nested_view_query() {
        let sql = r#"
            CREATE VIEW top_users AS
            SELECT * FROM user_summary
            WHERE order_count > 10
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_view_with_distinct() {
        let sql = r#"
            CREATE VIEW unique_categories AS
            SELECT DISTINCT category FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_drop_view() {
        let sql = "DROP VIEW user_view";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_drop_view_if_exists() {
        let sql = "DROP VIEW IF EXISTS user_view";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_create_view() {
        let sql = "SHOW CREATE VIEW user_view";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_describe_view() {
        let sql = "DESCRIBE user_view";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_views() {
        let sql = "SHOW VIEWS";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // CTE (Common Table Expression) Tests (30 tests)

    #[test]
    fn test_cte_basic() {
        let sql = r#"
            WITH active_users AS (
                SELECT * FROM users WHERE active = true
            )
            SELECT * FROM active_users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_aggregation() {
        let sql = r#"
            WITH order_summary AS (
                SELECT user_id, COUNT(*) as count, SUM(total) as total
                FROM orders
                GROUP BY user_id
            )
            SELECT * FROM order_summary WHERE total > 1000
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_multiple_ctes() {
        let sql = r#"
            WITH
            active_users AS (
                SELECT id, name FROM users WHERE active = true
            ),
            user_orders AS (
                SELECT user_id, SUM(total) as total FROM orders GROUP BY user_id
            )
            SELECT au.name, uo.total
            FROM active_users au
            JOIN user_orders uo ON au.id = uo.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_join() {
        let sql = r#"
            WITH user_stats AS (
                SELECT user_id, COUNT(*) as order_count
                FROM orders
                GROUP BY user_id
            )
            SELECT u.name, us.order_count
            FROM users u
            JOIN user_stats us ON u.id = us.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_subquery() {
        let sql = r#"
            WITH high_spenders AS (
                SELECT user_id, SUM(total) as total_spent
                FROM orders
                GROUP BY user_id
                HAVING SUM(total) > 10000
            )
            SELECT * FROM users
            WHERE id IN (SELECT user_id FROM high_spenders)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_nested_cte() {
        let sql = r#"
            WITH
            level1 AS (SELECT * FROM users WHERE active = true),
            level2 AS (SELECT * FROM level1 WHERE verified = true),
            level3 AS (SELECT * FROM level2 WHERE age > 18)
            SELECT * FROM level3
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_recursive_cte() {
        let sql = r#"
            WITH RECURSIVE employee_hierarchy AS (
                SELECT id, name, manager_id, 1 as level
                FROM employees
                WHERE manager_id IS NULL
                UNION ALL
                SELECT e.id, e.name, e.manager_id, eh.level + 1
                FROM employees e
                JOIN employee_hierarchy eh ON e.manager_id = eh.id
            )
            SELECT * FROM employee_hierarchy
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_window_function() {
        let sql = r#"
            WITH ranked AS (
                SELECT
                    id,
                    name,
                    price,
                    ROW_NUMBER() OVER (ORDER BY price DESC) as rank
                FROM products
            )
            SELECT * FROM ranked WHERE rank <= 10
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_union() {
        let sql = r#"
            WITH all_transactions AS (
                SELECT id, amount, 'order' as type FROM orders
                UNION ALL
                SELECT id, amount, 'refund' as type FROM refunds
            )
            SELECT type, SUM(amount) as total
            FROM all_transactions
            GROUP BY type
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_distinct() {
        let sql = r#"
            WITH unique_emails AS (
                SELECT DISTINCT email FROM users
            )
            SELECT COUNT(*) FROM unique_emails
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_case() {
        let sql = r#"
            WITH categorized AS (
                SELECT
                    id,
                    amount,
                    CASE
                        WHEN amount > 1000 THEN 'Large'
                        WHEN amount > 100 THEN 'Medium'
                        ELSE 'Small'
                    END as size
                FROM transactions
            )
            SELECT size, COUNT(*) FROM categorized GROUP BY size
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_having() {
        let sql = r#"
            WITH category_sales AS (
                SELECT category, SUM(sales) as total
                FROM products
                GROUP BY category
                HAVING SUM(sales) > 10000
            )
            SELECT * FROM category_sales ORDER BY total DESC
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_order_by_limit() {
        let sql = r#"
            WITH top_products AS (
                SELECT * FROM products
                ORDER BY sales DESC
                LIMIT 100
            )
            SELECT category, COUNT(*) FROM top_products GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_in_subquery() {
        let sql = r#"
            SELECT *
            FROM users
            WHERE id IN (
                WITH active_orders AS (
                    SELECT DISTINCT user_id FROM orders WHERE status = 'active'
                )
                SELECT user_id FROM active_orders
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_exists() {
        let sql = r#"
            WITH premium_users AS (
                SELECT id FROM users WHERE tier = 'premium'
            )
            SELECT * FROM orders o
            WHERE EXISTS (SELECT 1 FROM premium_users pu WHERE pu.id = o.user_id)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_not_exists() {
        let sql = r#"
            WITH blocked_users AS (
                SELECT user_id FROM blocked_list
            )
            SELECT * FROM users u
            WHERE NOT EXISTS (SELECT 1 FROM blocked_users bu WHERE bu.user_id = u.id)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_cross_join() {
        let sql = r#"
            WITH
            dates AS (SELECT DISTINCT order_date FROM orders),
            products AS (SELECT DISTINCT product_id FROM order_items)
            SELECT d.order_date, p.product_id
            FROM dates d
            CROSS JOIN products p
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_left_join() {
        let sql = r#"
            WITH user_stats AS (
                SELECT user_id, COUNT(*) as count FROM orders GROUP BY user_id
            )
            SELECT u.*, COALESCE(us.count, 0) as order_count
            FROM users u
            LEFT JOIN user_stats us ON u.id = us.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_full_outer_join() {
        let sql = r#"
            WITH
            table1_sum AS (SELECT id, SUM(value) as total FROM table1 GROUP BY id),
            table2_sum AS (SELECT id, SUM(value) as total FROM table2 GROUP BY id)
            SELECT COALESCE(t1.id, t2.id) as id, t1.total, t2.total
            FROM table1_sum t1
            FULL OUTER JOIN table2_sum t2 ON t1.id = t2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_self_join() {
        let sql = r#"
            WITH employees_cte AS (
                SELECT id, name, manager_id FROM employees
            )
            SELECT e1.name as employee, e2.name as manager
            FROM employees_cte e1
            JOIN employees_cte e2 ON e1.manager_id = e2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_coalesce() {
        let sql = r#"
            WITH nullable_data AS (
                SELECT id, COALESCE(value, 0) as value FROM table1
            )
            SELECT * FROM nullable_data WHERE value > 0
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_nullif() {
        let sql = r#"
            WITH safe_division AS (
                SELECT id, value1 / NULLIF(value2, 0) as result FROM table1
            )
            SELECT * FROM safe_division WHERE result IS NOT NULL
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_date_functions() {
        let sql = r#"
            WITH monthly_data AS (
                SELECT
                    EXTRACT(YEAR FROM date_col) as year,
                    EXTRACT(MONTH FROM date_col) as month,
                    SUM(amount) as total
                FROM transactions
                GROUP BY year, month
            )
            SELECT * FROM monthly_data ORDER BY year, month
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_string_functions() {
        let sql = r#"
            WITH cleaned_data AS (
                SELECT
                    id,
                    UPPER(TRIM(name)) as clean_name,
                    LOWER(email) as clean_email
                FROM users
            )
            SELECT * FROM cleaned_data
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_cast() {
        let sql = r#"
            WITH converted AS (
                SELECT
                    id,
                    CAST(value AS DECIMAL(10,2)) as decimal_value,
                    CAST(date_str AS DATE) as date_value
                FROM raw_data
            )
            SELECT * FROM converted
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_in_clause() {
        let sql = r#"
            WITH filtered AS (
                SELECT * FROM products
                WHERE category IN ('Electronics', 'Books', 'Clothing')
            )
            SELECT category, COUNT(*) FROM filtered GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_between() {
        let sql = r#"
            WITH price_range AS (
                SELECT * FROM products
                WHERE price BETWEEN 10 AND 100
            )
            SELECT AVG(price) FROM price_range
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_like() {
        let sql = r#"
            WITH filtered_users AS (
                SELECT * FROM users
                WHERE email LIKE '%@company.com'
            )
            SELECT COUNT(*) FROM filtered_users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_complex_analytics() {
        let sql = r#"
            WITH
            daily_sales AS (
                SELECT DATE(order_date) as date, SUM(amount) as total
                FROM sales
                GROUP BY DATE(order_date)
            ),
            moving_avg AS (
                SELECT
                    date,
                    total,
                    AVG(total) OVER (ORDER BY date ROWS BETWEEN 6 PRECEDING AND CURRENT ROW) as avg_7day
                FROM daily_sales
            )
            SELECT * FROM moving_avg WHERE total > avg_7day * 1.2
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cte_with_union_all() {
        let sql = r#"
            WITH combined AS (
                SELECT 'Q1' as quarter, SUM(amount) as total FROM sales WHERE QUARTER(date) = 1
                UNION ALL
                SELECT 'Q2', SUM(amount) FROM sales WHERE QUARTER(date) = 2
                UNION ALL
                SELECT 'Q3', SUM(amount) FROM sales WHERE QUARTER(date) = 3
                UNION ALL
                SELECT 'Q4', SUM(amount) FROM sales WHERE QUARTER(date) = 4
            )
            SELECT * FROM combined ORDER BY quarter
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
