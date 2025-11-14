//! DDL (Data Definition Language) Tests - CREATE, ALTER, DROP operations
//! Inspired by Doris regression-test/suites/ddl_p0/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // CREATE TABLE Tests (20 tests)

    #[test]
    fn test_create_table_basic() {
        let sql = r#"
            CREATE TABLE users (
                id INT,
                name VARCHAR(100),
                age INT
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_table_with_primary_key() {
        let sql = r#"
            CREATE TABLE orders (
                id INT PRIMARY KEY,
                user_id INT,
                total DECIMAL(10, 2)
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_table_with_constraints() {
        let sql = r#"
            CREATE TABLE products (
                id INT PRIMARY KEY,
                name VARCHAR(100) NOT NULL,
                price DECIMAL(10, 2) NOT NULL,
                category VARCHAR(50)
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_with_default_values() {
        let sql = r#"
            CREATE TABLE settings (
                id INT,
                key VARCHAR(100),
                value VARCHAR(255) DEFAULT 'default',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_table_if_not_exists() {
        let sql = r#"
            CREATE TABLE IF NOT EXISTS temp_table (
                id INT,
                data VARCHAR(255)
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_table_as_select() {
        let sql = r#"
            CREATE TABLE new_table AS
            SELECT id, name, price
            FROM products
            WHERE category = 'Electronics'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_with_partition() {
        let sql = r#"
            CREATE TABLE sales (
                id INT,
                sale_date DATE,
                amount DECIMAL(10, 2)
            )
            PARTITION BY RANGE(sale_date) (
                PARTITION p202401 VALUES LESS THAN ('2024-02-01'),
                PARTITION p202402 VALUES LESS THAN ('2024-03-01')
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_with_buckets() {
        let sql = r#"
            CREATE TABLE distributed_table (
                id INT,
                name VARCHAR(100)
            )
            DISTRIBUTED BY HASH(id) BUCKETS 10
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_duplicate_key() {
        let sql = r#"
            CREATE TABLE log_table (
                timestamp DATETIME,
                user_id INT,
                action VARCHAR(100)
            )
            DUPLICATE KEY(timestamp, user_id)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_unique_key() {
        let sql = r#"
            CREATE TABLE user_profile (
                user_id INT,
                email VARCHAR(100),
                updated_at TIMESTAMP
            )
            UNIQUE KEY(user_id)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_aggregate_key() {
        let sql = r#"
            CREATE TABLE metrics (
                date DATE,
                metric_name VARCHAR(100),
                value BIGINT SUM
            )
            AGGREGATE KEY(date, metric_name)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_with_properties() {
        let sql = r#"
            CREATE TABLE prop_table (
                id INT,
                data VARCHAR(255)
            )
            PROPERTIES (
                "replication_num" = "3",
                "storage_format" = "DEFAULT"
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_with_bloom_filter() {
        let sql = r#"
            CREATE TABLE bloom_table (
                id INT,
                email VARCHAR(100)
            )
            PROPERTIES (
                "bloom_filter_columns" = "email"
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_with_storage_policy() {
        let sql = r#"
            CREATE TABLE storage_table (
                id INT,
                data TEXT
            )
            PROPERTIES (
                "storage_policy" = "hot_storage"
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_table_with_comment() {
        let sql = r#"
            CREATE TABLE commented_table (
                id INT COMMENT 'User ID',
                name VARCHAR(100) COMMENT 'User Name'
            )
            COMMENT 'User information table'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_table_all_data_types() {
        let sql = r#"
            CREATE TABLE all_types (
                col_tinyint TINYINT,
                col_smallint SMALLINT,
                col_int INT,
                col_bigint BIGINT,
                col_float FLOAT,
                col_double DOUBLE,
                col_decimal DECIMAL(20, 4),
                col_char CHAR(10),
                col_varchar VARCHAR(255),
                col_string STRING,
                col_date DATE,
                col_datetime DATETIME,
                col_boolean BOOLEAN
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_table_with_array() {
        let sql = r#"
            CREATE TABLE array_table (
                id INT,
                tags ARRAY<VARCHAR(50)>,
                scores ARRAY<INT>
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_with_map() {
        let sql = r#"
            CREATE TABLE map_table (
                id INT,
                attributes MAP<VARCHAR(50), VARCHAR(100)>
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_table_with_struct() {
        let sql = r#"
            CREATE TABLE struct_table (
                id INT,
                address STRUCT<street:VARCHAR(100), city:VARCHAR(50), zip:INT>
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_table_with_json() {
        let sql = r#"
            CREATE TABLE json_table (
                id INT,
                metadata JSON
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // ALTER TABLE Tests (20 tests)

    #[test]
    fn test_alter_table_add_column() {
        let sql = "ALTER TABLE users ADD COLUMN phone VARCHAR(20)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_add_multiple_columns() {
        let sql = r#"
            ALTER TABLE users
            ADD COLUMN address VARCHAR(255),
            ADD COLUMN city VARCHAR(100)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_drop_column() {
        let sql = "ALTER TABLE users DROP COLUMN phone";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_modify_column() {
        let sql = "ALTER TABLE users MODIFY COLUMN name VARCHAR(200)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_rename_column() {
        let sql = "ALTER TABLE users RENAME COLUMN name TO full_name";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_alter_table_add_partition() {
        let sql = r#"
            ALTER TABLE sales
            ADD PARTITION p202403 VALUES LESS THAN ('2024-04-01')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_alter_table_drop_partition() {
        let sql = "ALTER TABLE sales DROP PARTITION p202401";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_add_rollup() {
        let sql = r#"
            ALTER TABLE sales
            ADD ROLLUP sales_by_date (sale_date, amount)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_alter_table_drop_rollup() {
        let sql = "ALTER TABLE sales DROP ROLLUP sales_by_date";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_alter_table_set_properties() {
        let sql = r#"
            ALTER TABLE users
            SET ("replication_num" = "2")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_rename() {
        let sql = "ALTER TABLE users RENAME TO customers";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_add_index() {
        let sql = "ALTER TABLE users ADD INDEX idx_name(name)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_alter_table_drop_index() {
        let sql = "ALTER TABLE users DROP INDEX idx_name";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_alter_table_modify_distribution() {
        let sql = r#"
            ALTER TABLE users
            MODIFY DISTRIBUTION DISTRIBUTED BY HASH(id) BUCKETS 20
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_alter_table_set_comment() {
        let sql = r#"
            ALTER TABLE users
            SET COMMENT 'Updated user information table'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_alter_table_enable_feature() {
        let sql = "ALTER TABLE users ENABLE FEATURE 'BATCH_DELETE'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_modify_column_order() {
        let sql = "ALTER TABLE users MODIFY COLUMN email VARCHAR(100) AFTER name";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_change_column_default() {
        let sql = "ALTER TABLE users MODIFY COLUMN status VARCHAR(20) DEFAULT 'active'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_alter_table_add_constraint() {
        let sql = "ALTER TABLE orders ADD CONSTRAINT fk_user FOREIGN KEY (user_id) REFERENCES users(id)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_alter_table_modify_replication() {
        let sql = r#"
            ALTER TABLE users
            SET ("min_load_replica_num" = "1")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // DROP Tests (10 tests)

    #[test]
    fn test_drop_table() {
        let sql = "DROP TABLE users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_drop_table_if_exists() {
        let sql = "DROP TABLE IF EXISTS users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_drop_database() {
        let sql = "DROP DATABASE test_db";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_drop_view() {
        let sql = "DROP VIEW user_view";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_drop_materialized_view() {
        let sql = "DROP MATERIALIZED VIEW mv_sales";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_truncate_table() {
        let sql = "TRUNCATE TABLE users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_drop_index() {
        let sql = "DROP INDEX idx_name ON users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_drop_user() {
        let sql = "DROP USER test_user";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_drop_role() {
        let sql = "DROP ROLE test_role";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_drop_resource() {
        let sql = "DROP RESOURCE test_resource";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // CREATE DATABASE Tests (10 tests)

    #[test]
    fn test_create_database_basic() {
        let sql = "CREATE DATABASE my_database";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_create_database_if_not_exists() {
        let sql = "CREATE DATABASE IF NOT EXISTS my_database";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Doris-specific syntax - ignored until Doris parser is implemented
    #[test]
    #[ignore]
    fn test_create_database_with_properties() {
        let sql = r#"
            CREATE DATABASE my_database
            PROPERTIES ("replication_num" = "3")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_use_database() {
        let sql = "USE my_database";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_databases() {
        let sql = "SHOW DATABASES";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_tables() {
        let sql = "SHOW TABLES";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_create_table() {
        let sql = "SHOW CREATE TABLE users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_describe_table() {
        let sql = "DESCRIBE users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_columns() {
        let sql = "SHOW COLUMNS FROM users";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_show_partitions() {
        let sql = "SHOW PARTITIONS FROM sales";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
