//! Aggregate Regression Tests - 50 tests covering aggregations and window functions
//! Inspired by Doris regression-test/suites/query_p0/aggregate/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // Basic Aggregate Tests (15 tests)

    #[test]
    fn test_count_all() {
        let sql = "SELECT COUNT(*) FROM users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_count_column() {
        let sql = "SELECT COUNT(id) FROM users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_count_distinct() {
        let sql = "SELECT COUNT(DISTINCT category) FROM products";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_sum_basic() {
        let sql = "SELECT SUM(amount) FROM orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_avg_basic() {
        let sql = "SELECT AVG(price) FROM products";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_min_basic() {
        let sql = "SELECT MIN(price) FROM products";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_max_basic() {
        let sql = "SELECT MAX(price) FROM products";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_multiple_aggregates() {
        let sql = r#"
            SELECT COUNT(*), SUM(amount), AVG(amount), MIN(amount), MAX(amount)
            FROM orders
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_aggregate_with_where() {
        let sql = r#"
            SELECT COUNT(*), SUM(amount)
            FROM orders
            WHERE status = 'completed'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_aggregate_with_null() {
        let sql = "SELECT COUNT(nullable_column), SUM(nullable_column) FROM table1";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_aggregate_distinct_multiple() {
        let sql = "SELECT COUNT(DISTINCT user_id), COUNT(DISTINCT product_id) FROM orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_sum_with_expression() {
        let sql = "SELECT SUM(price * quantity) AS total FROM order_items";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_avg_with_cast() {
        let sql = "SELECT AVG(CAST(value AS DOUBLE)) FROM measurements";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_count_with_case() {
        let sql = r#"
            SELECT COUNT(CASE WHEN status = 'active' THEN 1 END) AS active_count
            FROM users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_aggregate_subquery() {
        let sql = r#"
            SELECT (SELECT COUNT(*) FROM orders WHERE user_id = u.id) AS order_count
            FROM users u
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // GROUP BY Tests (15 tests)

    #[test]
    fn test_group_by_single_column() {
        let sql = r#"
            SELECT category, COUNT(*)
            FROM products
            GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_multiple_columns() {
        let sql = r#"
            SELECT category, brand, COUNT(*)
            FROM products
            GROUP BY category, brand
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_with_order() {
        let sql = r#"
            SELECT category, COUNT(*) AS cnt
            FROM products
            GROUP BY category
            ORDER BY cnt DESC
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_with_having() {
        let sql = r#"
            SELECT category, COUNT(*) AS cnt
            FROM products
            GROUP BY category
            HAVING COUNT(*) > 10
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_with_where_and_having() {
        let sql = r#"
            SELECT category, SUM(price) AS total
            FROM products
            WHERE active = true
            GROUP BY category
            HAVING SUM(price) > 1000
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_expression() {
        let sql = r#"
            SELECT EXTRACT(YEAR FROM order_date) AS year, COUNT(*)
            FROM orders
            GROUP BY EXTRACT(YEAR FROM order_date)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_case() {
        let sql = r#"
            SELECT
                CASE WHEN price < 100 THEN 'Low' ELSE 'High' END AS price_category,
                COUNT(*)
            FROM products
            GROUP BY CASE WHEN price < 100 THEN 'Low' ELSE 'High' END
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_with_limit() {
        let sql = r#"
            SELECT category, COUNT(*) AS cnt
            FROM products
            GROUP BY category
            ORDER BY cnt DESC
            LIMIT 10
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_all_aggregates() {
        let sql = r#"
            SELECT
                category,
                COUNT(*) AS cnt,
                SUM(price) AS total,
                AVG(price) AS avg_price,
                MIN(price) AS min_price,
                MAX(price) AS max_price
            FROM products
            GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_with_nulls() {
        let sql = r#"
            SELECT nullable_column, COUNT(*)
            FROM table1
            GROUP BY nullable_column
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_three_levels() {
        let sql = r#"
            SELECT year, month, day, COUNT(*)
            FROM events
            GROUP BY year, month, day
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_with_join() {
        let sql = r#"
            SELECT c.name, COUNT(o.id)
            FROM customers c
            LEFT JOIN orders o ON c.id = o.customer_id
            GROUP BY c.name
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_having_with_multiple_conditions() {
        let sql = r#"
            SELECT category, COUNT(*) AS cnt, SUM(price) AS total
            FROM products
            GROUP BY category
            HAVING COUNT(*) > 5 AND SUM(price) > 500
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_having_with_aggregate_expression() {
        let sql = r#"
            SELECT category, AVG(price) AS avg_price
            FROM products
            GROUP BY category
            HAVING AVG(price) BETWEEN 50 AND 150
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_group_by_rollup() {
        let sql = r#"
            SELECT category, brand, SUM(sales)
            FROM products
            GROUP BY ROLLUP(category, brand)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Window Function Tests (20 tests)

    #[test]
    fn test_row_number_basic() {
        let sql = r#"
            SELECT id, name, ROW_NUMBER() OVER (ORDER BY id) AS row_num
            FROM users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_row_number_partition() {
        let sql = r#"
            SELECT
                category,
                name,
                ROW_NUMBER() OVER (PARTITION BY category ORDER BY price DESC) AS rank_in_category
            FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_rank_basic() {
        let sql = r#"
            SELECT name, score, RANK() OVER (ORDER BY score DESC) AS rank
            FROM students
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_dense_rank() {
        let sql = r#"
            SELECT name, score, DENSE_RANK() OVER (ORDER BY score DESC) AS dense_rank
            FROM students
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_ntile() {
        let sql = r#"
            SELECT name, score, NTILE(4) OVER (ORDER BY score DESC) AS quartile
            FROM students
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_lag_basic() {
        let sql = r#"
            SELECT
                date,
                value,
                LAG(value, 1) OVER (ORDER BY date) AS prev_value
            FROM time_series
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_lag_with_default() {
        let sql = r#"
            SELECT
                date,
                value,
                LAG(value, 1, 0) OVER (ORDER BY date) AS prev_value
            FROM time_series
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_lead_basic() {
        let sql = r#"
            SELECT
                date,
                value,
                LEAD(value, 1) OVER (ORDER BY date) AS next_value
            FROM time_series
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_first_value() {
        let sql = r#"
            SELECT
                category,
                name,
                price,
                FIRST_VALUE(name) OVER (PARTITION BY category ORDER BY price) AS cheapest
            FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_last_value() {
        let sql = r#"
            SELECT
                category,
                name,
                price,
                LAST_VALUE(name) OVER (
                    PARTITION BY category
                    ORDER BY price
                    ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
                ) AS most_expensive
            FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_sum_window() {
        let sql = r#"
            SELECT
                date,
                amount,
                SUM(amount) OVER (ORDER BY date) AS running_total
            FROM transactions
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_avg_window() {
        let sql = r#"
            SELECT
                date,
                value,
                AVG(value) OVER (ORDER BY date ROWS BETWEEN 6 PRECEDING AND CURRENT ROW) AS moving_avg_7day
            FROM metrics
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_count_window() {
        let sql = r#"
            SELECT
                user_id,
                event_time,
                COUNT(*) OVER (PARTITION BY user_id ORDER BY event_time) AS event_sequence
            FROM user_events
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_min_max_window() {
        let sql = r#"
            SELECT
                date,
                value,
                MIN(value) OVER (ORDER BY date ROWS BETWEEN 29 PRECEDING AND CURRENT ROW) AS min_30day,
                MAX(value) OVER (ORDER BY date ROWS BETWEEN 29 PRECEDING AND CURRENT ROW) AS max_30day
            FROM daily_metrics
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_window_with_frame_range() {
        let sql = r#"
            SELECT
                id,
                value,
                SUM(value) OVER (ORDER BY id RANGE BETWEEN 5 PRECEDING AND 5 FOLLOWING) AS range_sum
            FROM table1
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_multiple_windows() {
        let sql = r#"
            SELECT
                category,
                name,
                price,
                ROW_NUMBER() OVER (PARTITION BY category ORDER BY price) AS rank_by_price,
                ROW_NUMBER() OVER (PARTITION BY category ORDER BY name) AS rank_by_name
            FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_window_with_named_window() {
        let sql = r#"
            SELECT
                category,
                name,
                price,
                ROW_NUMBER() OVER w AS row_num,
                RANK() OVER w AS rank
            FROM products
            WINDOW w AS (PARTITION BY category ORDER BY price DESC)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_percent_rank() {
        let sql = r#"
            SELECT
                name,
                score,
                PERCENT_RANK() OVER (ORDER BY score) AS percent_rank
            FROM students
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cume_dist() {
        let sql = r#"
            SELECT
                name,
                score,
                CUME_DIST() OVER (ORDER BY score) AS cumulative_distribution
            FROM students
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_window_with_complex_partition() {
        let sql = r#"
            SELECT
                year,
                quarter,
                month,
                revenue,
                SUM(revenue) OVER (PARTITION BY year, quarter ORDER BY month) AS quarterly_cumulative
            FROM monthly_revenue
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
