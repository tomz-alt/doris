# Thrift Payload Verification Test Plan

## Goal
Ensure Rust FE generates **identical Thrift payloads** as Java FE for 100% compatibility with C++ BE.

## Strategy: Capture & Compare

### Phase 1: Capture Java FE Thrift Payload

#### Setup Java FE Test
```java
// In Java FE test: fe/fe-core/src/test/java/org/apache/doris/qe/ThriftPayloadTest.java
@Test
public void testTpcHLineitemScanPayload() throws Exception {
    // 1. Create TPC-H lineitem table
    createTable("CREATE TABLE lineitem (...16 columns...)");

    // 2. Parse simple query
    String sql = "SELECT l_orderkey, l_quantity FROM lineitem WHERE l_shipdate <= '1998-12-01'";

    // 3. Build plan
    Planner planner = new Planner();
    PlanFragment fragment = planner.plan(sql);

    // 4. Convert to Thrift
    TPlanFragment thriftFragment = fragment.toThrift();

    // 5. SERIALIZE TO JSON (for comparison)
    TSerializer serializer = new TSerializer(new TSimpleJSONProtocol.Factory());
    String json = serializer.toString(thriftFragment);

    // 6. Write to file
    Files.write(Paths.get("/tmp/java_plan.json"), json.getBytes());

    // 7. Also write binary
    TSerializer binSerializer = new TSerializer(new TBinaryProtocol.Factory());
    byte[] binary = binSerializer.serialize(thriftFragment);
    Files.write(Paths.get("/tmp/java_plan.bin"), binary);
}
```

#### Run Java Test
```bash
cd fe/fe-core
mvn test -Dtest=ThriftPayloadTest#testTpcHLineitemScanPayload

# Output files:
# /tmp/java_plan.json - Human-readable JSON
# /tmp/java_plan.bin  - Binary Thrift
```

### Phase 2: Generate Rust FE Thrift Payload

#### Implement Rust Test
```rust
// In Rust: fe-planner/tests/thrift_compat_test.rs
#[test]
fn test_tpch_lineitem_scan_payload() {
    // 1. Create catalog with lineitem
    let catalog = create_tpch_lineitem_catalog();

    // 2. Parse query
    let sql = "SELECT l_orderkey, l_quantity FROM lineitem WHERE l_shipdate <= '1998-12-01'";
    let stmt = DorisParser::parse_one(sql).unwrap();

    // 3. Build plan
    let planner = QueryPlanner::new(catalog);
    let plan = planner.plan(&stmt).unwrap();

    // 4. Convert to Thrift
    let thrift_fragment = plan.to_thrift().unwrap();

    // 5. SERIALIZE TO JSON (for comparison)
    let json = serde_json::to_string_pretty(&thrift_fragment).unwrap();
    std::fs::write("/tmp/rust_plan.json", json).unwrap();

    // 6. Also write binary
    let mut buf = Vec::new();
    thrift_fragment.write_to_out_protocol(&mut TBinaryProtocol::new(&mut buf)).unwrap();
    std::fs::write("/tmp/rust_plan.bin", buf).unwrap();
}
```

#### Run Rust Test
```bash
cargo test -p fe-planner test_tpch_lineitem_scan_payload

# Output files:
# /tmp/rust_plan.json - Human-readable JSON
# /tmp/rust_plan.bin  - Binary Thrift
```

### Phase 3: Compare Payloads

#### Binary Comparison (Exact Match Required)
```bash
# Binary must be IDENTICAL
diff /tmp/java_plan.bin /tmp/rust_plan.bin
# Should output: Files are identical
```

#### JSON Comparison (Field-by-field)
```bash
# Pretty-print both
jq '.' /tmp/java_plan.json > /tmp/java_pretty.json
jq '.' /tmp/rust_plan.json > /tmp/rust_pretty.json

# Compare
diff /tmp/java_pretty.json /tmp/rust_pretty.json
```

#### Expected Structure
```json
{
  "plan": {
    "nodes": [
      {
        "node_id": 0,
        "node_type": 0,  // OLAP_SCAN_NODE
        "num_children": 0,
        "limit": -1,
        "row_tuples": [0],
        "nullable_tuples": [false],
        "conjuncts": [
          {
            "nodes": [
              {
                "node_type": "BINARY_PRED",
                "opcode": "LE",
                "child_type": "DATE",
                "fn": {...}
              }
            ]
          }
        ],
        "olap_scan_node": {
          "tuple_id": 0,
          "key_column_name": ["l_orderkey", "l_partkey"],
          "is_preaggregation": true,
          "tablets_count": 3,
          "enable_tablet_internal_parallel": true
        }
      }
    ]
  },
  "output_sink": {...},
  "partition": {...}
}
```

