# Regression Test Framework

## Overview

A comprehensive regression test framework inspired by Doris's TeamCity CI with 7,484+ tests.

## Structure

```
src/regression/
├── mod.rs              - Module definition
├── test_runner.rs      - SQL test execution infrastructure  
├── join_tests.rs       - 40 JOIN operation tests
├── aggregate_tests.rs  - 50 aggregation and window function tests
├── subquery_tests.rs   - 45 subquery tests (IN, EXISTS, correlated)
├── function_tests.rs   - 80 function tests (string, math, date, cast)
├── datatype_tests.rs   - 30 data type tests
└── edge_case_tests.rs  - 28 edge case and error handling tests
```

## Test Categories

1. **JOIN Tests** (40 tests)
   - INNER, LEFT, RIGHT, FULL, CROSS joins
   - Self joins, multiple joins
   - JOIN with subqueries, CTEs, window functions

2. **Aggregate Tests** (50 tests)
   - COUNT, SUM, AVG, MIN, MAX
   - GROUP BY with multiple columns
   - HAVING clauses
   - Window functions: ROW_NUMBER, RANK, LAG, LEAD
   - Cumulative and moving averages

3. **Subquery Tests** (45 tests)
   - Scalar subqueries
   - IN/NOT IN subqueries
   - EXISTS/NOT EXISTS
   - Correlated subqueries
   - ANY/ALL/SOME

4. **Function Tests** (80 tests)
   - String: CONCAT, UPPER, LOWER, SUBSTRING, TRIM
   - Math: ABS, ROUND, POWER, SQRT, TRIG functions
   - Date/Time: EXTRACT, DATE_ADD, DATE_DIFF
   - Conditional: CASE, COALESCE, NULLIF
   - Type conversions: CAST

5. **Datatype Tests** (30 tests)
   - Numeric types (INT, BIGINT, DECIMAL, FLOAT, DOUBLE)
   - String types (CHAR, VARCHAR, TEXT)
   - Date/Time types (DATE, TIME, TIMESTAMP)
   - Boolean and NULL handling

6. **Edge Case Tests** (28 tests)
   - Empty result sets
   - NULL handling in various contexts
   - Extreme values
   - Long identifiers
   - Syntax errors

## Total Tests: 273 Regression Tests

Combined with existing test suites:
- MySQL Protocol: 28 tests
- SQL Parser: 57 tests
- TPC-H: 23 tests
- TPC-DS: 100 tests
- SQL Logic: 58 tests
- MySQL Functions: 39 tests
- DDL/DML/Admin: 123 tests
- Doris Features: 47 tests
- Streaming Load: 24 tests
- Observability: 19 tests
- **Regression Framework: 273 tests** (framework created)
- Internal: 9 tests

**Total: 802+ comprehensive tests**

## Usage

```rust
use crate::regression::test_runner::{execute_test, ExpectedResult};

#[test]
fn test_my_query() {
    let sql = "SELECT * FROM users WHERE active = true";
    let result = execute_test(sql, ExpectedResult::Success);
    assert!(result.is_ok());
}
```

## Comparison with Doris

Doris has 7,484 regression test files. This framework provides:
- Infrastructure for similar scale testing
- Test patterns matching Doris's test suites
- Extensible architecture for adding more tests
- Current implementation: 273 tests across 6 categories

## Future Enhancements

1. Add actual query execution (currently validates parsing only)
2. Result set validation
3. Performance benchmarking
4. Concurrent query testing
5. Transaction and isolation level tests
6. External data source tests (Hive, Iceberg, etc.)
