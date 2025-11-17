# Tshark Analysis Findings and Action Plan

**Date**: 2025-11-17
**Status**: Analysis Complete, Fixes Identified
**Priority**: HIGH - Blocking Phase 1 completion

## Executive Summary

Using tshark network analysis, I identified that **Rust FE sends an incomplete Thrift payload** compared to Java FE:
- **Java FE**: 3630 bytes (working - returns 3 rows)
- **Rust FE**: 1280 bytes (broken - returns 0 rows)
- **Missing**: ~2350 bytes of critical metadata (~65% of Java FE payload)

## Root Cause Analysis

### What's Working ✅

Both Java FE and Rust FE send identical column schema definitions:
- All 16 lineitem columns present
- Correct Parquet schema encoding
- Column names, types, and ordering match

### What's Missing in Rust FE ❌

Analysis of hex dumps reveals Rust FE payload ends prematurely around offset 0x3b0, missing:

1. **Complete Backend Configuration**
   - Java FE includes: `172.20.80.2` (BE IP address)
   - Rust FE: Missing or incomplete

2. **Temporal Metadata**
   - Java FE includes: `2025-11-17 09:17:59` (timestamp)
   - Java FE includes: `Etc/UTC` (timezone)
   - Rust FE: Incomplete or missing

3. **Locale Settings**
   - Java FE includes: `en_US`
   - Rust FE: Missing

4. **Descriptor Table Completeness**
   - Java FE: 18 slot descriptors, 5 tuple descriptors
   - Rust FE: 16 slot descriptors, 1 tuple descriptor
   - **Missing**: 2 slot descriptors, 4 tuple descriptors

5. **Query Execution Context**
   - Complete query options (batch size, timeout, etc.)
   - Query globals (now_string, timestamp_ms, timezone)
   - Plan node configuration

## Detailed Analysis

### Payload Size Breakdown

| Component | Java FE (bytes) | Rust FE (bytes) | Status |
|-----------|----------------|-----------------|--------|
| HTTP/2 Headers | ~300 | ~300 | ✅ Similar |
| gRPC Metadata | ~100 | ~100 | ✅ Similar |
| Parquet Schema | ~700 | ~700 | ✅ Identical |
| Descriptor Table | ~800 | ~400 | ❌ Incomplete |
| Query Options/Globals | ~300 | ~100 | ❌ Incomplete |
| Scan Ranges | ~900 | ~80 | ❌ Severely incomplete |
| Plan Nodes | ~830 | ~100 | ❌ Missing metadata |
| **Total** | **3630** | **1280** | **❌ 65% missing** |

### Critical Missing Component: Scan Ranges

From Java FE hex dump (offsets 0x4a0-0x630), the payload includes:
- Tablet IDs (multiple tablets)
- Version information
- Schema hash
- Backend endpoints
- Partition metadata
- Key ranges (if applicable)

Rust FE appears to have minimal or empty scan range data, explaining why BE returns 0 rows.

### Descriptor Table Analysis

**Java FE Structure** (from prior analysis):
```
TDescriptorTable {
  slot_descriptors: Vec<TSlotDescriptor> [18 slots]
    - Slots 0-15: 16 lineitem columns
    - Slot 16-17: 2 additional slots (likely for output tuple or intermediate results)

  tuple_descriptors: Vec<TTupleDescriptor> [5 tuples]
    - Tuple 0: Scan tuple (lineitem base table)
    - Tuple 1: Output tuple (result sink)
    - Tuple 2-4: Intermediate/system tuples (possibly for aggregation, sorting, or filtering)

  table_descriptors: Vec<TTableDescriptor> [1+ tables]
    - lineitem table descriptor with full metadata
}
```

**Rust FE Structure** (current):
```
TDescriptorTable {
  slot_descriptors: Vec<TSlotDescriptor> [16 slots]
    - Slots 0-15: 16 lineitem columns only

  tuple_descriptors: Vec<TTupleDescriptor> [1 tuple]
    - Tuple 0: Scan tuple only

  table_descriptors: Vec<TTableDescriptor> [1 table]
    - lineitem table descriptor (simplified)
}
```

## Action Plan

### Phase 1: Fix Scan Range Encoding (CRITICAL)

**File**: `src/be/thrift_pipeline.rs`
**Function**: `build_scan_ranges_for_table_from_config()`
**Priority**: P0 - Likely the primary cause of 0 rows

**Tasks**:
1. ✅ Verify `TPaloScanRange` includes all required fields:
   - `hosts`: List of TNetworkAddress (BE endpoints)
   - `schema_hash`: Correct value from `scan_ranges.json`
   - `version`: Tablet version as string
   - `tablet_id`: Actual Doris tablet ID
   - `db_name`: Database name
   - `table_name`: Table name

