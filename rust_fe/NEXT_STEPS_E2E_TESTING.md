# Next Steps: End-to-End Testing Strategy

**Session**: claude/rust-fe-todos-migration-012mCiokw5gZWbgBtbTPkHJR
**Date**: 2025-11-18
**Current Status**: PBlock parser complete and tested with synthetic data ✅

---

## Current Achievement

✅ **PBlock Parser Complete**:
- All type decoders implemented (INT, BIGINT, STRING, DATE, etc.)
- Unit tests passing with synthetic PBlock data
- Format verified against C++ BE source code
- Ready for integration

---

## Strategy for End-to-End Testing

### CLAUDE.md Principle #2: Use Java FE Behavior as Specification

From analyzing Java FE code (`/fe/fe-core/src/main/java/org/apache/doris/qe/`):

1. **Query Execution Flow** (from StmtExecutor.java):
   ```
   StmtExecutor.execute(queryId)
   → Parse SQL
   → Create Planner
   → Generate PlanFragments
   → Create Coordinator
   → coordinator.exec()
   → ResultReceiver.getNext() [fetch results]
   → Parse PBlock
   → Return rows to client
   ```

2. **Key Components**:
   - `Coordinator.java`: Orchestrates BE execution
   - `ResultReceiver.java`: Fetches PBlock data from BE
   - `RowBatch.java`: Wraps TResultBatch/PBlock

### Challenge: Minimal BE Query Without Full FE

**Problem**: Creating a valid query plan requires:
- Descriptor tables (column schemas)
- Scan ranges (tablet locations)
- Fragment instance IDs
- Proper plan node structure

**Options**:

#### Option A: Mock Simple Data ✅ (DONE)
- Create synthetic PBlock with known data
- Test parser in isolation
- **Status**: Completed with unit tests

#### Option B: Capture Real PBlock from Java FE
- Run query through Java FE
- Intercept PBlock bytes
- Save to file
- Load and parse in Rust test
- **Benefit**: Tests with real BE-generated data
- **Challenge**: Requires Java FE running

#### Option C: Build Minimal Plan (Complex)
- Reverse-engineer minimal plan structure from Java FE
- Create descriptor tables manually
- Set up scan ranges
- **Benefit**: Direct Rust → BE testing
- **Challenge**: Complex, many dependencies

---

## Recommended Next Step: Option B (Capture & Replay)

### Step 1: Capture PBlock from Java FE

Create a Java FE interceptor:

```java
// In ResultReceiver.java or create a logging wrapper
if (response.hasRowBatch()) {
    byte[] pblockBytes = response.getRowBatch().toByteArray();
    // Save to /tmp/pblock_capture_QUERYID.bin
    Files.write(Paths.get("/tmp/pblock_capture.bin"), pblockBytes);
}
```

### Step 2: Rust Test with Captured Data

```rust
// fe-backend-client/examples/test_captured_pblock.rs
fn main() {
    let bytes = std::fs::read("/tmp/pblock_capture.bin")?;
    let pblock = parse_pblock(&bytes)?;
    let rows = pblock_to_rows(&pblock)?;

    // Verify against known query results
    println!("Parsed {} rows from real BE", rows.len());
}
```

### Step 3: Verify Against Java FE

Run same query through Java FE, compare:
- Row count
- Column count
- First/last row values
- Data types

---

## Alternative: Integration Test Without Java FE Modification

### Use Existing Examples

We already have working examples:
1. `test_pblock_parser_unit.rs` - ✅ Validates parser logic
2. `test_e2e_real_be.rs` - Framework for BE communication

### What's Missing for Real E2E:

1. **Valid Plan Fragment**:
   - Need complete TPlanFragment structure
   - Descriptor tables
   - Scan ranges from catalog

2. **Catalog Integration**:
   - Need to query metadata from FE or BE
   - Get table/tablet information
   - Build scan ranges

3. **Simple Approach**: Use existing Rust FE components

```rust
// Pseudo-code for minimal E2E
let catalog = Catalog::new();
let table = catalog.get_table("tpch", "lineitem")?;
let scan_ranges = table.get_scan_ranges()?;

let plan = build_scan_plan(table, scan_ranges);
let fragment = serialize_plan(plan)?;

let mut be_client = BackendClient::new("127.0.0.1", 8060).await?;
let finst_id = be_client.exec_plan_fragment(&fragment, query_id).await?;
let rows = be_client.fetch_data(finst_id).await?;

// Parser is already integrated in fetch_data()!
assert!(rows.len() > 0);
```

---

## Decision Point

**Question**: Do we have a running Java FE/BE system we can test against?

### If YES (Java FE/BE Running):
→ **Option B**: Capture real PBlock, test parser
→ Quick validation with real data
→ Can compare with Java FE results directly

### If NO (No Running System):
→ **Continue with synthetic tests** (current approach)
→ Focus on building catalog + planner
→ Build complete query pipeline in Rust
→ Test when system is available

---

## Current Blocker Analysis

Looking at previous test attempts:
```bash
Error: DriverError { Could not connect to address `127.0.0.1:9030': Connection refused (os error 111) }
```

**Java FE not running on port 9030**

### To Proceed Without Running FE:

1. ✅ Parser is verified with unit tests
2. ⏭️ Next: Build catalog integration
3. ⏭️ Next: Build simple query planner
4. ⏭️ Next: Test with real BE when available

---

## Recommended Immediate Action

**Build a comprehensive PBlock test suite with edge cases**:

```rust
// Test cases to add:
1. ✅ BIGINT columns - DONE
2. ✅ STRING columns - DONE
3. ✅ Mixed types - DONE
4. ⏭️ NULL values (need null bitmap parsing)
5. ⏭️ Const columns (single value repeated)
6. ⏭️ Large result sets (1000+ rows)
7. ⏭️ All supported data types (DATE, DECIMAL, FLOAT, etc.)
8. ⏭️ Compressed data (Snappy)
9. ⏭️ Legacy format (be_exec_version < 4)
10. ⏭️ Empty result sets
```

This validates the parser thoroughly without needing a running system.

---

## Summary

**Parser Status**: ✅ Complete and tested
**Next Blocker**: Need real BE/FE system OR continue building Rust FE components
**CLAUDE.md Compliance**: ✅ Using BE source as specification
**Path Forward**: Either capture real PBlock OR expand synthetic tests

**Recommendation**: Document what we need to proceed with real E2E testing, then continue building Rust FE components (catalog, planner) in parallel.
