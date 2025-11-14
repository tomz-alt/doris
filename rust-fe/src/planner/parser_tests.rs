// SQL Parser Tests
// Tests ensure SQL parsing correctness and compatibility with Doris SQL dialect
//
// Based on Java FE tests in:
// - fe/fe-core/src/test/java/org/apache/doris/nereids/parser/NereidsParserTest.java
// - fe/fe-core/src/test/java/org/apache/doris/nereids/trees/expressions/ExpressionParserTest.java

#[cfg(test)]
mod tests {
    use super::super::datafusion_planner::DataFusionPlanner;
    use datafusion::logical_expr::{LogicalPlan, Expr};

    /// Helper function to parse SQL and return logical plan
    async fn parse_sql(sql: &str) -> Result<LogicalPlan, String> {
        let planner = DataFusionPlanner::new().await;
        planner.create_logical_plan(sql).await
            .map_err(|e| format!("{:?}", e))
    }

    /// Helper function to parse SQL and verify it doesn't error
    async fn assert_parses(sql: &str) {
        let result = parse_sql(sql).await;
        assert!(result.is_ok(), "Failed to parse: {}\nError: {:?}", sql, result.err());
    }

    /// Helper function to verify SQL parsing fails
    async fn assert_parse_error(sql: &str) {
        let result = parse_sql(sql).await;
        assert!(result.is_err(), "Expected parse error for: {}", sql);
    }