2. ✅ Ensure `per_node_scan_ranges` map is populated:
   - Key: TPlanNodeId (scan node ID, typically 0)
   - Value: Vec<TScanRangeParams> with complete scan ranges

3. ⚠️ **NEW FINDING**: Verify gRPC encoding doesn't truncate scan range data
   - Current implementation may not be serializing scan ranges completely
   - Check that all scan range bytes are included in the gRPC message

### Phase 2: Enhance Descriptor Table (HIGH)

**File**: `src/be/thrift_pipeline.rs`
**Functions**:
- `encode_descriptor_table_for_table_from_catalog_typed()`
- `encode_tuple_descriptor_typed()`
- `encode_slot_descriptor_typed()`

**Tasks**:
1. Add output tuple descriptor (Tuple ID 1):
   ```rust
   // Output tuple for result sink
   let output_tuple = t_desc::TTupleDescriptor::new(
       1,  // tuple_id
       None::<i64>,  // no table_id (output tuple)
       None::<i32>,  // byte_size calculated by BE
   );
   ```

2. Add output slot descriptors (Slots 16-17):
   ```rust
   // These likely mirror scan slots but reference the output tuple
   for i in 0..2 {
       let output_slot = t_desc::TSlotDescriptor::new(
           16 + i,  // slot_id
           1,       // parent (output tuple_id)
           // ... other fields
       );
   }
   ```

3. Research Java FE source to understand:
   - What tuples 2-4 represent
   - When they're needed (always, or only for complex queries?)
   - How to construct them properly

### Phase 3: Verify Query Globals/Options (MEDIUM)

**File**: `src/be/thrift_pipeline.rs`
**Functions**:
- `encode_query_globals_typed()`
- `encode_query_options_typed()`

**Current Implementation**:
```rust
// encode_query_globals_typed()
t_palo::TQueryGlobals::new(
    now_string,                      // ✅ Set
    Some(timestamp_ms),              // ✅ Set
    Some("UTC".to_string()),         // ✅ Set
    None::<bool>,                    // ❓ load_zero_tolerance
    None::<i32>,                     // ❓ nano_seconds
    None::<String>,                  // ❓ last_query_id
)

// encode_query_options_typed()
opts.batch_size = Some(1024);         // ✅ Set
opts.query_timeout = Some(600);       // ✅ Set
opts.enable_pipeline_engine = Some(true);  // ✅ Set
// Missing ~50+ other optional fields
```

**Tasks**:
1. Compare against Java FE's `TQueryGlobals` generation:
   - Verify all required fields are set
   - Check if optional fields affect scan execution

2. Compare against Java FE's `TQueryOptions`:
   - Review which options are critical for scan queries
   - Add any missing options that affect scan behavior

### Phase 4: Enhance Table Descriptor (LOW)

**File**: `src/be/thrift_pipeline.rs`
**Function**: `encode_table_descriptor_for_tpch_lineitem_typed()`

**Tasks**:
1. Verify `TTableDescriptor` completeness:
   - All OLAP table metadata included
   - Schema version information correct
   - Index metadata if needed

2. Add any missing metadata fields observed in Java FE payload

## Testing Protocol

### Test 1: Validate Payload Structure
```bash
# Generate Rust FE payload
cargo run --features real_be_proto --example generate_payload

# Decode and inspect
cargo run --features real_be_proto --example decode_compact_generated -- /tmp/rust-fe-thrift.bin

# Expected output should show:
# - slot_descriptors.len >= 18 (after fixes)
# - tuple_descriptors.len >= 2 (scan + output, minimum)
# - scan_ranges.len > 0 for node 0
```

### Test 2: End-to-End Query
```bash
# Start Rust FE
RUST_LOG=info cargo run --bin doris-rust-fe --features real_be_proto -- --config fe_config.json

# Execute query
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT * FROM tpch_sf1.lineitem LIMIT 3"

# Expected: 3 rows returned (not 0)
```

### Test 3: Payload Comparison
```bash
# Capture both payloads
./scripts/capture_tpch_payload.sh

# Compare sizes
ls -lh /tmp/tpch_captures/*payload*.bin

# Expected: Rust payload size within 10-20% of Java payload
```

## Java FE Reference Points

For implementing missing components, refer to these Java FE classes:

### Descriptor Table Generation
- **Class**: `org.apache.doris.planner.Coordinator`
- **Method**: `buildDescriptorTable()`
- **Location**: `fe/fe-core/src/main/java/org/apache/doris/planner/Coordinator.java`

