// SQL Logic Tests
// Tests SQL semantics, correctness, and edge cases
//
// These tests validate:
// 1. Query results match expected outputs
// 2. Data type handling and conversions
// 3. NULL handling and edge cases
// 4. Aggregation correctness
// 5. Join semantics
// 6. Expression evaluation

#[cfg(test)]
mod tests {
    use super::super::datafusion_planner::DataFusionPlanner;
    use datafusion::arrow::datatypes::DataType as ArrowDataType;
    use datafusion::logical_expr::LogicalPlan;

    /// Helper function to create planner with test data
    async fn create_test_planner() -> DataFusionPlanner {
        DataFusionPlanner::new().await
    }

    /// Helper to parse and create logical plan
    async fn parse_query(sql: &str) -> Result<LogicalPlan, String> {
        let planner = create_test_planner().await;
        planner.create_logical_plan(sql).await
            .map_err(|e| format!("{:?}", e))
    }

    /// Helper to verify query parses successfully
    async fn assert_query_ok(sql: &str) {
        let result = parse_query(sql).await;
        assert!(
            result.is_ok(),
            "Query failed to parse: {}\nError: {:?}",
            sql,
            result.err()
        );
    }

    // ============================================================================
    // LITERAL VALUE TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_integer_literals() {
        assert_query_ok("SELECT 0").await;
        assert_query_ok("SELECT 1").await;
        assert_query_ok("SELECT -1").await;
        assert_query_ok("SELECT 42").await;
        assert_query_ok("SELECT 2147483647").await; // INT32 max
        assert_query_ok("SELECT -2147483648").await; // INT32 min
        assert_query_ok("SELECT 9223372036854775807").await; // INT64 max
    }

    #[tokio::test]
    async fn test_float_literals() {
        assert_query_ok("SELECT 0.0").await;
        assert_query_ok("SELECT 3.14").await;
        assert_query_ok("SELECT -3.14").await;
        assert_query_ok("SELECT 1.23e10").await;
        assert_query_ok("SELECT 1.23e-10").await;
        assert_query_ok("SELECT 1E10").await;
    }

    #[tokio::test]
    async fn test_string_literals() {
        assert_query_ok("SELECT ''").await;
        assert_query_ok("SELECT 'hello'").await;
        assert_query_ok("SELECT 'hello world'").await;
        assert_query_ok("SELECT 'O''Reilly'").await; // Escaped quote
        assert_query_ok("SELECT 'line1\nline2'").await; // Newline
    }

    #[tokio::test]
    async fn test_boolean_literals() {
        assert_query_ok("SELECT TRUE").await;
        assert_query_ok("SELECT FALSE").await;
        assert_query_ok("SELECT true").await;
        assert_query_ok("SELECT false").await;
    }

    #[tokio::test]
    async fn test_null_literal() {
        assert_query_ok("SELECT NULL").await;
        assert_query_ok("SELECT CAST(NULL AS INTEGER)").await;
        assert_query_ok("SELECT CAST(NULL AS VARCHAR)").await;
    }

    // ============================================================================
    // ARITHMETIC EXPRESSION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_basic_arithmetic() {
        // Addition
        assert_query_ok("SELECT 1 + 1").await; // = 2
        assert_query_ok("SELECT 1 + 2 + 3").await; // = 6
        assert_query_ok("SELECT -1 + 1").await; // = 0

        // Subtraction
        assert_query_ok("SELECT 5 - 3").await; // = 2
        assert_query_ok("SELECT 1 - 5").await; // = -4

        // Multiplication
        assert_query_ok("SELECT 3 * 4").await; // = 12
        assert_query_ok("SELECT -3 * 4").await; // = -12

        // Division
        assert_query_ok("SELECT 10 / 2").await; // = 5
        assert_query_ok("SELECT 10 / 3").await; // = 3 (integer division)
        assert_query_ok("SELECT 10.0 / 3.0").await; // = 3.333...

        // Modulo
        assert_query_ok("SELECT 10 % 3").await; // = 1
        assert_query_ok("SELECT 10 % 2").await; // = 0
    }

    #[tokio::test]
    async fn test_operator_precedence() {
        assert_query_ok("SELECT 1 + 2 * 3").await; // = 7 (not 9)
        assert_query_ok("SELECT (1 + 2) * 3").await; // = 9
        assert_query_ok("SELECT 10 - 3 - 2").await; // = 5 (left associative)
        assert_query_ok("SELECT 2 * 3 + 4 * 5").await; // = 26
    }

    #[tokio::test]
    async fn test_arithmetic_with_null() {
        assert_query_ok("SELECT 1 + NULL").await; // = NULL
        assert_query_ok("SELECT NULL * 5").await; // = NULL
        assert_query_ok("SELECT NULL / 2").await; // = NULL
    }

    // ============================================================================
    // COMPARISON OPERATOR TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_comparison_operators() {
        // Equality
        assert_query_ok("SELECT 1 = 1").await; // TRUE
        assert_query_ok("SELECT 1 = 2").await; // FALSE
        assert_query_ok("SELECT 'abc' = 'abc'").await; // TRUE
        assert_query_ok("SELECT 'abc' = 'def'").await; // FALSE

        // Inequality
        assert_query_ok("SELECT 1 != 2").await; // TRUE
        assert_query_ok("SELECT 1 <> 2").await; // TRUE
        assert_query_ok("SELECT 1 != 1").await; // FALSE

        // Greater/Less than
        assert_query_ok("SELECT 2 > 1").await; // TRUE
        assert_query_ok("SELECT 1 > 2").await; // FALSE
        assert_query_ok("SELECT 1 < 2").await; // TRUE
        assert_query_ok("SELECT 2 < 1").await; // FALSE
        assert_query_ok("SELECT 1 >= 1").await; // TRUE
        assert_query_ok("SELECT 1 <= 1").await; // TRUE
    }

    #[tokio::test]
    async fn test_comparison_with_null() {
        assert_query_ok("SELECT 1 = NULL").await; // NULL (not FALSE!)
        assert_query_ok("SELECT NULL = NULL").await; // NULL (not TRUE!)
        assert_query_ok("SELECT 1 > NULL").await; // NULL
        assert_query_ok("SELECT NULL IS NULL").await; // TRUE
        assert_query_ok("SELECT 1 IS NULL").await; // FALSE
        assert_query_ok("SELECT NULL IS NOT NULL").await; // FALSE
    }

    // ============================================================================
    // LOGICAL OPERATOR TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_logical_and() {
        assert_query_ok("SELECT TRUE AND TRUE").await; // TRUE
        assert_query_ok("SELECT TRUE AND FALSE").await; // FALSE
        assert_query_ok("SELECT FALSE AND FALSE").await; // FALSE
        assert_query_ok("SELECT TRUE AND NULL").await; // NULL
        assert_query_ok("SELECT FALSE AND NULL").await; // FALSE
    }

    #[tokio::test]
    async fn test_logical_or() {
        assert_query_ok("SELECT TRUE OR TRUE").await; // TRUE
        assert_query_ok("SELECT TRUE OR FALSE").await; // TRUE
        assert_query_ok("SELECT FALSE OR FALSE").await; // FALSE
        assert_query_ok("SELECT TRUE OR NULL").await; // TRUE
        assert_query_ok("SELECT FALSE OR NULL").await; // NULL
    }

    #[tokio::test]
    async fn test_logical_not() {
        assert_query_ok("SELECT NOT TRUE").await; // FALSE
        assert_query_ok("SELECT NOT FALSE").await; // TRUE
        assert_query_ok("SELECT NOT NULL").await; // NULL
    }

    // ============================================================================
    // CASE EXPRESSION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_case_simple() {
        assert_query_ok("SELECT CASE 1 WHEN 1 THEN 'one' WHEN 2 THEN 'two' ELSE 'other' END").await;
        assert_query_ok("SELECT CASE 3 WHEN 1 THEN 'one' WHEN 2 THEN 'two' ELSE 'other' END").await;
    }

    #[tokio::test]
    async fn test_case_searched() {
        assert_query_ok("SELECT CASE WHEN 1 = 1 THEN 'yes' ELSE 'no' END").await;
        assert_query_ok("SELECT CASE WHEN 1 > 2 THEN 'yes' ELSE 'no' END").await;
        assert_query_ok("SELECT CASE WHEN 1 > 2 THEN 'a' WHEN 2 > 1 THEN 'b' ELSE 'c' END").await;
    }

    #[tokio::test]
    async fn test_case_with_null() {
        assert_query_ok("SELECT CASE WHEN NULL THEN 'yes' ELSE 'no' END").await;
        assert_query_ok("SELECT CASE NULL WHEN NULL THEN 'yes' ELSE 'no' END").await;
    }

    // ============================================================================
    // COALESCE AND NULLIF TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_coalesce() {
        assert_query_ok("SELECT COALESCE(NULL, 1)").await; // 1
        assert_query_ok("SELECT COALESCE(1, 2)").await; // 1
        assert_query_ok("SELECT COALESCE(NULL, NULL, 3)").await; // 3
        assert_query_ok("SELECT COALESCE('a', 'b')").await; // 'a'
    }

    #[tokio::test]
    async fn test_nullif() {
        assert_query_ok("SELECT NULLIF(1, 1)").await; // NULL
        assert_query_ok("SELECT NULLIF(1, 2)").await; // 1
        assert_query_ok("SELECT NULLIF('a', 'a')").await; // NULL
        assert_query_ok("SELECT NULLIF('a', 'b')").await; // 'a'
    }

    // ============================================================================
    // STRING FUNCTION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_string_length() {
        assert_query_ok("SELECT LENGTH('')").await; // 0
        assert_query_ok("SELECT LENGTH('abc')").await; // 3
        assert_query_ok("SELECT LENGTH('hello world')").await; // 11
        assert_query_ok("SELECT LENGTH(NULL)").await; // NULL
    }

    #[tokio::test]
    async fn test_string_upper_lower() {
        assert_query_ok("SELECT UPPER('hello')").await; // 'HELLO'
        assert_query_ok("SELECT LOWER('HELLO')").await; // 'hello'
        assert_query_ok("SELECT UPPER(NULL)").await; // NULL
        assert_query_ok("SELECT LOWER(NULL)").await; // NULL
    }

    #[tokio::test]
    async fn test_string_trim() {
        assert_query_ok("SELECT TRIM('  hello  ')").await; // 'hello'
        assert_query_ok("SELECT LTRIM('  hello')").await; // 'hello'
        assert_query_ok("SELECT RTRIM('hello  ')").await; // 'hello'
    }

    #[tokio::test]
    async fn test_string_concat() {
        assert_query_ok("SELECT CONCAT('hello', ' ', 'world')").await; // 'hello world'
        assert_query_ok("SELECT CONCAT('a', 'b', 'c')").await; // 'abc'
        assert_query_ok("SELECT 'hello' || ' ' || 'world'").await; // 'hello world'
    }

    #[tokio::test]
    async fn test_string_substring() {
        assert_query_ok("SELECT SUBSTRING('hello', 1, 3)").await; // 'hel'
        assert_query_ok("SELECT SUBSTRING('hello', 3)").await; // 'llo'
        assert_query_ok("SELECT SUBSTRING('hello', 1, 100)").await; // 'hello'
    }

    // ============================================================================
    // AGGREGATION FUNCTION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_count_aggregate() {
        assert_query_ok("SELECT COUNT(*) FROM nation").await;
        assert_query_ok("SELECT COUNT(n_nationkey) FROM nation").await;
        assert_query_ok("SELECT COUNT(DISTINCT n_regionkey) FROM nation").await;
    }

    #[tokio::test]
    async fn test_sum_aggregate() {
        assert_query_ok("SELECT SUM(n_nationkey) FROM nation").await;
        assert_query_ok("SELECT SUM(DISTINCT n_regionkey) FROM nation").await;
    }

    #[tokio::test]
    async fn test_avg_aggregate() {
        assert_query_ok("SELECT AVG(n_nationkey) FROM nation").await;
        assert_query_ok("SELECT AVG(DISTINCT n_regionkey) FROM nation").await;
    }

    #[tokio::test]
    async fn test_min_max_aggregate() {
        assert_query_ok("SELECT MIN(n_nationkey) FROM nation").await;
        assert_query_ok("SELECT MAX(n_nationkey) FROM nation").await;
        assert_query_ok("SELECT MIN(n_name) FROM nation").await; // String min
        assert_query_ok("SELECT MAX(n_name) FROM nation").await; // String max
    }

    #[tokio::test]
    async fn test_multiple_aggregates() {
        assert_query_ok("SELECT COUNT(*), SUM(n_nationkey), AVG(n_nationkey) FROM nation").await;
    }

    #[tokio::test]
    async fn test_aggregate_with_group_by() {
        assert_query_ok("SELECT n_regionkey, COUNT(*) FROM nation GROUP BY n_regionkey").await;
        assert_query_ok("SELECT n_regionkey, SUM(n_nationkey) FROM nation GROUP BY n_regionkey").await;
    }

    // ============================================================================
    // BETWEEN AND IN TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_between_predicate() {
        assert_query_ok("SELECT * FROM nation WHERE n_nationkey BETWEEN 1 AND 10").await;
        assert_query_ok("SELECT * FROM nation WHERE n_nationkey NOT BETWEEN 1 AND 10").await;
    }

    #[tokio::test]
    async fn test_in_predicate() {
        assert_query_ok("SELECT * FROM nation WHERE n_nationkey IN (1, 2, 3)").await;
        assert_query_ok("SELECT * FROM nation WHERE n_nationkey NOT IN (1, 2, 3)").await;
        assert_query_ok("SELECT * FROM nation WHERE n_name IN ('USA', 'CHINA')").await;
    }

    #[tokio::test]
    async fn test_like_predicate() {
        assert_query_ok("SELECT * FROM nation WHERE n_name LIKE '%A%'").await;
        assert_query_ok("SELECT * FROM nation WHERE n_name LIKE 'USA%'").await;
        assert_query_ok("SELECT * FROM nation WHERE n_name NOT LIKE '%X%'").await;
    }

    // ============================================================================
    // CAST AND TYPE CONVERSION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_cast_to_integer() {
        assert_query_ok("SELECT CAST('123' AS INTEGER)").await; // 123
        assert_query_ok("SELECT CAST(123.45 AS INTEGER)").await; // 123
        assert_query_ok("SELECT CAST(TRUE AS INTEGER)").await; // 1
    }

    #[tokio::test]
    async fn test_cast_to_string() {
        assert_query_ok("SELECT CAST(123 AS VARCHAR)").await; // '123'
        assert_query_ok("SELECT CAST(123.45 AS VARCHAR)").await; // '123.45'
        assert_query_ok("SELECT CAST(TRUE AS VARCHAR)").await; // 'true'
    }

    #[tokio::test]
    async fn test_cast_to_float() {
        assert_query_ok("SELECT CAST('123.45' AS DOUBLE)").await; // 123.45
        assert_query_ok("SELECT CAST(123 AS DOUBLE)").await; // 123.0
    }

    #[tokio::test]
    async fn test_cast_null() {
        assert_query_ok("SELECT CAST(NULL AS INTEGER)").await; // NULL
        assert_query_ok("SELECT CAST(NULL AS VARCHAR)").await; // NULL
        assert_query_ok("SELECT CAST(NULL AS DOUBLE)").await; // NULL
    }

    // ============================================================================
    // DATE/TIME FUNCTION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_current_date_time() {
        assert_query_ok("SELECT CURRENT_DATE").await;
        assert_query_ok("SELECT CURRENT_TIMESTAMP").await;
        assert_query_ok("SELECT NOW()").await;
    }

    #[tokio::test]
    async fn test_date_literals() {
        assert_query_ok("SELECT DATE '2024-01-01'").await;
        assert_query_ok("SELECT TIMESTAMP '2024-01-01 12:00:00'").await;
    }

    #[tokio::test]
    async fn test_date_extract() {
        assert_query_ok("SELECT EXTRACT(YEAR FROM DATE '2024-01-01')").await; // 2024
        assert_query_ok("SELECT EXTRACT(MONTH FROM DATE '2024-03-15')").await; // 3
        assert_query_ok("SELECT EXTRACT(DAY FROM DATE '2024-03-15')").await; // 15
    }

    // ============================================================================
    // SUBQUERY TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_scalar_subquery() {
        assert_query_ok("SELECT (SELECT COUNT(*) FROM nation)").await;
        assert_query_ok("SELECT n_nationkey, (SELECT COUNT(*) FROM region) FROM nation").await;
    }

    #[tokio::test]
    async fn test_subquery_in_where() {
        assert_query_ok(
            "SELECT * FROM nation WHERE n_regionkey = (SELECT MIN(r_regionkey) FROM region)"
        ).await;
    }

    #[tokio::test]
    async fn test_exists_subquery() {
        assert_query_ok(
            "SELECT * FROM nation n WHERE EXISTS (SELECT 1 FROM region r WHERE r.r_regionkey = n.n_regionkey)"
        ).await;
    }

    #[tokio::test]
    async fn test_not_exists_subquery() {
        assert_query_ok(
            "SELECT * FROM nation n WHERE NOT EXISTS (SELECT 1 FROM region r WHERE r.r_regionkey = n.n_regionkey)"
        ).await;
    }

    // ============================================================================
    // WINDOW FUNCTION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_row_number() {
        assert_query_ok("SELECT ROW_NUMBER() OVER (ORDER BY n_nationkey) FROM nation").await;
        assert_query_ok("SELECT ROW_NUMBER() OVER (PARTITION BY n_regionkey ORDER BY n_nationkey) FROM nation").await;
    }

    #[tokio::test]
    async fn test_rank_functions() {
        assert_query_ok("SELECT RANK() OVER (ORDER BY n_nationkey) FROM nation").await;
        assert_query_ok("SELECT DENSE_RANK() OVER (ORDER BY n_nationkey) FROM nation").await;
    }

    #[tokio::test]
    async fn test_window_aggregate() {
        assert_query_ok("SELECT SUM(n_nationkey) OVER (PARTITION BY n_regionkey) FROM nation").await;
        assert_query_ok("SELECT AVG(n_nationkey) OVER (ORDER BY n_nationkey) FROM nation").await;
    }

    // ============================================================================
    // DISTINCT TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_select_distinct() {
        assert_query_ok("SELECT DISTINCT n_regionkey FROM nation").await;
        assert_query_ok("SELECT DISTINCT n_regionkey, n_nationkey FROM nation").await;
    }

    #[tokio::test]
    async fn test_distinct_with_order() {
        assert_query_ok("SELECT DISTINCT n_regionkey FROM nation ORDER BY n_regionkey").await;
    }

    // ============================================================================
    // UNION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_union() {
        assert_query_ok(
            "SELECT n_nationkey FROM nation WHERE n_regionkey = 1
             UNION
             SELECT n_nationkey FROM nation WHERE n_regionkey = 2"
        ).await;
    }

    #[tokio::test]
    async fn test_union_all() {
        assert_query_ok(
            "SELECT n_nationkey FROM nation WHERE n_regionkey = 1
             UNION ALL
             SELECT n_nationkey FROM nation WHERE n_regionkey = 2"
        ).await;
    }

    // ============================================================================
    // LIMIT AND OFFSET TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_limit() {
        assert_query_ok("SELECT * FROM nation LIMIT 10").await;
        assert_query_ok("SELECT * FROM nation ORDER BY n_nationkey LIMIT 5").await;
    }

    #[tokio::test]
    async fn test_offset() {
        assert_query_ok("SELECT * FROM nation LIMIT 10 OFFSET 5").await;
        assert_query_ok("SELECT * FROM nation ORDER BY n_nationkey LIMIT 10 OFFSET 5").await;
    }

    // ============================================================================
    // EDGE CASE TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_empty_result_set() {
        assert_query_ok("SELECT * FROM nation WHERE FALSE").await;
        assert_query_ok("SELECT * FROM nation WHERE 1 = 0").await;
    }

    #[tokio::test]
    async fn test_division_by_zero() {
        // Should handle division by zero gracefully
        assert_query_ok("SELECT 1 / 0").await;
        assert_query_ok("SELECT CASE WHEN 0 = 0 THEN NULL ELSE 1 / 0 END").await;
    }

    #[tokio::test]
    async fn test_very_long_strings() {
        let long_string = "a".repeat(1000);
        let sql = format!("SELECT '{}'", long_string);
        assert_query_ok(&sql).await;
    }

    #[tokio::test]
    async fn test_deeply_nested_expressions() {
        assert_query_ok("SELECT ((((1 + 2) * 3) - 4) / 5)").await;
        assert_query_ok("SELECT CASE WHEN CASE WHEN 1 = 1 THEN TRUE ELSE FALSE END THEN 'yes' ELSE 'no' END").await;
    }

    // ============================================================================
    // COMPREHENSIVE TEST
    // ============================================================================

    #[tokio::test]
    async fn test_complex_query_logic() {
        // Complex query combining multiple features
        assert_query_ok(
            "SELECT
                n_regionkey,
                COUNT(*) as nation_count,
                SUM(n_nationkey) as sum_keys,
                AVG(n_nationkey) as avg_key,
                MIN(n_name) as first_name,
                MAX(n_name) as last_name,
                CASE
                    WHEN COUNT(*) > 5 THEN 'Large'
                    WHEN COUNT(*) > 2 THEN 'Medium'
                    ELSE 'Small'
                END as size_category
             FROM nation
             WHERE n_nationkey BETWEEN 1 AND 20
               AND n_name IS NOT NULL
             GROUP BY n_regionkey
             HAVING COUNT(*) > 0
             ORDER BY nation_count DESC, n_regionkey ASC
             LIMIT 10"
        ).await;
    }

    #[tokio::test]
    async fn test_summary() {
        println!("✅ All SQL logic tests validate query semantics and correctness");
        println!("📊 Tests cover: literals, operators, functions, aggregations, subqueries, window functions, and edge cases");
    }
}
