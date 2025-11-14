//! Function Regression Tests - 80 tests covering SQL functions
//! Inspired by Doris regression-test/suites/query_p0/sql_functions/

use super::test_runner::{execute_test, ExpectedResult};

#[cfg(test)]
mod tests {
    use super::*;

    // String Functions (20 tests)

    #[test]
    fn test_concat() {
        let sql = "SELECT CONCAT('Hello', ' ', 'World') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_concat_ws() {
        let sql = "SELECT CONCAT_WS(',', 'a', 'b', 'c') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_upper() {
        let sql = "SELECT UPPER('hello') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_lower() {
        let sql = "SELECT LOWER('HELLO') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_substring() {
        let sql = "SELECT SUBSTRING('Hello World', 1, 5) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_substr() {
        let sql = "SELECT SUBSTR('Hello World', 7) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_left() {
        let sql = "SELECT LEFT('Hello', 3) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_right() {
        let sql = "SELECT RIGHT('Hello', 3) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_trim() {
        let sql = "SELECT TRIM('  Hello  ') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_ltrim() {
        let sql = "SELECT LTRIM('  Hello') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_rtrim() {
        let sql = "SELECT RTRIM('Hello  ') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_length() {
        let sql = "SELECT LENGTH('Hello') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_char_length() {
        let sql = "SELECT CHAR_LENGTH('Hello') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_replace() {
        let sql = "SELECT REPLACE('Hello World', 'World', 'There') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_reverse() {
        let sql = "SELECT REVERSE('Hello') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_repeat() {
        let sql = "SELECT REPEAT('ab', 3) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_lpad() {
        let sql = "SELECT LPAD('hi', 5, '?') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_rpad() {
        let sql = "SELECT RPAD('hi', 5, '?') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_position() {
        let sql = "SELECT POSITION('World' IN 'Hello World') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_split_part() {
        let sql = "SELECT SPLIT_PART('a,b,c', ',', 2) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Math Functions (20 tests)

    #[test]
    fn test_abs() {
        let sql = "SELECT ABS(-42) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_ceil() {
        let sql = "SELECT CEIL(42.3) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_floor() {
        let sql = "SELECT FLOOR(42.7) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_round() {
        let sql = "SELECT ROUND(42.4567, 2) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_truncate() {
        let sql = "SELECT TRUNCATE(42.4567, 2) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_power() {
        let sql = "SELECT POWER(2, 10) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_sqrt() {
        let sql = "SELECT SQRT(16) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_exp() {
        let sql = "SELECT EXP(1) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_ln() {
        let sql = "SELECT LN(10) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_log() {
        let sql = "SELECT LOG(10, 100) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_log10() {
        let sql = "SELECT LOG10(100) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_mod() {
        let sql = "SELECT MOD(10, 3) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_pi() {
        let sql = "SELECT PI() AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_sin() {
        let sql = "SELECT SIN(PI() / 2) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cos() {
        let sql = "SELECT COS(0) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_tan() {
        let sql = "SELECT TAN(PI() / 4) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_asin() {
        let sql = "SELECT ASIN(1) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_acos() {
        let sql = "SELECT ACOS(1) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_atan() {
        let sql = "SELECT ATAN(1) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_sign() {
        let sql = "SELECT SIGN(-42) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Date/Time Functions (20 tests)

    #[test]
    fn test_current_date() {
        let sql = "SELECT CURRENT_DATE AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_current_timestamp() {
        let sql = "SELECT CURRENT_TIMESTAMP AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_now() {
        let sql = "SELECT NOW() AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_extract_year() {
        let sql = "SELECT EXTRACT(YEAR FROM DATE '2024-05-15') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_extract_month() {
        let sql = "SELECT EXTRACT(MONTH FROM DATE '2024-05-15') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_extract_day() {
        let sql = "SELECT EXTRACT(DAY FROM DATE '2024-05-15') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_extract_hour() {
        let sql = "SELECT EXTRACT(HOUR FROM TIMESTAMP '2024-05-15 14:30:00') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_extract_minute() {
        let sql = "SELECT EXTRACT(MINUTE FROM TIMESTAMP '2024-05-15 14:30:00') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_extract_second() {
        let sql = "SELECT EXTRACT(SECOND FROM TIMESTAMP '2024-05-15 14:30:45') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_date_add() {
        let sql = "SELECT DATE '2024-01-01' + INTERVAL '1' DAY AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_date_sub() {
        let sql = "SELECT DATE '2024-01-01' - INTERVAL '1' DAY AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_date_diff() {
        let sql = "SELECT DATE '2024-01-10' - DATE '2024-01-01' AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_interval_year() {
        let sql = "SELECT DATE '2024-01-01' + INTERVAL '1' YEAR AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_interval_month() {
        let sql = "SELECT DATE '2024-01-01' + INTERVAL '3' MONTH AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_interval_hour() {
        let sql = "SELECT TIMESTAMP '2024-01-01 12:00:00' + INTERVAL '2' HOUR AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_date_trunc_year() {
        let sql = "SELECT DATE_TRUNC('year', DATE '2024-05-15') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_date_trunc_month() {
        let sql = "SELECT DATE_TRUNC('month', DATE '2024-05-15') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_date_trunc_day() {
        let sql = "SELECT DATE_TRUNC('day', TIMESTAMP '2024-05-15 14:30:00') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_to_char_date() {
        let sql = "SELECT TO_CHAR(DATE '2024-05-15', 'YYYY-MM-DD') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_age_calculation() {
        let sql = "SELECT DATE '2024-12-31' - DATE '2024-01-01' AS days_diff";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Conditional Functions (10 tests)

    #[test]
    fn test_case_simple() {
        let sql = r#"
            SELECT CASE
                WHEN 1 = 1 THEN 'true'
                ELSE 'false'
            END AS result
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_case_multiple_conditions() {
        let sql = r#"
            SELECT CASE
                WHEN 1 > 2 THEN 'greater'
                WHEN 1 < 2 THEN 'less'
                WHEN 1 = 2 THEN 'equal'
                ELSE 'unknown'
            END AS result
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_case_with_table() {
        let sql = r#"
            SELECT
                id,
                CASE
                    WHEN price < 10 THEN 'Cheap'
                    WHEN price < 100 THEN 'Moderate'
                    ELSE 'Expensive'
                END AS price_category
            FROM products
        "#;
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_coalesce_two_args() {
        let sql = "SELECT COALESCE(NULL, 'default') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_coalesce_multiple_args() {
        let sql = "SELECT COALESCE(NULL, NULL, 'third', 'fourth') AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_nullif_equal() {
        let sql = "SELECT NULLIF(5, 5) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_nullif_not_equal() {
        let sql = "SELECT NULLIF(5, 3) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_greatest() {
        let sql = "SELECT GREATEST(1, 5, 3, 9, 2) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_least() {
        let sql = "SELECT LEAST(1, 5, 3, 9, 2) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_if_expression() {
        let sql = "SELECT CASE WHEN 1 = 1 THEN 'yes' ELSE 'no' END AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    // Type Conversion Functions (10 tests)

    #[test]
    fn test_cast_to_varchar() {
        let sql = "SELECT CAST(123 AS VARCHAR) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cast_to_integer() {
        let sql = "SELECT CAST('123' AS INTEGER) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cast_to_bigint() {
        let sql = "SELECT CAST(123 AS BIGINT) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cast_to_double() {
        let sql = "SELECT CAST('123.456' AS DOUBLE) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cast_to_decimal() {
        let sql = "SELECT CAST(123.456 AS DECIMAL(10, 2)) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cast_to_date() {
        let sql = "SELECT CAST('2024-01-01' AS DATE) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cast_to_timestamp() {
        let sql = "SELECT CAST('2024-01-01 12:00:00' AS TIMESTAMP) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cast_to_boolean() {
        let sql = "SELECT CAST(1 AS BOOLEAN) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_try_cast() {
        let sql = "SELECT TRY_CAST('not_a_number' AS INTEGER) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }

    #[test]
    fn test_cast_null() {
        let sql = "SELECT CAST(NULL AS VARCHAR) AS result";
        assert!(execute_test(sql, ExpectedResult::Success).is_ok());
    }
}