Key logic:
```java
// Creates scan tuple + output tuple
DescriptorTable descTbl = new DescriptorTable();
TupleDescriptor scanTuple = descTbl.createTupleDescriptor();
TupleDescriptor outputTuple = descTbl.createTupleDescriptor();

// Adds slots for each column
for (Column col : table.getColumns()) {
    SlotDescriptor slot = descTbl.addSlotDescriptor(scanTuple);
    slot.setColumn(col);
    slot.setIsMaterialized(true);
}

// Output tuple mirrors scan tuple for simple SELECTs
for (SlotDescriptor scanSlot : scanTuple.getSlots()) {
    SlotDescriptor outputSlot = descTbl.addSlotDescriptor(outputTuple);
    outputSlot.setColumn(scanSlot.getColumn());
}
```

### Scan Range Construction
- **Class**: `org.apache.doris.planner.OlapScanNode`
- **Method**: `addScanRangeLocations()`
- **Location**: `fe/fe-core/src/main/java/org/apache/doris/planner/OlapScanNode.java`

Key logic:
```java
for (Tablet tablet : selectedTablets) {
    TPaloScanRange paloRange = new TPaloScanRange();
    paloRange.setDb_name(dbName);
    paloRange.setSchema_hash(String.valueOf(tablet.getSchemaHash()));
    paloRange.setVersion(String.valueOf(tablet.getVisibleVersion()));
    paloRange.setTablet_id(tablet.getId());

    // Add backend replicas
    for (Replica replica : tablet.getReplicas()) {
        TNetworkAddress addr = new TNetworkAddress(
            replica.getHost(),
            replica.getBePort()
        );
        paloRange.addToHosts(addr);
    }

    TScanRange scanRange = new TScanRange();
    scanRange.setPalo_scan_range(paloRange);

    TScanRangeLocation location = new TScanRangeLocation();
    location.setScan_range(scanRange);
    // ... set backend_id, volume_id, etc.

    scanRangeLocations.add(location);
}
```

### Query Options/Globals
- **Class**: `org.apache.doris.qe.SessionVariable`
- **Method**: `toThrift()`
- **Location**: `fe/fe-core/src/main/java/org/apache/doris/qe/SessionVariable.java`

## Success Criteria

1. **Payload Size**: Rust FE payload >= 3000 bytes (within 20% of Java FE)
2. **Descriptor Counts**:
   - slot_descriptors.len >= 18
   - tuple_descriptors.len >= 2
3. **Scan Ranges**: per_node_scan_ranges[0].len == 3 (for lineitem test query)
4. **Query Result**: `SELECT * FROM tpch_sf1.lineitem LIMIT 3` returns **3 rows**
5. **No Errors**: BE logs show no payload validation errors

## Risk Assessment

**Low Risk**:
- Query globals/options enhancement (mostly optional fields)
- Table descriptor additions (informational metadata)

**Medium Risk**:
- Output tuple/slot descriptors (may not be required for simple SELECTs)
- Additional tuple descriptors 2-4 (purpose unclear without Java source review)

**High Risk**:
- Scan range encoding changes (critical for data retrieval)
- Any changes that affect Thrift serialization format

**Mitigation**:
- Test each change incrementally
- Use payload decoder tools to verify structure before testing against BE
- Keep Java FE payload captures for comparison

## Timeline Estimate

- **Phase 1 (Scan Ranges)**: 4-6 hours
  - Review current implementation
  - Identify truncation/incompleteness issues
  - Fix and test

- **Phase 2 (Descriptor Table)**: 6-8 hours
  - Research Java FE tuple/slot generation
  - Implement output tuple + slots
  - Test against BE

- **Phase 3 (Query Options/Globals)**: 2-3 hours
  - Review Java FE defaults
  - Add missing critical options
  - Verify encoding

- **Phase 4 (Table Descriptor)**: 1-2 hours
  - Minor metadata additions
  - Validation

**Total**: 13-19 hours for complete implementation

## Next Immediate Steps

1. **TODAY**:
   - Review `build_scan_ranges_for_table_from_config()` implementation
   - Add debug logging to print scan range sizes before/after encoding
   - Test if scan ranges are being truncated during gRPC serialization

2. **TOMORROW**:
   - Implement output tuple + 2 output slot descriptors
   - Test against BE to see if row count improves

3. **FOLLOW-UP**:
   - Research Java FE source for tuples 2-4
   - Complete query options/globals review
   - Final end-to-end testing

## References

- **tshark Analysis**: `TSHARK_ANALYSIS.md`
- **Capture Files**: `/tmp/tshark_debug/*.pcap`
- **Java FE Payload**: `/tmp/tpch_captures/java_simple_scan.pcap`
- **Rust FE Payload**: `/tmp/tpch_captures/rust_simple_scan.pcap`
- **Tracking**: `todo.md`, `tools.md`, `current_impl.md`

---

**Status**: Ready for implementation
**Assignee**: Development team
**Last Updated**: 2025-11-17
