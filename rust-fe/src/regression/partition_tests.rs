//! Partition Management Tests
//! Inspired by Doris regression-test/suites/partition_p0/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: All partition tests use Doris-specific PARTITION BY RANGE/LIST syntax
    // which is not yet supported by the DataFusion parser. These tests are ignored
    // until the Doris-specific parser is implemented.

    // Range Partitioning Tests (15 tests)

    #[test]
    #[ignore]
    fn test_create_range_partition_table() {
        let sql = r#"
            CREATE TABLE sales (
                id INT,
                sale_date DATE,
                amount DECIMAL(10, 2)
            )
            PARTITION BY RANGE(sale_date) (
                PARTITION p20231 VALUES LESS THAN ('2023-02-01'),
                PARTITION p20232 VALUES LESS THAN ('2023-03-01'),
                PARTITION p20233 VALUES LESS THAN ('2023-04-01')
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_range_partition_by_int() {
        let sql = r#"
            CREATE TABLE users (
                id INT,
                name VARCHAR(100)
            )
            PARTITION BY RANGE(id) (
                PARTITION p1 VALUES LESS THAN (1000),
                PARTITION p2 VALUES LESS THAN (2000),
                PARTITION p3 VALUES LESS THAN (MAXVALUE)
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_add_range_partition() {
        let sql = r#"
            ALTER TABLE sales
            ADD PARTITION p20234 VALUES LESS THAN ('2023-05-01')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_drop_partition() {
        let sql = "ALTER TABLE sales DROP PARTITION p20231";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_drop_partition_if_exists() {
        let sql = "ALTER TABLE sales DROP PARTITION IF EXISTS p20231";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_add_temp_partition() {
        let sql = r#"
            ALTER TABLE sales
            ADD TEMPORARY PARTITION tp1 VALUES LESS THAN ('2023-12-01')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_replace_partition() {
        let sql = r#"
            ALTER TABLE sales
            REPLACE PARTITION (p20231) WITH TEMPORARY PARTITION (tp1)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_truncate_partition() {
        let sql = "ALTER TABLE sales TRUNCATE PARTITION p20231";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_query_specific_partition() {
        let sql = "SELECT * FROM sales PARTITION (p20231)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_query_multiple_partitions() {
        let sql = "SELECT * FROM sales PARTITION (p20231, p20232)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_partition_pruning() {
        let sql = r#"
            SELECT * FROM sales
            WHERE sale_date >= '2023-02-01' AND sale_date < '2023-03-01'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_range_partition_multi_column() {
        let sql = r#"
            CREATE TABLE events (
                year INT,
                month INT,
                data VARCHAR(100)
            )
            PARTITION BY RANGE(year, month) (
                PARTITION p202301 VALUES LESS THAN (2023, 2),
                PARTITION p202302 VALUES LESS THAN (2023, 3)
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_add_partition_with_properties() {
        let sql = r#"
            ALTER TABLE sales
            ADD PARTITION p20235 VALUES LESS THAN ('2023-06-01')
            ("replication_num" = "3")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_modify_partition_properties() {
        let sql = r#"
            ALTER TABLE sales
            MODIFY PARTITION p20231 SET ("replication_num" = "2")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_show_partitions() {
        let sql = "SHOW PARTITIONS FROM sales";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // List Partitioning Tests (10 tests)

    #[test]
    #[ignore]
    fn test_create_list_partition_table() {
        let sql = r#"
            CREATE TABLE regions (
                id INT,
                region VARCHAR(50),
                data VARCHAR(100)
            )
            PARTITION BY LIST(region) (
                PARTITION p_north VALUES IN ('US', 'CA', 'MX'),
                PARTITION p_south VALUES IN ('BR', 'AR', 'CL'),
                PARTITION p_europe VALUES IN ('UK', 'DE', 'FR')
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_list_partition_single_value() {
        let sql = r#"
            CREATE TABLE status_table (
                id INT,
                status VARCHAR(20),
                data VARCHAR(100)
            )
            PARTITION BY LIST(status) (
                PARTITION p_active VALUES IN ('active'),
                PARTITION p_inactive VALUES IN ('inactive'),
                PARTITION p_pending VALUES IN ('pending')
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_add_list_partition() {
        let sql = r#"
            ALTER TABLE regions
            ADD PARTITION p_asia VALUES IN ('JP', 'CN', 'KR')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_list_partition_default() {
        let sql = r#"
            CREATE TABLE categories (
                id INT,
                category VARCHAR(50)
            )
            PARTITION BY LIST(category) (
                PARTITION p_electronics VALUES IN ('phone', 'laptop'),
                PARTITION p_default VALUES IN (DEFAULT)
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_query_list_partition() {
        let sql = "SELECT * FROM regions PARTITION (p_north)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_list_partition_pruning() {
        let sql = "SELECT * FROM regions WHERE region = 'US'";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_list_partition_multi_value_query() {
        let sql = "SELECT * FROM regions WHERE region IN ('US', 'CA')";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_drop_list_partition() {
        let sql = "ALTER TABLE regions DROP PARTITION p_south";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_modify_list_partition_values() {
        let sql = r#"
            ALTER TABLE regions
            MODIFY PARTITION p_north ADD VALUES ('GL')
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_list_partition_integer() {
        let sql = r#"
            CREATE TABLE priority_queue (
                id INT,
                priority INT,
                data VARCHAR(100)
            )
            PARTITION BY LIST(priority) (
                PARTITION p_high VALUES IN (1, 2),
                PARTITION p_medium VALUES IN (3, 4, 5),
                PARTITION p_low VALUES IN (6, 7, 8, 9, 10)
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Dynamic Partitioning Tests (10 tests)

    #[test]
    #[ignore]
    fn test_dynamic_partition_enable() {
        let sql = r#"
            CREATE TABLE auto_partitioned (
                id INT,
                created_at DATETIME,
                data VARCHAR(100)
            )
            PARTITION BY RANGE(created_at) ()
            PROPERTIES (
                "dynamic_partition.enable" = "true",
                "dynamic_partition.time_unit" = "DAY",
                "dynamic_partition.start" = "-7",
                "dynamic_partition.end" = "3",
                "dynamic_partition.prefix" = "p",
                "dynamic_partition.buckets" = "10"
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_dynamic_partition_by_week() {
        let sql = r#"
            ALTER TABLE auto_partitioned
            SET (
                "dynamic_partition.time_unit" = "WEEK",
                "dynamic_partition.start" = "-4",
                "dynamic_partition.end" = "2"
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_dynamic_partition_by_month() {
        let sql = r#"
            ALTER TABLE auto_partitioned
            SET (
                "dynamic_partition.time_unit" = "MONTH",
                "dynamic_partition.start" = "-3",
                "dynamic_partition.end" = "1"
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_dynamic_partition_disable() {
        let sql = r#"
            ALTER TABLE auto_partitioned
            SET ("dynamic_partition.enable" = "false")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_dynamic_partition_hot_partition_num() {
        let sql = r#"
            ALTER TABLE auto_partitioned
            SET ("dynamic_partition.hot_partition_num" = "3")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_dynamic_partition_with_replication() {
        let sql = r#"
            ALTER TABLE auto_partitioned
            SET (
                "dynamic_partition.replication_num" = "3",
                "dynamic_partition.replication_allocation" = "tag.location.default: 3"
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_dynamic_partition_start_day_of_week() {
        let sql = r#"
            ALTER TABLE auto_partitioned
            SET ("dynamic_partition.start_day_of_week" = "1")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_dynamic_partition_start_day_of_month() {
        let sql = r#"
            ALTER TABLE auto_partitioned
            SET ("dynamic_partition.start_day_of_month" = "1")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_dynamic_partition_history_partition_num() {
        let sql = r#"
            ALTER TABLE auto_partitioned
            SET ("dynamic_partition.history_partition_num" = "30")
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_show_dynamic_partition_info() {
        let sql = "SHOW DYNAMIC PARTITION TABLES";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Partition Operations (15 tests)

    #[test]
    #[ignore]
    fn test_insert_into_partition() {
        let sql = r#"
            INSERT INTO sales PARTITION (p20231)
            VALUES (1, '2023-01-15', 1000.00)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_insert_auto_partition() {
        let sql = r#"
            INSERT INTO sales
            VALUES (1, '2023-01-15', 1000.00)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_update_partition() {
        let sql = r#"
            UPDATE sales PARTITION (p20231)
            SET amount = amount * 1.1
            WHERE id < 100
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_delete_from_partition() {
        let sql = r#"
            DELETE FROM sales PARTITION (p20231)
            WHERE amount < 10
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_count_partition() {
        let sql = "SELECT COUNT(*) FROM sales PARTITION (p20231)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_aggregate_partition() {
        let sql = r#"
            SELECT
                COUNT(*) as count,
                SUM(amount) as total,
                AVG(amount) as average
            FROM sales PARTITION (p20231, p20232)
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_join_across_partitions() {
        let sql = r#"
            SELECT s1.*, s2.*
            FROM sales PARTITION (p20231) s1
            INNER JOIN sales PARTITION (p20232) s2 ON s1.id = s2.id
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_partition_statistics() {
        let sql = r#"
            SELECT PARTITION_NAME, PARTITION_METHOD, PARTITION_DESCRIPTION
            FROM information_schema.partitions
            WHERE TABLE_NAME = 'sales'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_partition_size() {
        let sql = r#"
            SELECT
                PARTITION_NAME,
                TABLE_ROWS,
                DATA_LENGTH,
                INDEX_LENGTH
            FROM information_schema.partitions
            WHERE TABLE_NAME = 'sales'
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_reorganize_partition() {
        let sql = r#"
            ALTER TABLE sales
            REORGANIZE PARTITION p20231, p20232 INTO (
                PARTITION p2023q1 VALUES LESS THAN ('2023-04-01')
            )
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_exchange_partition() {
        let sql = r#"
            ALTER TABLE sales
            EXCHANGE PARTITION p20231 WITH TABLE sales_backup
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_analyze_partition() {
        let sql = "ANALYZE TABLE sales PARTITION (p20231)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_partition_recovery() {
        let sql = "ADMIN REPAIR TABLE sales PARTITION (p20231)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_partition_balance() {
        let sql = "ADMIN BALANCE DISK";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    #[ignore]
    fn test_check_partition_health() {
        let sql = "ADMIN CHECK TABLET (12345)";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
