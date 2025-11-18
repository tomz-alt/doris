# C++ BE Integration Test Plan

## Current Status

**Blockers**:
- ⚠️ protoc installation required for gRPC bindings generation
- ⚠️ C++ BE must be running on port 9060

**Workaround**: Using MockBackend for test-driven development (6 tests passing)

## Test Scenarios (Ready for Real BE)

### 1. Connection & Health Check
```rust
#[tokio::test]
async fn test_connect_to_cpp_be() {
    // GIVEN: C++ BE running on localhost:9060
    let client = BackendClient::new("127.0.0.1", 9060).await;

    // THEN: Connection succeeds
    assert!(client.is_ok(), "Should connect to C++ BE");
}
```

**Java FE equivalent**: `BackendServiceClient.connect()`

### 2. Execute TPC-H Q1 Fragment
```rust
#[tokio::test]
async fn test_tpch_q1_exec_plan_fragment() {
    // GIVEN: TPC-H database with lineitem table loaded
    let mut client = BackendClient::new("127.0.0.1", 9060).await.unwrap();
    let query_id = uuid::Uuid::new_v4().as_bytes();

    // WHEN: Execute TPC-H Q1 plan fragment
    let fragment = create_tpch_q1_fragment();  // See fe-planner for generation
    let result = client.exec_plan_fragment(&fragment, *query_id).await;

    // THEN: Fragment executes successfully, returns fragment instance ID
    assert!(result.is_ok());
    let finst_id = result.unwrap();
    assert_ne!(finst_id, [0u8; 16], "Should return valid fragment instance ID");
}
```

**Java FE equivalent**: `Coordinator.exec()`

### 3. Fetch TPC-H Q1 Results
```rust
#[tokio::test]
async fn test_tpch_q1_fetch_data() {
    // GIVEN: TPC-H Q1 fragment executed (from test above)
    let mut client = BackendClient::new("127.0.0.1", 9060).await.unwrap();
    let finst_id = execute_tpch_q1_fragment(&mut client).await;

    // WHEN: Fetch result data
    let rows = client.fetch_data(finst_id).await.unwrap();

    // THEN: Returns expected TPC-H Q1 result (4 rows for standard dataset)
    assert_eq!(rows.len(), 4, "TPC-H Q1 should return 4 rows");

    // Verify row structure (10 columns)
    assert_eq!(rows[0].values.len(), 10);

    // Verify column types match schema
    assert!(matches!(rows[0].values[0], Value::Varchar(_)));  // l_returnflag
    assert!(matches!(rows[0].values[1], Value::Varchar(_)));  // l_linestatus
    assert!(matches!(rows[0].values[2], Value::Decimal(_)));  // sum_qty
    assert!(matches!(rows[0].values[3], Value::Decimal(_)));  // sum_base_price
    // ... verify all 10 columns
}
```

**Java FE equivalent**: `Coordinator.fetchData()`

### 4. Verify Results Match Java FE
```rust
#[tokio::test]
async fn test_rust_fe_matches_java_fe_results() {
    // GIVEN: Same C++ BE, same TPC-H database
    // AND: Java FE executed TPC-H Q1, results saved to /tmp/java_fe_q1_results.json
    // AND: Rust FE executes TPC-H Q1

    let rust_results = execute_tpch_q1_rust_fe().await;
    let java_results = load_java_fe_results("/tmp/java_fe_q1_results.json");

    // THEN: Results are 100% identical
    assert_eq!(rust_results.len(), java_results.len());

    for (i, (rust_row, java_row)) in rust_results.iter().zip(java_results.iter()).enumerate() {
        assert_eq!(
            rust_row, java_row,
            "Row {} mismatch between Rust FE and Java FE", i
        );
    }
}
```

**This is the CRITICAL test** - verifies Rust FE is drop-in compatible with Java FE.

### 5. Error Handling - Non-Existent Table
```rust
#[tokio::test]
async fn test_query_nonexistent_table() {
    // GIVEN: Query on non-existent table
    let sql = "SELECT * FROM nonexistent_table";

    // WHEN: Execute via Rust FE
    let result = execute_query_rust_fe(sql).await;

    // THEN: Returns same error as Java FE
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Table does not exist"));
    // TODO: Verify exact error code matches Java FE
}
```

