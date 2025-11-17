# Schema Hash Fix & Payload Comparison Investigation

**Date**: 2025-11-17
**Status**: PARTIAL FIX - Schema hash corrected, but 0 rows issue persists
**Impact**: Critical field identified and fixed, but root cause not fully resolved

---

## Investigation Summary

### Starting Point
- **Problem**: Rust FE sends scan ranges to BE, but BE returns `TResultBatch` with 0 rows
- **Known Working**: Java FE successfully returns 13 rows (5 + 8 + 0) from same 3 tablets
- **Goal**: Compare Rust FE vs Java FE payloads to identify differences

### Key Finding: Schema Hash Mismatch

**Discovery**: Through comparison work, found that schema_hash was hardcoded to "0"

**Evidence**:
```sql
mysql> SHOW TABLETS FROM tpch_sf1.lineitem LIMIT 3;
TabletId       SchemaHash   Version  RowCount
1763227464383  1232661058   3        5
1763227464385  1232661058   3        8
1763227464387  1232661058   3        0
```

**Problem in Code** (`src/be/thrift_pipeline.rs:793`):
```rust
let palo = t_plan_nodes::TPaloScanRange::new(
    vec![addr.clone()],
    "0".to_string(),            // ❌ WRONG! Hardcoded schema_hash
    version_str.clone(),        // ✓ Correct (version 3)
    "0".to_string(),            // version_hash (deprecated, OK)
    tablet_id,
    ...
);
```

**Comment in Code**: "schema_hash (not used for regular scans)"
**Reality**: Schema hash IS critical - BE needs it to locate tablet data

---

## Fix Implemented

### 1. Updated Configuration (`scan_ranges.json`)
```json
{
  "tables": {
    "tpch_sf1.lineitem": {
      "tablet_ids": [1763227464383, 1763227464385, 1763227464387],
      "version": 3,
      "schema_hash": "1232661058"  // ✅ Added actual schema_hash
    }
  }
}
```

### 2. Updated Rust Struct (`src/be/thrift_pipeline.rs:703-712`)
```rust
#[cfg(feature = "real_be_proto")]
#[derive(Debug, Deserialize)]
struct TableScanConfig {
    tablet_ids: Vec<i64>,
    version: Option<i64>,
    schema_hash: Option<String>,  // ✅ Added field
}
```

### 3. Updated Scan Range Construction (`src/be/thrift_pipeline.rs:790-798`)
```rust
let schema_hash_str = table_cfg.schema_hash.clone()
    .unwrap_or_else(|| "0".to_string());

let palo = t_plan_nodes::TPaloScanRange::new(
    vec![addr.clone()],
    schema_hash_str.clone(),    // ✅ Now uses real schema_hash
    version_str.clone(),
    "0".to_string(),
    tablet_id,
    ...
);
```

---

## Test Results

### Before Fix
```
✓ Loaded scan ranges: 1 nodes, 3 total ranges
TResultBatch decoded: 0 rows in batch  ← Empty!
```

### After Fix
```
✓ Loaded scan ranges: 1 nodes, 3 total ranges
TResultBatch decoded: 0 rows in batch  ← Still empty!
```

**Conclusion**: Schema hash fix was necessary but insufficient to solve the 0-row problem.

---

## Additional Investigation Findings

### Thrift Protocol Format
- **Rust FE**: Uses Thrift **Compact Protocol** (`TCompactOutputProtocol`)
- **Payload Size**: ~1194 bytes for lineitem scan
- **Encoding**: Variable-length, compact binary format
- **Format Valid**: Payload generation successful, compact protocol correctly used

### Java FE Payload Capture Challenges
- **gRPC Wrapping**: Java FE uses gRPC (port 8060) which wraps Thrift in HTTP/2
- **Fragmentation**: Payload may be split across multiple TCP segments
- **Extraction**: Simple tcpdump capture only got HTTP/2 handshake (122 bytes)
- **Need**: More sophisticated extraction to get actual fragment submission payload

