//! DML (Data Manipulation Language) Tests - INSERT, UPDATE, DELETE, MERGE
//! Inspired by Doris regression-test/suites/insert_p0, delete_p0, update/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // INSERT Tests (30 tests)

    #[test]
    fn test_insert_values_basic() {
        let sql = r#"
            INSERT INTO users (id, name, age)
            VALUES (1, 'Alice', 30)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_multiple_values() {
        let sql = r#"
            INSERT INTO users (id, name, age)
            VALUES
                (1, 'Alice', 30),
                (2, 'Bob', 25),
                (3, 'Charlie', 35)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_select() {
        let sql = r#"
            INSERT INTO users_backup
            SELECT * FROM users WHERE age > 25
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_select_with_columns() {
        let sql = r#"
            INSERT INTO users_summary (id, name)
            SELECT id, name FROM users WHERE active = true
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_defaults() {
        let sql = r#"
            INSERT INTO users (id, name)
            VALUES (1, 'Alice')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_into_partition() {
        let sql = r#"
            INSERT INTO sales PARTITION (p202401)
            VALUES (1, '2024-01-15', 1000.00)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_insert_with_label() {
        let sql = r#"
            INSERT INTO users WITH LABEL label_123
            VALUES (1, 'Alice', 30)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_null_values() {
        let sql = r#"
            INSERT INTO users (id, name, phone)
            VALUES (1, 'Alice', NULL)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_expressions() {
        let sql = r#"
            INSERT INTO users (id, name, created_at)
            VALUES (1, UPPER('alice'), CURRENT_TIMESTAMP)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_overwrite() {
        let sql = r#"
            INSERT OVERWRITE TABLE users
            VALUES (1, 'Alice', 30)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_overwrite_partition() {
        let sql = r#"
            INSERT OVERWRITE TABLE sales PARTITION (p202401)
            SELECT * FROM staging_sales WHERE sale_date >= '2024-01-01'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_subquery() {
        let sql = r#"
            INSERT INTO summary
            SELECT
                category,
                COUNT(*) as count,
                SUM(amount) as total
            FROM transactions
            GROUP BY category
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_join() {
        let sql = r#"
            INSERT INTO user_orders
            SELECT u.id, u.name, o.order_id, o.total
            FROM users u
            INNER JOIN orders o ON u.id = o.user_id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_cte() {
        let sql = r#"
            INSERT INTO summary
            WITH active_users AS (
                SELECT id, name FROM users WHERE active = true
            )
            SELECT id, name FROM active_users
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_union() {
        let sql = r#"
            INSERT INTO combined
            SELECT id, name FROM table1
            UNION
            SELECT id, name FROM table2
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_array_values() {
        let sql = r#"
            INSERT INTO tags_table (id, tags)
            VALUES (1, ['tag1', 'tag2', 'tag3'])
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_map_values() {
        let sql = r#"
            INSERT INTO attributes_table (id, attrs)
            VALUES (1, MAP('key1', 'value1', 'key2', 'value2'))
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_json_values() {
        let sql = r#"
            INSERT INTO json_table (id, data)
            VALUES (1, '{"name": "Alice", "age": 30}')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_large_batch() {
        let sql = r#"
            INSERT INTO large_table (id, value)
            SELECT id, REPEAT('x', 100) FROM numbers(1000)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_duplicate_handling() {
        let sql = r#"
            INSERT INTO users (id, name, age)
            VALUES (1, 'Alice', 30)
            ON DUPLICATE KEY UPDATE age = VALUES(age)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_returning() {
        let sql = r#"
            INSERT INTO users (name, age)
            VALUES ('Alice', 30)
            RETURNING id, name
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_all_columns() {
        let sql = r#"
            INSERT INTO users
            VALUES (1, 'Alice', 30, 'alice@example.com', CURRENT_TIMESTAMP)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_partial_columns() {
        let sql = r#"
            INSERT INTO users (id, name)
            VALUES (1, 'Alice')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_cast() {
        let sql = r#"
            INSERT INTO users (id, name, age)
            VALUES (CAST('1' AS INT), 'Alice', CAST('30' AS INT))
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_case() {
        let sql = r#"
            INSERT INTO categorized
            SELECT
                id,
                CASE
                    WHEN value > 100 THEN 'High'
                    WHEN value > 50 THEN 'Medium'
                    ELSE 'Low'
                END as category
            FROM source_table
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_window_function() {
        let sql = r#"
            INSERT INTO ranked
            SELECT
                id,
                name,
                ROW_NUMBER() OVER (ORDER BY score DESC) as rank
            FROM scores
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_aggregation() {
        let sql = r#"
            INSERT INTO daily_summary
            SELECT
                DATE(timestamp) as date,
                COUNT(*) as count,
                SUM(amount) as total
            FROM transactions
            GROUP BY DATE(timestamp)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_cross_database() {
        let sql = r#"
            INSERT INTO db1.table1
            SELECT * FROM db2.table2
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_with_filter() {
        let sql = r#"
            INSERT INTO filtered
            SELECT * FROM source
            WHERE created_at > DATE '2024-01-01'
            AND status = 'active'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_insert_distinct() {
        let sql = r#"
            INSERT INTO unique_values
            SELECT DISTINCT category FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // UPDATE Tests (20 tests)

    #[test]
    fn test_update_basic() {
        let sql = "UPDATE users SET age = 31 WHERE id = 1";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_multiple_columns() {
        let sql = r#"
            UPDATE users
            SET age = 31, name = 'Alice Smith', updated_at = CURRENT_TIMESTAMP
            WHERE id = 1
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_expression() {
        let sql = "UPDATE products SET price = price * 1.1 WHERE category = 'Electronics'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_case() {
        let sql = r#"
            UPDATE users
            SET status = CASE
                WHEN age >= 18 THEN 'adult'
                ELSE 'minor'
            END
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_subquery() {
        let sql = r#"
            UPDATE users
            SET total_spent = (
                SELECT SUM(amount) FROM orders WHERE orders.user_id = users.id
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_join() {
        let sql = r#"
            UPDATE users u
            SET u.status = 'premium'
            FROM orders o
            WHERE u.id = o.user_id
            AND o.total > 1000
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_all_rows() {
        let sql = "UPDATE users SET active = true";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_update_with_limit() {
        let sql = "UPDATE users SET processed = true WHERE active = false LIMIT 100";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_increment() {
        let sql = "UPDATE counters SET value = value + 1 WHERE id = 1";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_decrement() {
        let sql = "UPDATE inventory SET quantity = quantity - 1 WHERE product_id = 100";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_coalesce() {
        let sql = r#"
            UPDATE users
            SET phone = COALESCE(phone, 'N/A')
            WHERE phone IS NULL
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_concat() {
        let sql = "UPDATE users SET full_name = CONCAT(first_name, ' ', last_name)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_date_functions() {
        let sql = r#"
            UPDATE subscriptions
            SET expires_at = DATE_ADD(created_at, INTERVAL 1 YEAR)
            WHERE type = 'annual'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_in_clause() {
        let sql = r#"
            UPDATE products
            SET discount = 0.2
            WHERE category IN ('Electronics', 'Clothing', 'Books')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_between() {
        let sql = r#"
            UPDATE products
            SET tier = 'premium'
            WHERE price BETWEEN 100 AND 500
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_like() {
        let sql = r#"
            UPDATE users
            SET verified = true
            WHERE email LIKE '%@company.com'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_null_to_value() {
        let sql = "UPDATE users SET status = 'inactive' WHERE status IS NULL";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_value_to_null() {
        let sql = "UPDATE users SET deleted_at = NULL WHERE restored = true";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_with_exists() {
        let sql = r#"
            UPDATE users
            SET has_orders = true
            WHERE EXISTS (SELECT 1 FROM orders WHERE orders.user_id = users.id)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_update_partition() {
        let sql = r#"
            UPDATE sales PARTITION (p202401)
            SET processed = true
            WHERE sale_date < '2024-01-15'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // DELETE Tests (20 tests)

    #[test]
    fn test_delete_basic() {
        let sql = "DELETE FROM users WHERE id = 1";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_multiple_conditions() {
        let sql = r#"
            DELETE FROM users
            WHERE age > 65 AND active = false AND last_login < DATE '2023-01-01'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_in() {
        let sql = "DELETE FROM users WHERE id IN (1, 2, 3, 4, 5)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_subquery() {
        let sql = r#"
            DELETE FROM users
            WHERE id IN (SELECT user_id FROM blocked_users)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_not_in() {
        let sql = r#"
            DELETE FROM temp_users
            WHERE id NOT IN (SELECT id FROM permanent_users)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_exists() {
        let sql = r#"
            DELETE FROM users
            WHERE NOT EXISTS (
                SELECT 1 FROM orders WHERE orders.user_id = users.id
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_delete_with_join() {
        let sql = r#"
            DELETE u
            FROM users u
            INNER JOIN banned_list b ON u.email = b.email
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_limit() {
        let sql = "DELETE FROM logs WHERE created_at < DATE '2024-01-01' LIMIT 1000";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_old_records() {
        let sql = "DELETE FROM sessions WHERE created_at < NOW() - INTERVAL 30 DAY";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_duplicate_records() {
        let sql = r#"
            DELETE FROM users
            WHERE id NOT IN (
                SELECT MIN(id) FROM users GROUP BY email
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_null_values() {
        let sql = "DELETE FROM users WHERE email IS NULL";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_between() {
        let sql = "DELETE FROM transactions WHERE amount BETWEEN 0.01 AND 1.00";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_like() {
        let sql = "DELETE FROM users WHERE email LIKE '%@spam.com'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_from_partition() {
        let sql = r#"
            DELETE FROM sales PARTITION (p202401)
            WHERE amount < 10
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_or_conditions() {
        let sql = r#"
            DELETE FROM users
            WHERE status = 'banned' OR status = 'deleted' OR status = 'suspended'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_and_or_combination() {
        let sql = r#"
            DELETE FROM products
            WHERE (category = 'Electronics' AND price < 10)
            OR (category = 'Books' AND stock = 0)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_with_not_exists() {
        let sql = r#"
            DELETE FROM temp_data
            WHERE NOT EXISTS (
                SELECT 1 FROM master_data WHERE master_data.id = temp_data.id
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_cascade_effect() {
        let sql = "DELETE FROM users WHERE user_type = 'test'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_soft_delete() {
        let sql = "UPDATE users SET deleted_at = CURRENT_TIMESTAMP WHERE id = 1";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_delete_all_records() {
        let sql = "DELETE FROM temp_table";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