### 6. Multiple Fragments (Distributed Query)
```rust
#[tokio::test]
async fn test_distributed_query_multiple_fragments() {
    // GIVEN: Query requiring multiple BE nodes (JOIN, shuffle)
    let sql = r#"
        SELECT o.o_orderkey, l.l_quantity
        FROM orders o
        JOIN lineitem l ON o.o_orderkey = l.l_orderkey
        LIMIT 10
    "#;

    // WHEN: Rust FE plans and executes
    let fragments = plan_query(sql);  // Returns Vec<TPlanFragment>
    assert!(fragments.len() > 1, "JOIN should create multiple fragments");

    // Execute all fragments
    let results = execute_distributed_query(fragments).await;

    // THEN: Results match Java FE execution
    assert!(results.is_ok());
    assert_eq!(results.unwrap().len(), 10);
}
```

## Test Data Setup

### Prerequisites
```bash
# 1. Start C++ BE
cd /path/to/doris/be
./bin/start_be.sh

# 2. Start Java FE (for loading test data)
cd /path/to/doris/fe
./bin/start_fe.sh

# 3. Load TPC-H data (scale factor 1)
mysql -h 127.0.0.1 -P 9030 -u root
CREATE DATABASE tpch;
USE tpch;
# Run TPC-H schema creation
# Run TPC-H data load
```

### Test Data Verification
```sql
-- Verify lineitem table
SELECT COUNT(*) FROM lineitem;  -- Should be 6,001,215 for SF=1

-- Verify TPC-H Q1 baseline results
SELECT
    l_returnflag,
    l_linestatus,
    sum(l_quantity) as sum_qty,
    sum(l_extendedprice) as sum_base_price,
    sum(l_extendedprice * (1 - l_discount)) as sum_disc_price,
    sum(l_extendedprice * (1 - l_discount) * (1 + l_tax)) as sum_charge,
    avg(l_quantity) as avg_qty,
    avg(l_extendedprice) as avg_price,
    avg(l_discount) as avg_disc,
    count(*) as count_order
FROM lineitem
WHERE l_shipdate <= date '1998-12-01' - interval '90' day
GROUP BY l_returnflag, l_linestatus
ORDER BY l_returnflag, l_linestatus;

-- Expected: 4 rows
-- A F, A O, N F, N O
```

## Running Integration Tests

### With MockBackend (Current - No protoc needed)
```bash
cd /home/user/doris/rust_fe
cargo test --package fe-backend-client
# 6 tests pass
```

### With Real C++ BE (Future - Requires protoc + running BE)
```bash
# 1. Install protoc
sudo apt-get install protobuf-compiler

# 2. Rebuild with protobuf generation
cd /home/user/doris/rust_fe
cargo build --package fe-backend-client

# 3. Start C++ BE
# (See Test Data Setup above)

# 4. Run integration tests
cargo test --package fe-backend-client --features real-be -- --test-threads=1

# 5. Run full system test (Rust FE + C++ BE)
cargo run --bin doris-fe &
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT * FROM tpch.lineitem LIMIT 10"
```

## Success Criteria

**Phase 1**: MockBackend (✅ Complete)
- [x] 6 tests passing with MockBackend
- [x] Test infrastructure ready for real BE

**Phase 2**: Real C++ BE Connection
- [ ] protoc installed and bindings generated
- [ ] Connection to C++ BE succeeds
- [ ] exec_plan_fragment RPC works
- [ ] fetch_data RPC works

**Phase 3**: TPC-H Q1 Execution
- [ ] TPC-H Q1 executes on real data
- [ ] Results are retrieved successfully
- [ ] Results match Java FE 100%

**Phase 4**: Full Compatibility
- [ ] All TPC-H queries (Q1-Q22) work
- [ ] Error handling matches Java FE
- [ ] Performance within 10% of Java FE
- [ ] Distributed queries work (multiple BE nodes)

## Known Limitations

1. **protoc not available in current environment** - Prevents gRPC binding generation
2. **No C++ BE running** - Prevents real integration testing
3. **Using MockBackend** - Returns dummy data, good for API testing only

## Next Steps

1. **Install protoc** - Required for production deployment
2. **Start C++ BE** - Needed for real integration tests
3. **Load TPC-H data** - Baseline for result verification
4. **Run tests** - Execute all 6 integration test scenarios
5. **Compare results** - Verify 100% match with Java FE