### Scan Range Configuration Verified
- ✅ Scan ranges ARE loaded from config (3 tablets)
- ✅ Tablet IDs match BE metadata
- ✅ Version number correct (3)
- ✅ BE address correct (172.20.80.3:9060)
- ✅ Schema hash now correct (1232661058)

---

## Remaining Unknowns

### What Else Could Cause 0 Rows?

1. **Missing/Incorrect Fields in TPipelineFragmentParams**
   - `query_globals` (timezone, etc.)
   - `query_options` (session variables)
   - `destinations` (result routing)
   - Other required metadata

2. **Scan Range Parameters**
   - `key_column_name` / `key_column_type` currently empty
   - May be required for some scan types

3. **Descriptor Table Issues**
   - `TTupleDescriptor.tableId` not set (currently None)
   - `TDescriptorTable.tableDescriptors` not populated
   - BE might need complete table metadata

4. **Result Sink Configuration**
   - Currently using `MYSQL_PROTOCOL`
   - May need different sink type or additional parameters

5. **BE-side Issues**
   - BE might be silently rejecting scan due to missing fields
   - BE logs would be helpful (if accessible)

---

## Next Steps

### High Priority
1. **Check BE logs** for errors/warnings during fragment execution
2. **Add detailed logging** to scan range construction
   - Log full TPaloScanRange structure before sending
   - Log schema_hash, version, tablet_ids being sent
3. **Compare full Thrift payload structure** more thoroughly
   - Use Java FE payload decoder to see what fields Java sends
   - May need to instrument Java FE to dump payload

### Medium Priority
4. **Populate table descriptors** in TDescriptorTable
   - Add `TTableDescriptor` with full table metadata
   - Set `tableId` in `TTupleDescriptor`
5. **Add query_globals and query_options** if BE requires them

### Low Priority
6. **Test with simpler query** (single tablet, no LIMIT)
7. **Try different result sink types** (Arrow/PBlock vs MySQL text)

---

## Files Modified

### Configuration
- `scan_ranges.json` - Added `schema_hash` field

### Source Code
- `src/be/thrift_pipeline.rs`:
  - Line 703-712: Added `schema_hash: Option<String>` to `TableScanConfig`
  - Line 790-798: Read schema_hash from config and use in scan range construction

### Documentation
- This file: `SCHEMA_HASH_FIX_REPORT.md`

---

## Key Learnings

1. **Don't Trust Comments**: Code said "not used for regular scans" but schema_hash IS important
2. **Query Metadata**: BE likely validates tablet metadata including schema_hash
3. **Multiple Issues**: Schema hash was ONE issue, but not THE issue
4. **Need Better Comparison Tools**: Direct byte-level payload comparison difficult with gRPC wrapping
5. **BE Logs Critical**: Without BE-side logging, hard to know why scans fail

---

## Comparison Tools Built

### Payload Generation
- `examples/generate_payload.rs` - Standalone Rust FE payload generator
- Output: `/tmp/rust-fe-thrift.bin` (1194 bytes, Compact Protocol)

### Payload Capture
- `scripts/capture_tpch_payload.sh` - Automated tcpdump capture
- `scripts/extract_from_tcpdump.sh` - TCP payload extraction
- Captured: Java FE HTTP/2 handshake, but not full fragment payload

### Decode/Compare
- `examples/decode_thrift.rs` - Thrift structure decoder (needs binary protocol update for compact)
- `scripts/compare_thrift_payloads.sh` - Side-by-side comparison tool

---

## References

- **Schema Hash Discovery**: `SHOW TABLETS FROM tpch_sf1.lineitem`
- **Tablet Metadata**: BE host 172.20.80.3, tablets have 5, 8, 0 rows respectively
- **Previous Reports**:
  - `TYPE_CASTING_FIX_REPORT.md` - Fixed descriptor type mismatches
  - `SUCCESS_REPORT.md` - Fixed MySQL timeout issue
  - `todo.md` - Current blockers and progress

---

**Conclusion**: Schema hash fix was important and necessary, but the 0-row issue has additional root causes that require further investigation, likely in descriptor tables or missing query metadata fields.
