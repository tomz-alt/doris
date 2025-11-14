//! Subquery Regression Tests - 45 tests covering various subquery patterns
//! Inspired by Doris regression-test/suites/query_p0/subquery/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // Scalar Subqueries (10 tests)

    #[test]
    fn test_scalar_subquery_select() {
        let sql = r#"
            SELECT
                id,
                name,
                (SELECT COUNT(*) FROM orders WHERE user_id = users.id) AS order_count
            FROM users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_scalar_subquery_where() {
        let sql = r#"
            SELECT *
            FROM products
            WHERE price > (SELECT AVG(price) FROM products)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_scalar_subquery_multiple() {
        let sql = r#"
            SELECT
                id,
                (SELECT MAX(price) FROM products) AS max_price,
                (SELECT MIN(price) FROM products) AS min_price,
                (SELECT AVG(price) FROM products) AS avg_price
            FROM dual
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_scalar_subquery_in_expression() {
        let sql = r#"
            SELECT
                id,
                price,
                price - (SELECT AVG(price) FROM products) AS price_diff_from_avg
            FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_scalar_subquery_case() {
        let sql = r#"
            SELECT
                id,
                CASE
                    WHEN price > (SELECT AVG(price) FROM products) THEN 'Above Average'
                    ELSE 'Below Average'
                END AS price_category
            FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_scalar_subquery_nested() {
        let sql = r#"
            SELECT *
            FROM products
            WHERE price > (
                SELECT AVG(price)
                FROM products
                WHERE category = (SELECT category FROM products ORDER BY sales DESC LIMIT 1)
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_scalar_subquery_with_join() {
        let sql = r#"
            SELECT
                u.id,
                u.name,
                (SELECT SUM(amount) FROM orders o WHERE o.user_id = u.id) AS total_spent
            FROM users u
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_scalar_subquery_aggregate() {
        let sql = r#"
            SELECT
                category,
                COUNT(*) AS product_count,
                (SELECT AVG(price) FROM products) AS overall_avg_price
            FROM products
            GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_scalar_subquery_having() {
        let sql = r#"
            SELECT category, AVG(price) AS avg_price
            FROM products
            GROUP BY category
            HAVING AVG(price) > (SELECT AVG(price) FROM products)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_scalar_subquery_order_by() {
        let sql = r#"
            SELECT id, name, price
            FROM products
            ORDER BY (SELECT AVG(price) FROM products WHERE category = products.category)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // IN/NOT IN Subqueries (10 tests)

    #[test]
    fn test_in_subquery_basic() {
        let sql = r#"
            SELECT *
            FROM users
            WHERE id IN (SELECT user_id FROM orders WHERE status = 'completed')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_not_in_subquery() {
        let sql = r#"
            SELECT *
            FROM products
            WHERE id NOT IN (SELECT product_id FROM discontinued_products)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_in_subquery_multiple_columns() {
        let sql = r#"
            SELECT *
            FROM orders
            WHERE (user_id, product_id) IN (
                SELECT user_id, product_id FROM favorites
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_in_subquery_nested() {
        let sql = r#"
            SELECT *
            FROM users
            WHERE id IN (
                SELECT user_id FROM orders
                WHERE product_id IN (SELECT id FROM products WHERE category = 'Electronics')
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_in_subquery_with_aggregation() {
        let sql = r#"
            SELECT *
            FROM products
            WHERE price IN (SELECT MAX(price) FROM products GROUP BY category)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_in_subquery_correlated() {
        let sql = r#"
            SELECT *
            FROM products p1
            WHERE price IN (
                SELECT MAX(price)
                FROM products p2
                WHERE p2.category = p1.category
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_not_in_with_null_handling() {
        let sql = r#"
            SELECT *
            FROM users
            WHERE id NOT IN (SELECT user_id FROM blocked_users WHERE user_id IS NOT NULL)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_in_subquery_with_distinct() {
        let sql = r#"
            SELECT *
            FROM orders
            WHERE user_id IN (SELECT DISTINCT user_id FROM premium_members)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_in_subquery_complex() {
        let sql = r#"
            SELECT *
            FROM products
            WHERE id IN (
                SELECT product_id
                FROM order_items
                WHERE order_id IN (
                    SELECT id FROM orders WHERE created_date > DATE '2024-01-01'
                )
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_in_subquery_with_join() {
        let sql = r#"
            SELECT *
            FROM users u
            WHERE u.id IN (
                SELECT o.user_id
                FROM orders o
                JOIN order_items oi ON o.id = oi.order_id
                WHERE oi.product_id = 123
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // EXISTS/NOT EXISTS Subqueries (10 tests)

    #[test]
    fn test_exists_basic() {
        let sql = r#"
            SELECT *
            FROM users
            WHERE EXISTS (SELECT 1 FROM orders WHERE orders.user_id = users.id)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_not_exists_basic() {
        let sql = r#"
            SELECT *
            FROM users
            WHERE NOT EXISTS (SELECT 1 FROM orders WHERE orders.user_id = users.id)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_exists_with_condition() {
        let sql = r#"
            SELECT *
            FROM products
            WHERE EXISTS (
                SELECT 1 FROM order_items
                WHERE order_items.product_id = products.id
                AND order_items.quantity > 10
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_exists_multiple_conditions() {
        let sql = r#"
            SELECT *
            FROM users u
            WHERE EXISTS (
                SELECT 1 FROM orders o
                WHERE o.user_id = u.id
                AND o.status = 'completed'
                AND o.total_amount > 1000
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_exists_nested() {
        let sql = r#"
            SELECT *
            FROM users u
            WHERE EXISTS (
                SELECT 1 FROM orders o
                WHERE o.user_id = u.id
                AND EXISTS (
                    SELECT 1 FROM order_items oi
                    WHERE oi.order_id = o.id
                    AND oi.product_id IN (SELECT id FROM premium_products)
                )
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_exists_with_aggregation() {
        let sql = r#"
            SELECT *
            FROM categories c
            WHERE EXISTS (
                SELECT 1 FROM products p
                WHERE p.category_id = c.id
                GROUP BY p.category_id
                HAVING COUNT(*) > 10
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_exists_and_not_exists() {
        let sql = r#"
            SELECT *
            FROM users u
            WHERE EXISTS (SELECT 1 FROM orders WHERE user_id = u.id)
            AND NOT EXISTS (SELECT 1 FROM complaints WHERE user_id = u.id)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_exists_with_join() {
        let sql = r#"
            SELECT *
            FROM products p
            WHERE EXISTS (
                SELECT 1
                FROM order_items oi
                JOIN orders o ON oi.order_id = o.id
                WHERE oi.product_id = p.id
                AND o.created_date > DATE '2024-01-01'
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_exists_select_list() {
        let sql = r#"
            SELECT
                id,
                name,
                CASE WHEN EXISTS (
                    SELECT 1 FROM orders WHERE user_id = users.id
                ) THEN 'Has Orders' ELSE 'No Orders' END AS order_status
            FROM users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_not_exists_anti_join() {
        let sql = r#"
            SELECT p.*
            FROM products p
            WHERE NOT EXISTS (
                SELECT 1 FROM order_items oi
                WHERE oi.product_id = p.id
                AND oi.created_date > DATE '2024-01-01'
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Derived Table Subqueries (10 tests)

    #[test]
    fn test_derived_table_basic() {
        let sql = r#"
            SELECT *
            FROM (SELECT id, name, price FROM products WHERE active = true) AS active_products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_derived_table_with_aggregation() {
        let sql = r#"
            SELECT category, total_sales
            FROM (
                SELECT category, SUM(sales) AS total_sales
                FROM products
                GROUP BY category
            ) AS category_sales
            WHERE total_sales > 10000
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_derived_table_join() {
        let sql = r#"
            SELECT u.name, o.order_count
            FROM users u
            JOIN (
                SELECT user_id, COUNT(*) AS order_count
                FROM orders
                GROUP BY user_id
            ) o ON u.id = o.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_derived_table_multiple() {
        let sql = r#"
            SELECT s1.category, s1.product_count, s2.total_revenue
            FROM (
                SELECT category, COUNT(*) AS product_count
                FROM products
                GROUP BY category
            ) s1
            JOIN (
                SELECT category, SUM(revenue) AS total_revenue
                FROM sales
                GROUP BY category
            ) s2 ON s1.category = s2.category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_derived_table_nested() {
        let sql = r#"
            SELECT *
            FROM (
                SELECT category, avg_price
                FROM (
                    SELECT category, AVG(price) AS avg_price
                    FROM products
                    GROUP BY category
                ) AS inner_subq
                WHERE avg_price > 100
            ) AS outer_subq
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_derived_table_with_window() {
        let sql = r#"
            SELECT *
            FROM (
                SELECT
                    id,
                    name,
                    price,
                    ROW_NUMBER() OVER (PARTITION BY category ORDER BY price DESC) AS rank
                FROM products
            ) ranked
            WHERE rank <= 10
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_derived_table_union() {
        let sql = r#"
            SELECT *
            FROM (
                SELECT id, name FROM customers WHERE country = 'US'
                UNION
                SELECT id, name FROM customers WHERE country = 'UK'
            ) AS combined
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_derived_table_cross_join() {
        let sql = r#"
            SELECT d1.value, d2.value
            FROM (SELECT 1 AS value UNION SELECT 2 UNION SELECT 3) d1
            CROSS JOIN (SELECT 'A' AS value UNION SELECT 'B') d2
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_derived_table_complex() {
        let sql = r#"
            SELECT
                dt.user_id,
                dt.total_orders,
                dt.total_spent,
                dt.avg_order_value
            FROM (
                SELECT
                    o.user_id,
                    COUNT(*) AS total_orders,
                    SUM(o.total_amount) AS total_spent,
                    AVG(o.total_amount) AS avg_order_value
                FROM orders o
                WHERE o.status = 'completed'
                GROUP BY o.user_id
                HAVING COUNT(*) > 5
            ) dt
            WHERE dt.total_spent > 1000
            ORDER BY dt.total_spent DESC
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_derived_table_with_cte() {
        let sql = r#"
            WITH top_products AS (
                SELECT id, name, price
                FROM products
                ORDER BY sales DESC
                LIMIT 100
            )
            SELECT category, COUNT(*) AS product_count
            FROM (
                SELECT p.category
                FROM top_products tp
                JOIN products p ON tp.id = p.id
            ) AS categorized
            GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Correlated Subqueries (5 tests)

    #[test]
    fn test_correlated_subquery_select() {
        let sql = r#"
            SELECT
                p.id,
                p.name,
                (SELECT COUNT(*) FROM reviews r WHERE r.product_id = p.id) AS review_count
            FROM products p
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_correlated_subquery_where() {
        let sql = r#"
            SELECT *
            FROM products p1
            WHERE price > (
                SELECT AVG(price)
                FROM products p2
                WHERE p2.category = p1.category
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_correlated_subquery_complex() {
        let sql = r#"
            SELECT u.id, u.name
            FROM users u
            WHERE (
                SELECT SUM(amount)
                FROM orders o
                WHERE o.user_id = u.id
                AND o.created_date >= DATE '2024-01-01'
            ) > 1000
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_correlated_subquery_nested() {
        let sql = r#"
            SELECT *
            FROM products p
            WHERE EXISTS (
                SELECT 1 FROM order_items oi
                WHERE oi.product_id = p.id
                AND oi.quantity > (
                    SELECT AVG(quantity)
                    FROM order_items oi2
                    WHERE oi2.product_id = p.id
                )
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_correlated_subquery_multiple() {
        let sql = r#"
            SELECT
                p.id,
                p.name,
                (SELECT COUNT(*) FROM reviews WHERE product_id = p.id) AS review_count,
                (SELECT AVG(rating) FROM reviews WHERE product_id = p.id) AS avg_rating,
                (SELECT COUNT(*) FROM order_items WHERE product_id = p.id) AS times_ordered
            FROM products p
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
