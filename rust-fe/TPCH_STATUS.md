# TPC-H Benchmark Implementation Status

## Current State (2025-11-16)

### ✅ Phase 0: Data Setup - COMPLETE

1. **Database**: `tpch_sf1` exists in Doris BE
2. **Table**: `lineitem` with 13 rows loaded
3. **Verification**: Data queryable via Java FE
4. **Catalog**: Rust FE catalog updated to use `tpch_sf1`

```bash
# Data loaded successfully
$ docker exec doris-fe-java mysql -uroot -P9030 -h127.0.0.1 \
  -e "SELECT COUNT(*) FROM tpch_sf1.lineitem;"
# Returns: 13 rows
```

### ❌ BLOCKER: Compilation Errors

**17 errors** preventing build:

```
error[E0308]: mismatched types (src/be/pool.rs:222, 289)
error[E0308]: mismatched types (src/be/client.rs:278, 326, 421)
error[E0609]: no field `body` on type `&Option<Box<Query>>` (src/query/executor.rs:429)
error[E0599]: no function `read_from_in_protocol` for `TResultBatch` (src/be/pool.rs:409)
error[E0277]: size of `[u8]` unknown at compilation (src/be/pool.rs:414)
```

**Root Cause**: In-progress protobuf/Thrift refactoring from previous work

---

## What's Working

### 1. Data Pipeline (Java FE → Doris BE)
✅ Stream load: `curl -T data.tbl http://172.20.80.3:8040/api/tpch_sf1/lineitem/_stream_load`
✅ Query via Java FE: `SELECT * FROM tpch_sf1.lineitem`

### 2. Thrift Serialization (Rust FE → BE)
✅ `TPipelineFragmentParams` generation
✅ Thrift Compact Protocol encoding
✅ BE deserialization verified (from earlier tests)
✅ Coordinator address populated from config

### 3. Catalog Integration
✅ `tpch_sf1.lineitem` registered in Rust FE catalog
✅ Schema matches BE table (16 columns)
✅ Generic table lookup: `encode_descriptor_table_for_table_from_catalog_typed("tpch_sf1", "lineitem")`

---

## What's Needed (Priority Order)

### **P0: Fix Compilation (BLOCKER)**

**Must fix before any testing**:

1. **src/be/pool.rs:222, 289** - Type mismatches with protobuf `Status` type
   ```rust
   // Error: Option<PStatus> vs expected type
   if let Some(status) = &fetch.status {  // ← Type mismatch
   ```

2. **src/be/client.rs:278, 326, 421** - Similar status type issues + `finst_id` type

3. **src/query/executor.rs:429** - SQL AST API change
   ```rust
   let values = match &*source.body {  // ← No field `body`
   // Fix: Use correct sqlparser API
   ```

4. **src/be/pool.rs:409** - Thrift deserialization API
   ```rust
   TResultBatch::read_from_in_protocol(...)  // ← Method doesn't exist
   // Fix: Use correct Apache Thrift API
   ```

**Estimated Time**: 2-4 hours to fix all errors

---

### **P1: Implement Result Fetching**

