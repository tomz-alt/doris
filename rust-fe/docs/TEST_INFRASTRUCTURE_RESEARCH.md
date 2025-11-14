# Doris FE Test Infrastructure Research

## Executive Summary

This document summarizes research on the Apache Doris FE (Frontend) test infrastructure, particularly focusing on MySQL compatibility tests, SQL logic tests, and the regression test framework. This research informs the design of the Rust FE test suite.

## Test Infrastructure Overview

### 1. Regression Test Framework (Groovy-based)

**Location**: `/home/user/doris/regression-test/`

**Key Statistics**:
- **7,484 test files** in the regression-test/suites directory
- Written in **Groovy** (JVM-based scripting language)
- Tests execute against running Doris FE and BE instances via JDBC

**Framework Structure**:
```
regression-test/
├── common/              # Shared test resources (SQL, tables)
├── conf/                # Configuration files
├── data/                # Test data files (CSV, JSON, etc.)
├── framework/           # Core test framework (Groovy)
│   └── src/main/groovy/org/apache/doris/regression/
│       ├── Config.groovy
│       ├── RegressionTest.groovy
│       └── action/      # Test action implementations
├── plugins/             # Test plugins
├── script/              # Helper scripts
└── suites/              # Test suites (211 directories)
```

**Test Execution**: `./run-regression-test.sh`

### 2. MySQL Compatibility Tests

**Location**: `/home/user/doris/regression-test/suites/mysql_compatibility_p0/`

**Test Files**:
1. `header.groovy` - Tests MySQL result set header compatibility
2. `metadata.groovy` - Tests column/table metadata compatibility

**Example Test Structure**:
```groovy
suite ("header") {
    def (result, meta) = JdbcUtils.executeToList(
        context.getConnection(),
        "select connection_id(), database()"
    )

    // Verify MySQL protocol compatibility
    assertEquals("connection_id()", meta.getColumnName(1))
    assertEquals("database()", meta.getColumnLabel(2))
}
```

**Key Aspects Tested**:
- Column names and labels
- Table names in result metadata
- MySQL function compatibility (connection_id(), database(), etc.)
- View metadata handling
- Result set structure

### 3. MySQL Protocol Tests (Java Unit Tests)

**Location**: `/home/user/doris/fe/fe-core/src/test/java/org/apache/doris/mysql/`

**Test Files** (16 total):
```
- MysqlAuthPacketTest.java       # Authentication packet parsing
- MysqlCapabilityTest.java       # Client capability negotiation
- MysqlChannelTest.java          # Communication channel
- MysqlColDefTest.java           # Column definition packets
- MysqlColTypeTest.java          # MySQL column type mapping
- MysqlCommandTest.java          # MySQL command processing
- MysqlEofPacketTest.java        # EOF packet handling
- MysqlErrPacketTest.java        # Error packet formatting
- MysqlHandshakePacketTest.java  # Connection handshake
- MysqlOkPacketTest.java         # OK packet responses
- MysqlPasswordTest.java         # Password authentication
- MysqlProtoTest.java            # Protocol-level integration
- MysqlSerializerVarbinaryTest.java # Binary data serialization
- ConnectionExceedTest.java      # Connection limit testing
```

**Testing Framework**: JUnit with Mockit for mocking

**Example Test Pattern**:
```java
@Test
public void testAuthPacket() {
    // Setup
    MysqlAuthPacket packet = new MysqlAuthPacket();

    // Mock channel and environment
    new Expectations() {{
        channel.fetchOnePacket();
        result = mockPacket;
    }};

    // Execute
    packet.read(channel);

    // Verify
    Assert.assertEquals(expectedUser, packet.getUser());
    Assert.assertArrayEquals(expectedAuth, packet.getAuthResponse());
}
```

### 4. SQL Query Tests (Nereids Optimizer)

**Location**: `/home/user/doris/fe/fe-core/src/test/java/org/apache/doris/nereids/sqltest/`

**Base Class**: `SqlTestBase.java` (extends TestWithFeService)

**Test Categories**:
1. **Join Tests**
   - `JoinTest.java` - Basic join operations
   - `MultiJoinTest.java` - Complex multi-table joins
   - `CascadesJoinReorderTest.java` - Join reordering optimization
   - `JoinOrderJobTest.java` - TPC-H join order optimization

2. **Expression Tests**
   - `ExpressionRewriteSqlTest.java` - Expression rewriting rules
   - `SimplifyComparisonPredicateSqlTest.java` - Predicate simplification

3. **General Tests**
   - `SortTest.java` - Order by operations
   - `InferTest.java` - Type inference