    // ============================================================================
    // BASIC QUERY TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_simple_select() {
        assert_parses("SELECT 1").await;
        assert_parses("SELECT 1, 2, 3").await;
        assert_parses("SELECT 'hello'").await;
        assert_parses("SELECT 1 + 2").await;
    }

    #[tokio::test]
    async fn test_select_with_alias() {
        assert_parses("SELECT 1 AS one").await;
        assert_parses("SELECT 1 AS one, 2 AS two").await;
        assert_parses("SELECT n_nationkey AS key FROM nation").await;
    }

    #[tokio::test]
    async fn test_select_all() {
        assert_parses("SELECT * FROM nation").await;
        assert_parses("SELECT nation.* FROM nation").await;
    }

    #[tokio::test]
    async fn test_select_with_table() {
        assert_parses("SELECT n_nationkey FROM nation").await;
        assert_parses("SELECT n_nationkey, n_name FROM nation").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey = 1").await;
    }

    // ============================================================================
    // WHERE CLAUSE TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_where_comparison() {
        assert_parses("SELECT * FROM nation WHERE n_nationkey = 1").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey > 5").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey < 10").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey >= 5").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey <= 10").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey != 5").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey <> 5").await;
    }

    #[tokio::test]
    async fn test_where_between() {
        // Based on ExpressionParserTest.java testSqlBetweenPredicate
        assert_parses("SELECT * FROM nation WHERE n_nationkey BETWEEN 1 AND 10").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey NOT BETWEEN 1 AND 10").await;
    }

    #[tokio::test]
    async fn test_where_in() {
        // Based on ExpressionParserTest.java testInPredicate
        assert_parses("SELECT * FROM nation WHERE n_nationkey IN (1, 2, 3)").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey NOT IN (1, 2, 3)").await;
        assert_parses("SELECT * FROM nation WHERE n_name IN ('USA', 'CHINA')").await;
    }

    #[tokio::test]
    async fn test_where_like() {
        assert_parses("SELECT * FROM nation WHERE n_name LIKE 'USA%'").await;
        assert_parses("SELECT * FROM nation WHERE n_name NOT LIKE '%CHINA%'").await;
    }

    #[tokio::test]
    async fn test_where_null() {
        assert_parses("SELECT * FROM nation WHERE n_comment IS NULL").await;
        assert_parses("SELECT * FROM nation WHERE n_comment IS NOT NULL").await;
    }

    // ============================================================================
    // LOGICAL OPERATOR TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_where_and() {
        // Based on ExpressionParserTest.java testSqlAnd
        assert_parses("SELECT * FROM nation WHERE n_nationkey > 1 AND n_regionkey > 1").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey > 1 AND n_regionkey > 1 AND n_nationkey < 10").await;
    }

    #[tokio::test]
    async fn test_where_or() {
        // Based on ExpressionParserTest.java testExprOr
        assert_parses("SELECT * FROM nation WHERE n_nationkey = 1 OR n_nationkey = 2").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey = 1 OR n_nationkey = 2 OR n_nationkey = 3").await;
    }

    #[tokio::test]
    async fn test_where_not() {
        assert_parses("SELECT * FROM nation WHERE NOT n_nationkey = 1").await;
        assert_parses("SELECT * FROM nation WHERE NOT (n_nationkey > 1 AND n_regionkey > 1)").await;
    }

    #[tokio::test]
    async fn test_where_complex_logical() {
        assert_parses("SELECT * FROM nation WHERE (n_nationkey > 1 OR n_regionkey > 1) AND n_nationkey < 10").await;
        assert_parses("SELECT * FROM nation WHERE n_nationkey > 1 AND (n_regionkey = 1 OR n_regionkey = 2)").await;
    }

    // ============================================================================
    // ARITHMETIC OPERATOR TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_arithmetic_operators() {
        // Based on ExpressionParserTest.java testExprArithmetic
        assert_parses("SELECT 1 + 2").await;
        assert_parses("SELECT 5 - 3").await;
        assert_parses("SELECT 3 * 4").await;
        assert_parses("SELECT 10 / 2").await;
        assert_parses("SELECT 10 % 3").await;
    }

    #[tokio::test]
    async fn test_arithmetic_precedence() {
        assert_parses("SELECT 1 + 2 * 3").await;
        assert_parses("SELECT (1 + 2) * 3").await;
        assert_parses("SELECT 10 / 2 + 3").await;
        assert_parses("SELECT 10 % 3 * 2").await;
    }

    #[tokio::test]
    async fn test_arithmetic_with_columns() {
        assert_parses("SELECT n_nationkey + 1 FROM nation").await;
        assert_parses("SELECT n_nationkey * 2 FROM nation").await;
        assert_parses("SELECT n_nationkey + n_regionkey FROM nation").await;
    }

    // ============================================================================
    // JOIN TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_inner_join() {
        assert_parses("SELECT * FROM nation JOIN region ON n_regionkey = r_regionkey").await;
        assert_parses("SELECT * FROM nation INNER JOIN region ON n_regionkey = r_regionkey").await;
    }

    #[tokio::test]
    async fn test_left_join() {
        assert_parses("SELECT * FROM nation LEFT JOIN region ON n_regionkey = r_regionkey").await;
        assert_parses("SELECT * FROM nation LEFT OUTER JOIN region ON n_regionkey = r_regionkey").await;
    }

    #[tokio::test]
    async fn test_right_join() {
        assert_parses("SELECT * FROM nation RIGHT JOIN region ON n_regionkey = r_regionkey").await;
        assert_parses("SELECT * FROM nation RIGHT OUTER JOIN region ON n_regionkey = r_regionkey").await;
    }

    #[tokio::test]
    async fn test_full_join() {
        assert_parses("SELECT * FROM nation FULL JOIN region ON n_regionkey = r_regionkey").await;
        assert_parses("SELECT * FROM nation FULL OUTER JOIN region ON n_regionkey = r_regionkey").await;
    }

    #[tokio::test]
    async fn test_cross_join() {
        assert_parses("SELECT * FROM nation CROSS JOIN region").await;
    }

    #[tokio::test]
    async fn test_multiple_joins() {
        assert_parses(
            "SELECT * FROM nation
             JOIN region ON n_regionkey = r_regionkey
             JOIN supplier ON n_nationkey = s_nationkey"
        ).await;
    }

    // ============================================================================
    // AGGREGATION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_aggregate_functions() {
        assert_parses("SELECT COUNT(*) FROM nation").await;
        assert_parses("SELECT COUNT(n_nationkey) FROM nation").await;
        assert_parses("SELECT COUNT(DISTINCT n_regionkey) FROM nation").await;
        assert_parses("SELECT SUM(n_nationkey) FROM nation").await;
        assert_parses("SELECT AVG(n_nationkey) FROM nation").await;
        assert_parses("SELECT MIN(n_nationkey) FROM nation").await;
        assert_parses("SELECT MAX(n_nationkey) FROM nation").await;
    }

    #[tokio::test]
    async fn test_group_by() {
        assert_parses("SELECT n_regionkey, COUNT(*) FROM nation GROUP BY n_regionkey").await;
        assert_parses("SELECT n_regionkey, n_name, COUNT(*) FROM nation GROUP BY n_regionkey, n_name").await;
    }

    #[tokio::test]
    async fn test_having() {
        assert_parses("SELECT n_regionkey, COUNT(*) FROM nation GROUP BY n_regionkey HAVING COUNT(*) > 5").await;
        assert_parses("SELECT n_regionkey, SUM(n_nationkey) FROM nation GROUP BY n_regionkey HAVING SUM(n_nationkey) > 10").await;
    }

    // ============================================================================
    // ORDER BY AND LIMIT TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_order_by() {
        assert_parses("SELECT * FROM nation ORDER BY n_nationkey").await;
        assert_parses("SELECT * FROM nation ORDER BY n_nationkey ASC").await;
        assert_parses("SELECT * FROM nation ORDER BY n_nationkey DESC").await;
        assert_parses("SELECT * FROM nation ORDER BY n_regionkey, n_nationkey").await;
        assert_parses("SELECT * FROM nation ORDER BY n_regionkey DESC, n_nationkey ASC").await;
    }

    #[tokio::test]
    async fn test_limit() {
        assert_parses("SELECT * FROM nation LIMIT 10").await;
        assert_parses("SELECT * FROM nation ORDER BY n_nationkey LIMIT 5").await;
    }

    #[tokio::test]
    async fn test_offset() {
        assert_parses("SELECT * FROM nation LIMIT 10 OFFSET 5").await;
        assert_parses("SELECT * FROM nation ORDER BY n_nationkey LIMIT 10 OFFSET 5").await;
    }

    // ============================================================================
    // SUBQUERY TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_subquery_in_where() {
        assert_parses(
            "SELECT * FROM nation WHERE n_regionkey IN (SELECT r_regionkey FROM region)"
        ).await;
    }

    #[tokio::test]
    async fn test_subquery_in_from() {
        assert_parses(
            "SELECT * FROM (SELECT n_nationkey, n_name FROM nation) AS sub"
        ).await;
    }

    #[tokio::test]
    async fn test_subquery_in_select() {
        assert_parses(
            "SELECT n_nationkey, (SELECT COUNT(*) FROM region) AS region_count FROM nation"
        ).await;
    }

    // ============================================================================
    // CTE (WITH CLAUSE) TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_cte_basic() {
        // Based on NereidsParserTest.java testParseCTE
        assert_parses(
            "WITH t1 AS (SELECT n_nationkey FROM nation WHERE n_nationkey < 10)
             SELECT * FROM t1"
        ).await;
    }

    #[tokio::test]
    async fn test_cte_multiple() {
        // Based on NereidsParserTest.java testParseCTE (cteSql2)
        assert_parses(
            "WITH t1 AS (SELECT n_nationkey FROM nation),
                  t2 AS (SELECT n_nationkey FROM t1)
             SELECT * FROM t2"
        ).await;
    }

    #[tokio::test]
    async fn test_cte_with_column_aliases() {
        // Based on NereidsParserTest.java testParseCTE (cteSql3)
        assert_parses(
            "WITH t1 (key, name) AS (SELECT n_nationkey, n_name FROM nation)
             SELECT * FROM t1"
        ).await;
    }

    // ============================================================================
    // WINDOW FUNCTION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_window_rank() {
        // Based on NereidsParserTest.java testParseWindowFunctions
        assert_parses(
            "SELECT n_nationkey, RANK() OVER (PARTITION BY n_regionkey ORDER BY n_nationkey) AS ranking FROM nation"
        ).await;
    }

    #[tokio::test]
    async fn test_window_row_number() {
        assert_parses(
            "SELECT n_nationkey, ROW_NUMBER() OVER (ORDER BY n_nationkey) AS row_num FROM nation"
        ).await;
    }

    #[tokio::test]
    async fn test_window_dense_rank() {
        assert_parses(
            "SELECT n_nationkey, DENSE_RANK() OVER (PARTITION BY n_regionkey ORDER BY n_nationkey) AS dense_ranking FROM nation"
        ).await;
    }

    #[tokio::test]
    async fn test_window_with_aggregation() {
        // Based on NereidsParserTest.java testParseWindowFunctions (windowSql2)
        assert_parses(
            "SELECT n_regionkey, SUM(n_nationkey), RANK() OVER (PARTITION BY n_regionkey ORDER BY n_nationkey) AS ranking
             FROM nation GROUP BY n_regionkey, n_nationkey"
        ).await;
    }

    // ============================================================================
    // CASE EXPRESSION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_case_simple() {
        assert_parses(
            "SELECT CASE n_regionkey WHEN 1 THEN 'Region 1' WHEN 2 THEN 'Region 2' ELSE 'Other' END FROM nation"
        ).await;
    }

    #[tokio::test]
    async fn test_case_searched() {
        assert_parses(
            "SELECT CASE
                WHEN n_regionkey = 1 THEN 'Region 1'
                WHEN n_regionkey = 2 THEN 'Region 2'
                ELSE 'Other'
             END FROM nation"
        ).await;
    }

    // ============================================================================
    // CAST AND TYPE CONVERSION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_cast() {
        assert_parses("SELECT CAST(n_nationkey AS BIGINT) FROM nation").await;
        assert_parses("SELECT CAST(n_name AS VARCHAR) FROM nation").await;
        assert_parses("SELECT CAST('123' AS INTEGER)").await;
    }

    // ============================================================================
    // STRING FUNCTION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_string_functions() {
        assert_parses("SELECT UPPER(n_name) FROM nation").await;
        assert_parses("SELECT LOWER(n_name) FROM nation").await;
        assert_parses("SELECT LENGTH(n_name) FROM nation").await;
        assert_parses("SELECT CONCAT(n_name, '_suffix') FROM nation").await;
        assert_parses("SELECT SUBSTRING(n_name, 1, 3) FROM nation").await;
    }

    // ============================================================================
    // UNION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_union() {
        assert_parses(
            "SELECT n_nationkey FROM nation WHERE n_regionkey = 1
             UNION
             SELECT n_nationkey FROM nation WHERE n_regionkey = 2"
        ).await;
    }

    #[tokio::test]
    async fn test_union_all() {
        assert_parses(
            "SELECT n_nationkey FROM nation WHERE n_regionkey = 1
             UNION ALL
             SELECT n_nationkey FROM nation WHERE n_regionkey = 2"
        ).await;
    }

    // ============================================================================
    // DISTINCT TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_select_distinct() {
        assert_parses("SELECT DISTINCT n_regionkey FROM nation").await;
        assert_parses("SELECT DISTINCT n_regionkey, n_nationkey FROM nation").await;
    }

    // ============================================================================
    // DATE/TIME FUNCTION TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_date_functions() {
        assert_parses("SELECT CURRENT_DATE").await;
        assert_parses("SELECT CURRENT_TIMESTAMP").await;
        assert_parses("SELECT DATE '2024-01-01'").await;
        assert_parses("SELECT TIMESTAMP '2024-01-01 12:00:00'").await;
    }

    // ============================================================================
    // NULL HANDLING TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_coalesce() {
        assert_parses("SELECT COALESCE(n_comment, 'No comment') FROM nation").await;
        assert_parses("SELECT COALESCE(n_comment, n_name, 'Unknown') FROM nation").await;
    }

    #[tokio::test]
    async fn test_nullif() {
        assert_parses("SELECT NULLIF(n_nationkey, 0) FROM nation").await;
    }

    // ============================================================================
    // EXPLAIN TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_explain() {
        // Based on NereidsParserTest.java testExplainNormal
        assert_parses("EXPLAIN SELECT * FROM nation WHERE n_nationkey = 1").await;
    }

    // ============================================================================
    // ERROR HANDLING TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_syntax_error_missing_from() {
        assert_parse_error("SELECT n_nationkey WHERE n_regionkey = 1").await;
    }

    #[tokio::test]
    async fn test_syntax_error_invalid_operator() {
        assert_parse_error("SELECT * FROM nation WHERE n_nationkey === 1").await;
    }

    #[tokio::test]
    async fn test_syntax_error_unclosed_parenthesis() {
        assert_parse_error("SELECT * FROM nation WHERE (n_nationkey = 1").await;
    }

    #[tokio::test]
    async fn test_syntax_error_invalid_keyword() {
        // Based on NereidsParserTest.java testErrorListener
        assert_parse_error("SELECT * FROM nation WHERE n_nationkey = 1 illegal_symbol").await;
    }

    // ============================================================================
    // COMPLEX QUERY TESTS (TPC-H style)
    // ============================================================================

    #[tokio::test]
    async fn test_tpch_q1_style() {
        // Simplified TPC-H Q1 style query
        assert_parses(
            "SELECT n_regionkey,
                    COUNT(*) AS count,
                    SUM(n_nationkey) AS sum_key
             FROM nation
             WHERE n_nationkey > 5
             GROUP BY n_regionkey
             ORDER BY n_regionkey"
        ).await;
    }

    #[tokio::test]
    async fn test_complex_join_with_aggregation() {
        assert_parses(
            "SELECT r.r_name,
                    COUNT(n.n_nationkey) AS nation_count
             FROM region r
             JOIN nation n ON r.r_regionkey = n.n_regionkey
             WHERE r.r_regionkey < 3
             GROUP BY r.r_name
             HAVING COUNT(n.n_nationkey) > 1
             ORDER BY nation_count DESC"
        ).await;
    }

    // ============================================================================
    // SPECIAL CHARACTER TESTS
    // ============================================================================

    #[tokio::test]
    async fn test_escape_sequences() {
        // Based on ExpressionParserTest.java testNoBackslashEscapes
        // Note: DataFusion may handle escapes differently than Doris
        assert_parses("SELECT 'hello''world'").await; // Single quote escape
    }

    #[tokio::test]
    async fn test_quoted_identifiers() {
        // Based on NereidsParserTest.java testPostProcessor
        assert_parses("SELECT \"n_nationkey\" FROM nation").await;
    }
}