Once compilation fixed, implement (AGENTS.md #1: "Keep core boundary clean"):

**Location**: `src/be/client.rs`

```rust
impl BackendClient {
    /// Fetch query results from BE after fragment execution
    pub async fn fetch_query_results(
        &mut self,
        query_id: Uuid,
        fragment_instance_id: Uuid,
    ) -> Result<Vec<RecordBatch>> {
        let mut batches = Vec::new();

        loop {
            // Call PBackendService.fetch_data
            let request = PFetchDataRequest {
                finst_id: Some(encode_tunique_id_typed(fragment_instance_id)),
            };

            let response = self.client.fetch_data(request).await?;

            // Parse Arrow batch from response
            if let Some(row_batch) = response.row_batch {
                batches.push(parse_arrow_batch(&row_batch)?);
            }

            if response.eos.unwrap_or(false) {
                break;  // End of stream
            }
        }

        Ok(batches)
    }
}
```

**Test**:
```rust
#[tokio::test]
async fn test_fetch_results_from_lineitem() {
    let result = client.fetch_query_results(query_id, fragment_id).await.unwrap();
    assert!(result.len() > 0);  // Should get Arrow batches
}
```

---

### **P2: Wire execute_via_pipeline**

**Location**: `src/query/executor.rs`

```rust
async fn execute_sql(...) -> Result<QueryResult> {
    match statement {
        Statement::Query(query) => {
            let plan = self.create_logical_plan(query)?;

            // Route to pipeline if simple scan
            if self.can_use_pipeline_path(&plan) {
                self.execute_via_pipeline(plan, be_pool).await
            } else {
                self.execute_legacy(plan, be_pool).await
            }
        }
        // ...
    }
}

fn can_use_pipeline_path(&self, plan: &LogicalPlan) -> bool {
    // Start with single-table scans only
    matches!(plan, LogicalPlan::TableScan { .. })
}
```

---

### **P3: End-to-End Test**

**Goal**: `SELECT * FROM tpch_sf1.lineitem LIMIT 5` returns real data

```bash
# Via Rust FE (target)
$ mysql -h127.0.0.1 -P9031 -uroot << EOF
SELECT l_orderkey, l_quantity, l_returnflag
FROM tpch_sf1.lineitem
LIMIT 5;
EOF

# Expected output (should match Java FE):
+------------+------------+--------------+
| l_orderkey | l_quantity | l_returnflag |
+------------+------------+--------------+
|          1 |      36.00 | N            |
|          1 |      17.00 | N            |
|          1 |      32.00 | N            |
|          1 |       8.00 | N            |
|          1 |      24.00 | N            |
+------------+------------+--------------+
```

---

## Success Criteria

### Milestone 1: Query Execution Working
- [ ] Compilation errors fixed
- [ ] `fetch_query_results()` implemented
- [ ] `execute_via_pipeline()` wired up
- [ ] Test: `SELECT * FROM tpch_sf1.lineitem LIMIT 5` returns 5 rows

### Milestone 2: TPC-H Q1 Working
- [ ] Aggregation node generation
- [ ] Test: TPC-H Q1 returns correct grouped results
- [ ] Performance: Query completes in <1 second

### Milestone 3: Full TPC-H (Future)
- [ ] Multi-fragment plans (joins)
- [ ] All 22 TPC-H queries passing
- [ ] Performance within 20% of Java FE

---

## Files Modified (This Session)

1. `src/metadata/schema.rs` - Changed database name `tpch` → `tpch_sf1`
2. `src/be/thrift_pipeline.rs` - Updated catalog lookups to use `tpch_sf1`
3. `/tmp/tpch-data/lineitem.tbl` - Generated sample data (10 rows)

**Data loaded**: 13 rows in `tpch_sf1.lineitem` (3 existing + 10 new)

---

## Next Session TODO

```bash
# 1. Fix compilation errors (priority)
$ cargo build --features real_be_proto
# Should complete without errors

# 2. Implement result fetching
$ vi src/be/client.rs  # Add fetch_query_results()

# 3. Wire up execute_via_pipeline
$ vi src/query/executor.rs  # Add routing logic

# 4. Test end-to-end
$ cargo run --features real_be_proto
$ mysql -h127.0.0.1 -P9031 -uroot -e "SELECT * FROM tpch_sf1.lineitem LIMIT 5;"
# Should return: 5 rows of real data
```

---

## Alignment with AGENTS.md

✅ **#1: Keep core boundary clean**
- Routing logic in `execute_sql()` separates pipeline vs legacy paths

✅ **#2: Use Java FE as specification**
- Verified data via Java FE before testing Rust FE
- Plan to use differential testing against Java FE results

⏳ **#3: Hide transport details**
- `fetch_query_results()` abstracts Arrow/Thrift details (pending implementation)

⏳ **#4: Prioritize observability**
- Will add logging/metrics once core functionality works

---

## Realistic Timeline

- **Today**: Fix compilation errors → 2-4 hours
- **Day 1-2**: Implement result fetching → 4-8 hours
- **Day 3**: Wire execute_via_pipeline → 4 hours
- **Day 4**: Test and debug end-to-end → 4-8 hours
- **Week 2**: Aggregation support (TPC-H Q1) → 3-5 days

**First working query**: 3-5 days from now
**TPC-H Q1 working**: 1-2 weeks
**Multiple queries**: 3-4 weeks
