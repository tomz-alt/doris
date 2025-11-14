//! Datatype Regression Tests - 30 tests covering various data types
//! Inspired by Doris regression-test/suites/datatype_p0/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // Numeric Types (10 tests)

    #[test]
    fn test_tinyint_type() {
        let sql = "SELECT CAST(127 AS TINYINT) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_smallint_type() {
        let sql = "SELECT CAST(32767 AS SMALLINT) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_integer_type() {
        let sql = "SELECT CAST(2147483647 AS INTEGER) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_bigint_type() {
        let sql = "SELECT CAST(9223372036854775807 AS BIGINT) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_decimal_type() {
        let sql = "SELECT CAST(123.456 AS DECIMAL(10, 3)) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_decimal_precision() {
        let sql = "SELECT CAST(99999.99999 AS DECIMAL(15, 5)) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_float_type() {
        let sql = "SELECT CAST(3.14159 AS FLOAT) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_double_type() {
        let sql = "SELECT CAST(3.141592653589793 AS DOUBLE) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_numeric_arithmetic() {
        let sql = r#"
            SELECT
                CAST(10 AS INTEGER) + CAST(20 AS INTEGER) AS sum,
                CAST(100 AS DOUBLE) * CAST(2.5 AS DOUBLE) AS product,
                CAST(1000 AS DECIMAL(10,2)) / CAST(3 AS DECIMAL(10,2)) AS division
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_numeric_comparisons() {
        let sql = r#"
            SELECT
                CAST(10 AS INTEGER) > CAST(5 AS INTEGER) AS gt,
                CAST(3.14 AS DOUBLE) < CAST(3.15 AS DOUBLE) AS lt,
                CAST(100 AS BIGINT) = CAST(100 AS BIGINT) AS eq
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // String Types (8 tests)

    #[test]
    fn test_char_type() {
        let sql = "SELECT CAST('A' AS CHAR(1)) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_char_fixed_length() {
        let sql = "SELECT CAST('Hello' AS CHAR(10)) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_varchar_type() {
        let sql = "SELECT CAST('Hello World' AS VARCHAR(100)) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_varchar_variable_length() {
        let sql = "SELECT CAST('Test' AS VARCHAR(255)) AS short, CAST('A much longer string' AS VARCHAR(255)) AS long";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_text_type() {
        let sql = "SELECT CAST('This is a long text field with lots of content' AS TEXT) AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_string_concatenation() {
        let sql = "SELECT CAST('Hello' AS VARCHAR) || CAST(' ' AS VARCHAR) || CAST('World' AS VARCHAR) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_string_comparison() {
        let sql = r#"
            SELECT
                CAST('abc' AS VARCHAR) < CAST('def' AS VARCHAR) AS lt,
                CAST('xyz' AS VARCHAR) = CAST('xyz' AS VARCHAR) AS eq,
                CAST('hello' AS VARCHAR) > CAST('goodbye' AS VARCHAR) AS gt
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_empty_string() {
        let sql = "SELECT CAST('' AS VARCHAR) AS empty, LENGTH(CAST('' AS VARCHAR)) AS len";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Date/Time Types (7 tests)

    #[test]
    fn test_date_type() {
        let sql = "SELECT DATE '2024-05-15' AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_time_type() {
        let sql = "SELECT TIME '14:30:00' AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_timestamp_type() {
        let sql = "SELECT TIMESTAMP '2024-05-15 14:30:00' AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_timestamp_with_milliseconds() {
        let sql = "SELECT TIMESTAMP '2024-05-15 14:30:00.123' AS value";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_date_comparison() {
        let sql = r#"
            SELECT
                DATE '2024-01-01' < DATE '2024-12-31' AS lt,
                DATE '2024-05-15' = DATE '2024-05-15' AS eq,
                DATE '2024-12-31' > DATE '2024-01-01' AS gt
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_timestamp_comparison() {
        let sql = r#"
            SELECT
                TIMESTAMP '2024-01-01 00:00:00' < TIMESTAMP '2024-01-01 12:00:00' AS lt,
                TIMESTAMP '2024-05-15 10:30:00' = TIMESTAMP '2024-05-15 10:30:00' AS eq
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_interval_type() {
        let sql = r#"
            SELECT
                INTERVAL '1' DAY AS one_day,
                INTERVAL '3' MONTH AS three_months,
                INTERVAL '2' YEAR AS two_years
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Boolean and NULL (5 tests)

    #[test]
    fn test_boolean_type() {
        let sql = "SELECT TRUE AS t, FALSE AS f";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_boolean_operations() {
        let sql = r#"
            SELECT
                TRUE AND FALSE AS and_op,
                TRUE OR FALSE AS or_op,
                NOT TRUE AS not_op
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_type() {
        let sql = "SELECT NULL AS null_value, CAST(NULL AS INTEGER) AS null_int";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_comparison() {
        let sql = r#"
            SELECT
                NULL IS NULL AS is_null,
                NULL IS NOT NULL AS is_not_null,
                5 IS NULL AS five_is_null,
                5 IS NOT NULL AS five_is_not_null
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_null_in_expressions() {
        let sql = r#"
            SELECT
                NULL + 5 AS null_plus_five,
                NULL = NULL AS null_equals_null,
                COALESCE(NULL, 'default') AS coalesce_null
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