**Test Structure**:
```java
public abstract class SqlTestBase extends TestWithFeService {
    @Override
    protected void runBeforeAll() throws Exception {
        createDatabase("test");
        connectContext.setDatabase("test");

        // Create test tables (TPC-H schema)
        createTables(
            "CREATE TABLE orders (...)",
            "CREATE TABLE lineitem (...)",
            // ...
        );
    }

    // Test utility methods available to subclasses
}
```

### 5. Regression Test Suites

**Major Test Suite Categories**:

| Suite | Description | Test Count (est.) |
|-------|-------------|-------------------|
| `query_p0/` | Core query functionality | 500+ |
| `query_p1/` | Extended query tests | 200+ |
| `datatype_p0/` | Data type compatibility | 300+ |
| `auth_p0/` | Authentication/authorization | 100+ |
| `external_table_p0/` | External data sources | 200+ |
| `mysql_compatibility_p0/` | MySQL compatibility | 2 |
| `mysql_ssl_p0/` | MySQL SSL connections | 5+ |
| `tpch_*` | TPC-H benchmarks | 50+ |

**Test Suite Structure** (query_p0 example):
```
query_p0/
├── aggregate/          # Aggregation functions
├── join/              # Join operations
├── operator/          # Arithmetic/logical operators
├── sql_functions/     # Built-in functions
├── grouping_sets/     # GROUPING SETS, ROLLUP, CUBE
├── window_functions/  # Window/analytical functions
├── subquery/          # Subquery handling
└── ...
```

### 6. Test Action Framework

**Location**: `/home/user/doris/regression-test/framework/src/main/groovy/org/apache/doris/regression/action/`

**Available Actions**:
```groovy
- BenchmarkAction    # Performance benchmarking
- CreateMVAction     # Materialized view testing
- ExplainAction      # Query plan validation
- HttpCliAction      # HTTP API testing
- ProfileAction      # Query profiling
- StreamLoadAction   # Data loading tests
- TestAction         # Generic SQL testing
- WaitForAction      # Async operation waiting
```

**Usage Example** (from test files):
```groovy
suite("test_name") {
    // Setup
    sql """ DROP TABLE IF EXISTS test_table """
    sql """ CREATE TABLE test_table (...) """

    // Load data
    streamLoad {
        table "test_table"
        file "test_data.csv"
        set 'column_separator', '|'
        check { result, exception, startTime, endTime ->
            // Verify load succeeded
        }
    }

    // Execute queries
    qt_sql """ SELECT COUNT(*) FROM test_table """

    // Verify results
    order_qt_sql """ SELECT * FROM test_table ORDER BY id """
}
```

## Key Testing Patterns

### 1. MySQL Protocol Compatibility

**What's Tested**:
- Authentication (native_password, sha256_password)
- Packet serialization/deserialization
- Column type mapping (MySQL ↔ Doris)
- Result set metadata (JDBC ResultSetMetaData)
- Connection parameters (character sets, timezones)
- Error responses

**Test Approach**:
```java
// Mock MySQL channel
@Mocked MysqlChannel channel;

// Test packet serialization
ByteBuffer buffer = ByteBuffer.allocate(1024);
packet.writeTo(buffer);

// Verify byte-level correctness
Assert.assertEquals(expectedBytes, buffer.array());
```

### 2. SQL Compatibility Testing

**What's Tested**:
- SQL parsing (ANTLR-based)
- Query planning (Nereids optimizer)
- Result correctness
- Performance characteristics

**Test Patterns**:
```groovy
// Quick test (no ordering required)
qt_sql """ SELECT COUNT(*) FROM table """

// Ordered test (deterministic results)
order_qt_sql """ SELECT * FROM table ORDER BY id """

// Explain plan validation
explain {
    sql """ SELECT * FROM table WHERE condition """
    contains "SCAN", "FILTER"
}
```

### 3. Compatibility Tests

**Purpose**: Ensure upgrades don't break existing functionality

**Structure**:
```
test_suite/
├── load.groovy       # Create resources before upgrade
└── verify.groovy     # Verify resources after upgrade
```

**Tagged with**: `restart_fe` group label

**Example**:
```groovy
// load.groovy - runs before FE restart
suite("udf_compatibility_load") {
    sql """ CREATE FUNCTION my_udf(...) """
    sql """ CREATE TABLE test_table """
}

// verify.groovy - runs after FE restart
suite("udf_compatibility_verify") {
    // Verify UDF still works
    qt_sql """ SELECT my_udf(col) FROM test_table """
}
```

## Test Data Management

**Data Location**: `/home/user/doris/regression-test/data/`

**Data Organization**:
```
data/
├── account_p0/      # Account management test data
├── datatype_p0/     # Type-specific test data
├── query_p0/        # Query test datasets
├── tpch/            # TPC-H benchmark data
└── ...
```

**Common Formats**:
- CSV files (with custom delimiters)
- JSON documents
- Parquet/ORC files
- SQL scripts