## Test Cases

### Test 1: Simple Scan (No Predicates)
```sql
SELECT l_orderkey, l_quantity FROM lineitem LIMIT 10
```
**Expected**: Single OLAP_SCAN_NODE, no predicates, LIMIT node

### Test 2: Scan with Filter
```sql
SELECT * FROM lineitem WHERE l_shipdate <= '1998-12-01'
```
**Expected**: OLAP_SCAN_NODE with LE predicate on l_shipdate

### Test 3: Aggregation
```sql
SELECT l_returnflag, SUM(l_quantity) FROM lineitem GROUP BY l_returnflag
```
**Expected**: OLAP_SCAN_NODE → AGGREGATION_NODE

### Test 4: TPC-H Q1 (Full Query)
```sql
SELECT
    l_returnflag,
    l_linestatus,
    sum(l_quantity) as sum_qty,
    avg(l_quantity) as avg_qty,
    count(*) as count_order
FROM lineitem
WHERE l_shipdate <= date '1998-12-01' - interval '90' day
GROUP BY l_returnflag, l_linestatus
ORDER BY l_returnflag, l_linestatus
```
**Expected**: OLAP_SCAN_NODE → AGGREGATION_NODE → SORT_NODE

## Validation Checklist

For each test case, verify:
- [ ] Binary Thrift is byte-for-byte identical
- [ ] All node types match
- [ ] All field IDs match
- [ ] All data types match
- [ ] Predicate expressions match
- [ ] Column references match
- [ ] Tuple descriptors match
- [ ] BE can execute the plan successfully
- [ ] Results match expected output

## Tools

### Thrift Inspector (for debugging)
```bash
# Install Thrift tools
apt-get install thrift-compiler

# View binary Thrift as text
thrift -gen json /tmp/java_plan.bin > /tmp/java_plan.txt
thrift -gen json /tmp/rust_plan.bin > /tmp/rust_plan.txt

# Compare
diff -u /tmp/java_plan.txt /tmp/rust_plan.txt
```

### Automated Test Script
```bash
#!/bin/bash
# test_thrift_compat.sh

echo "=== Building Java Thrift Payload ==="
cd fe/fe-core
mvn test -Dtest=ThriftPayloadTest 2>&1 | tee /tmp/java_test.log

echo "=== Building Rust Thrift Payload ==="
cd ../../rust_fe
cargo test -p fe-planner thrift_compat 2>&1 | tee /tmp/rust_test.log

echo "=== Comparing Binary ==="
if cmp -s /tmp/java_plan.bin /tmp/rust_plan.bin; then
    echo "✅ Binary payloads are IDENTICAL"
else
    echo "❌ Binary payloads DIFFER"
    xxd /tmp/java_plan.bin > /tmp/java_hex.txt
    xxd /tmp/rust_plan.bin > /tmp/rust_hex.txt
    diff -u /tmp/java_hex.txt /tmp/rust_hex.txt | head -50
    exit 1
fi

echo "=== Comparing JSON Structure ==="
diff -u <(jq -S '.' /tmp/java_plan.json) <(jq -S '.' /tmp/rust_plan.json)

echo "✅ All tests passed!"
```

## Success Criteria

1. **Byte-level compatibility**: Binary Thrift must be identical
2. **BE compatibility**: BE must execute Rust-generated plans
3. **Result correctness**: Query results must match
4. **All TPC-H queries**: All 22 TPC-H queries produce matching payloads

## Benefits

✅ **Guaranteed compatibility** - If binary matches, BE will execute
✅ **Regression testing** - Detect any deviations immediately
✅ **No BE changes** - Proves we can reuse existing BE
✅ **Incremental validation** - Test each plan node type separately
✅ **Documentation** - JSON provides human-readable plan structure

## Next Steps

1. Write Java test to capture reference payloads
2. Generate Rust Thrift bindings
3. Implement query planner in Rust
4. Compare payloads for simple scan
5. Iterate until binary match
6. Expand to all TPC-H queries
