//! Materialized View Tests - MTMV and traditional MVs
//! Inspired by Doris regression-test/suites/mtmv_p0/ and mv_p0/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: Doris-specific MTMV (Multi-Table Materialized View) and some traditional MV
    // tests use Doris-specific syntax (BUILD, REFRESH, MTMV-specific DDL) which is not yet
    // supported by the DataFusion parser. These tests are marked as ignored.

    // Traditional Materialized View Tests (20 tests)

    #[test]
    fn test_create_rollup_mv() {
        let sql = r#"
            CREATE MATERIALIZED VIEW sales_by_date AS
            SELECT sale_date, SUM(amount) as total
            FROM sales
            GROUP BY sale_date
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_with_aggregation() {
        let sql = r#"
            CREATE MATERIALIZED VIEW user_summary AS
            SELECT
                user_id,
                COUNT(*) as order_count,
                SUM(total) as total_spent,
                AVG(total) as avg_order_value
            FROM orders
            GROUP BY user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_with_join() {
        let sql = r#"
            CREATE MATERIALIZED VIEW user_orders AS
            SELECT
                u.id,
                u.name,
                o.order_id,
                o.total
            FROM users u
            INNER JOIN orders o ON u.id = o.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_query_with_mv() {
        let sql = "SELECT * FROM sales_by_date WHERE sale_date >= '2024-01-01'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_drop_materialized_view() {
        let sql = "DROP MATERIALIZED VIEW sales_by_date";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_refresh_materialized_view() {
        let sql = "REFRESH MATERIALIZED VIEW sales_by_date";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_with_where() {
        let sql = r#"
            CREATE MATERIALIZED VIEW active_users AS
            SELECT id, name, email
            FROM users
            WHERE active = true AND verified = true
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_with_distinct() {
        let sql = r#"
            CREATE MATERIALIZED VIEW unique_categories AS
            SELECT DISTINCT category
            FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_count_distinct() {
        let sql = r#"
            CREATE MATERIALIZED VIEW category_user_count AS
            SELECT
                category,
                COUNT(DISTINCT user_id) as unique_users
            FROM purchases
            GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_min_max() {
        let sql = r#"
            CREATE MATERIALIZED VIEW product_price_range AS
            SELECT
                category,
                MIN(price) as min_price,
                MAX(price) as max_price
            FROM products
            GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_with_having() {
        let sql = r#"
            CREATE MATERIALIZED VIEW high_value_customers AS
            SELECT
                user_id,
                SUM(total) as total_spent
            FROM orders
            GROUP BY user_id
            HAVING SUM(total) > 10000
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_with_order_by() {
        let sql = r#"
            CREATE MATERIALIZED VIEW top_products AS
            SELECT product_id, SUM(quantity) as total_sold
            FROM order_items
            GROUP BY product_id
            ORDER BY total_sold DESC
            LIMIT 100
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_multi_table_join() {
        let sql = r#"
            CREATE MATERIALIZED VIEW order_details AS
            SELECT
                o.id,
                u.name as customer_name,
                p.name as product_name,
                oi.quantity,
                oi.price
            FROM orders o
            JOIN users u ON o.user_id = u.id
            JOIN order_items oi ON o.id = oi.order_id
            JOIN products p ON oi.product_id = p.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_with_case() {
        let sql = r#"
            CREATE MATERIALIZED VIEW user_segments AS
            SELECT
                user_id,
                CASE
                    WHEN total_spent > 10000 THEN 'VIP'
                    WHEN total_spent > 1000 THEN 'Premium'
                    ELSE 'Regular'
                END as segment,
                COUNT(*) as count
            FROM (
                SELECT user_id, SUM(total) as total_spent
                FROM orders
                GROUP BY user_id
            ) t
            GROUP BY user_id, segment
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_mv_with_date_functions() {
        let sql = r#"
            CREATE MATERIALIZED VIEW monthly_sales AS
            SELECT
                EXTRACT(YEAR FROM sale_date) as year,
                EXTRACT(MONTH FROM sale_date) as month,
                SUM(amount) as total
            FROM sales
            GROUP BY year, month
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_mv_with_partition() {
        let sql = r#"
            CREATE MATERIALIZED VIEW partitioned_mv
            PARTITION BY date_col AS
            SELECT date_col, SUM(value) as total
            FROM fact_table
            GROUP BY date_col
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_show_materialized_views() {
        let sql = "SHOW MATERIALIZED VIEWS";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_describe_materialized_view() {
        let sql = "DESCRIBE sales_by_date";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_show_create_materialized_view() {
        let sql = "SHOW CREATE MATERIALIZED VIEW sales_by_date";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_alter_materialized_view_properties() {
        let sql = r#"
            ALTER MATERIALIZED VIEW sales_by_date
            SET ("replication_num" = "3")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // MTMV (Multi-Table Materialized View) Tests (30 tests)

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_basic() {
        let sql = r#"
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_orders
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            DISTRIBUTED BY RANDOM BUCKETS 2
            PROPERTIES ('replication_num' = '1')
            AS
            SELECT * FROM orders
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_with_join() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_order_summary
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            AS
            SELECT
                o.order_date,
                u.name,
                SUM(o.total) as total_amount
            FROM orders o
            JOIN users u ON o.user_id = u.id
            GROUP BY o.order_date, u.name
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_with_partition() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_sales_partitioned
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            PARTITION BY (order_date)
            AS
            SELECT
                order_date,
                SUM(amount) as total
            FROM sales
            GROUP BY order_date
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_deferred_build() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_deferred
            BUILD DEFERRED REFRESH AUTO ON MANUAL
            AS
            SELECT * FROM large_table
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_auto_refresh() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_auto_refresh
            BUILD IMMEDIATE REFRESH AUTO ON SCHEDULE EVERY 1 HOUR
            AS
            SELECT category, COUNT(*) as count
            FROM products
            GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_on_commit() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_on_commit
            BUILD IMMEDIATE REFRESH ON COMMIT
            AS
            SELECT * FROM realtime_data
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_with_where() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_filtered
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            AS
            SELECT *
            FROM transactions
            WHERE status = 'completed' AND amount > 100
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_with_union() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_combined
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            AS
            SELECT id, name FROM users_table1
            UNION
            SELECT id, name FROM users_table2
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_with_subquery() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_with_subquery
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            AS
            SELECT
                user_id,
                total_orders,
                CASE
                    WHEN total_orders > 100 THEN 'High'
                    WHEN total_orders > 10 THEN 'Medium'
                    ELSE 'Low'
                END as tier
            FROM (
                SELECT user_id, COUNT(*) as total_orders
                FROM orders
                GROUP BY user_id
            ) t
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_create_mtmv_with_window_function() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_ranked
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            AS
            SELECT
                product_id,
                sale_date,
                amount,
                ROW_NUMBER() OVER (PARTITION BY product_id ORDER BY sale_date) as row_num
            FROM sales
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_refresh_mtmv() {
        let sql = "REFRESH MATERIALIZED VIEW mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_refresh_mtmv_complete() {
        let sql = "REFRESH MATERIALIZED VIEW mv_orders COMPLETE";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_refresh_mtmv_partition() {
        let sql = "REFRESH MATERIALIZED VIEW mv_sales_partitioned PARTITION (p20240101)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_refresh_mtmv_auto() {
        let sql = "REFRESH MATERIALIZED VIEW mv_orders AUTO";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_alter_mtmv_refresh_mode() {
        let sql = r#"
            ALTER MATERIALIZED VIEW mv_orders
            REFRESH AUTO ON SCHEDULE EVERY 2 HOUR
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_alter_mtmv_properties() {
        let sql = r#"
            ALTER MATERIALIZED VIEW mv_orders
            SET ('replication_num' = '2')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_drop_mtmv() {
        let sql = "DROP MATERIALIZED VIEW mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_drop_mtmv_if_exists() {
        let sql = "DROP MATERIALIZED VIEW IF EXISTS mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_show_mtmv() {
        let sql = "SHOW MATERIALIZED VIEW mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_show_all_mtmv() {
        let sql = "SHOW MATERIALIZED VIEWS FROM database_name";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_describe_mtmv() {
        let sql = "DESC MATERIALIZED VIEW mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_show_create_mtmv() {
        let sql = "SHOW CREATE MATERIALIZED VIEW mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_mtmv_task_status() {
        let sql = "SHOW MTMV TASK ON mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_pause_mtmv() {
        let sql = "PAUSE MATERIALIZED VIEW JOB ON mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_resume_mtmv() {
        let sql = "RESUME MATERIALIZED VIEW JOB ON mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_cancel_mtmv_task() {
        let sql = "CANCEL MATERIALIZED VIEW TASK ON mv_orders";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_mtmv_with_properties_in_select() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_with_props
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            PROPERTIES (
                'replication_num' = '1',
                'storage_medium' = 'SSD',
                'storage_cooldown_time' = '2024-12-31 23:59:59'
            )
            AS
            SELECT * FROM hot_data
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_mtmv_with_distributed_by_hash() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_distributed
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            DISTRIBUTED BY HASH(id) BUCKETS 10
            AS
            SELECT id, name, value FROM source_table
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_mtmv_query_rewrite() {
        let sql = r#"
            SELECT order_date, total_amount
            FROM mv_order_summary
            WHERE order_date >= '2024-01-01'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]  // Doris-specific MTMV syntax
    fn test_mtmv_with_cte() {
        let sql = r#"
            CREATE MATERIALIZED VIEW mv_with_cte
            BUILD IMMEDIATE REFRESH AUTO ON MANUAL
            AS
            WITH active_users AS (
                SELECT id, name FROM users WHERE active = true
            )
            SELECT au.id, au.name, COUNT(o.id) as order_count
            FROM active_users au
            LEFT JOIN orders o ON au.id = o.user_id
            GROUP BY au.id, au.name
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