## Test Configuration

**Main Config**: `/home/user/doris/regression-test/conf/regression-conf.groovy`

**Key Settings**:
- FE connection parameters (host, port, user, password)
- BE connection details
- Test database name
- Parallel execution settings
- Timeout configurations

## Best Practices from Java FE Tests

### 1. Isolation
- Each test creates its own tables
- Use unique table names to avoid conflicts
- Clean up resources after tests

### 2. Determinism
- Use fixed timestamps instead of `now()`
- Order results for comparison
- Avoid global state modifications

### 3. Concurrency
- Mark tests that modify global state as `nonConcurrent`
- Use local session variables instead of global
- Synchronize after stream loads in multi-FE environments

### 4. Error Handling
- Test both success and failure cases
- Verify error messages
- Test edge cases and boundary conditions

## Gaps and Opportunities for Rust FE

### Current Java FE Test Gaps

1. **Limited MySQL Protocol Coverage**
   - Focus on basic packet types
   - Limited SSL/TLS testing
   - Few compression tests
   - Limited prepared statement testing

2. **Performance Tests**
   - Mostly correctness-focused
   - Limited latency/throughput benchmarks
   - Few concurrency stress tests

3. **Integration Tests**
   - Tests assume running cluster
   - Limited fault injection
   - Few multi-FE scenarios

### Rust FE Testing Opportunities

1. **Property-Based Testing**
   - Use `proptest` for SQL generation
   - Fuzz testing for protocol
   - QuickCheck-style invariant testing

2. **Benchmark Integration**
   - Criterion.rs for performance regression
   - Comparative benchmarks vs Java FE
   - Memory usage profiling

3. **Mock-Based Testing**
   - Mock BE for unit testing (✓ Already implemented!)
   - Hermetic test environments
   - Fast test execution

4. **SQL Logic Tests**
   - Adopt SQLLogicTest format (.slt files)
   - Share tests with DuckDB, PostgreSQL
   - Automated compatibility checking

## Recommendations for Rust FE Test Suite

### Phase 1: Foundation (Current)
✅ Mock BE gRPC server (implemented)
✅ Integration tests (implemented)
- [ ] MySQL protocol unit tests
- [ ] SQL parser tests
- [ ] Query planner tests

### Phase 2: Compatibility
- [ ] MySQL protocol compatibility suite
- [ ] SQL compatibility tests (TPC-H, TPC-DS)
- [ ] Result format compatibility
- [ ] JDBC driver compatibility

### Phase 3: Performance
- [ ] Latency benchmarks
- [ ] Throughput tests
- [ ] Concurrent query handling
- [ ] Memory usage profiling

### Phase 4: Advanced
- [ ] Fuzz testing
- [ ] Property-based testing
- [ ] Chaos engineering (fault injection)
- [ ] Long-running stability tests

## Test Framework Comparison

| Aspect | Java FE | Rust FE (Proposed) |
|--------|---------|-------------------|
| Language | Groovy | Rust |
| Unit Tests | JUnit + Mockit | cargo test + mockall |
| Integration | Regression framework | Mock BE + real tests |
| Coverage | ~7,500 tests | TBD (start with critical paths) |
| Execution | Requires cluster | Can run hermetically |
| Performance | Slower (JVM) | Faster (native) |
| CI/CD | Jenkins | GitHub Actions (recommended) |

## References

### Key Files Reviewed

1. **Regression Tests**:
   - `/home/user/doris/regression-test/README.md`
   - `/home/user/doris/regression-test/suites/mysql_compatibility_p0/`
   - `/home/user/doris/regression-test/suites/query_p0/`

2. **Unit Tests**:
   - `/home/user/doris/fe/fe-core/src/test/java/org/apache/doris/mysql/`
   - `/home/user/doris/fe/fe-core/src/test/java/org/apache/doris/nereids/sqltest/`

3. **Framework**:
   - `/home/user/doris/regression-test/framework/src/main/groovy/org/apache/doris/regression/`

### Related Documentation
- Doris regression test guide
- MySQL protocol documentation
- JDBC ResultSetMetaData specification
- TPC-H benchmark specification

## Conclusion

The Doris Java FE has a comprehensive test infrastructure with:
- **7,484 regression tests** covering query functionality
- **16 MySQL protocol unit tests** for wire protocol compatibility
- **Groovy-based framework** for end-to-end testing
- **Strong focus on correctness** over performance

For the Rust FE, we can:
1. Learn from the test organization and patterns
2. Improve on hermetic testing (mock BE approach)
3. Add property-based and fuzz testing
4. Focus on performance regression detection
5. Build compatibility test suites to ensure parity

---

*Research conducted: 2025-11-14*
*Researcher: Claude (Anthropic)*
*For: Rust FE implementation project*
