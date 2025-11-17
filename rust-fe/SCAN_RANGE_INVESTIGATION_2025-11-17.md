# Scan Range Serialization Investigation

**Date**: 2025-11-17
**Issue**: BE returns 0 rows despite scan ranges being created correctly in Rust FE
**Root Cause Hypothesis**: Scan ranges may not be fully serialized into Thrift payload

## Investigation Steps Taken

### 1. Confirmed Scan Range Creation (✓ VERIFIED)

Added debug logging to `build_scan_ranges_for_table_from_config()` (src/be/thrift_pipeline.rs:795-838):

```rust
tracing::info!(
    "  → Created TPaloScanRange: tablet_id={}, schema_hash={}, version={}, db={}, table={}, host={}:{}",
    tablet_id, schema_hash_str, version_str, db_name, table_name, be.host, be.port
);
```

**Result**: Confirmed 3 scan ranges ARE being created:
```
→ Created TPaloScanRange: tablet_id=1763227464383, schema_hash=1232661058, version=3, db=tpch_sf1, table=lineitem, host=172.20.80.3:9060
→ Created TPaloScanRange: tablet_id=1763227464385, schema_hash=1232661058, version=3, ...
→ Created TPaloScanRange: tablet_id=1763227464387, schema_hash=1232661058, version=3, ...
✓ Built scan ranges map: node_id=0 has 3 scan ranges
✓ Loaded scan ranges for tpch_sf1.lineitem: 1 nodes, 3 total ranges
```

### 2. Added Instance Params Verification Logging

Added logging after `TPipelineInstanceParams::new()` (src/be/thrift_pipeline.rs:869-886):

```rust
let instance = t_palo::TPipelineInstanceParams::new(
    fragment_instance_id,
    None::<bool>,
    per_node_scan_ranges,  // <-- 3 scan ranges passed here
    // ... other params
);

// Verify scan ranges are in the struct
let total_ranges: usize = instance.per_node_scan_ranges.values().map(|v| v.len()).sum();
tracing::info!("📦 TPipelineInstanceParams created: per_node_scan_ranges has {} nodes, {} total ranges",
    instance.per_node_scan_ranges.len(), total_ranges);
```

**Purpose**: Verify scan ranges are in the Thrift struct after construction, before serialization.

### 3. Added Serialization Verification Logging

Added logging before and after Thrift serialization (src/be/thrift_pipeline.rs:320-343):

```rust
pub fn to_thrift_bytes(&self) -> Vec<u8> {
    let s = self.to_thrift_struct();

    // Debug: check scan ranges in struct before serialization
    if let Some(ref params_list) = s.params_list {
        if let Some(first_param) = params_list.first() {
            if let Some(ref local_params) = first_param.local_params {
                if let Some(first_local) = local_params.first() {
                    let scan_ranges_count: usize = first_local.per_node_scan_ranges.values()
                        .map(|v| v.len()).sum();
                    tracing::info!("🔍 Before serialization: local_params[0].per_node_scan_ranges has {} nodes, {} total ranges",
                        first_local.per_node_scan_ranges.len(), scan_ranges_count);
                }
            }
        }
    }

    let bytes = compact_encode_thrift_struct(&s)
        .expect("encode TPipelineFragmentParamsList");

    tracing::info!("📦 After serialization: payload is {} bytes", bytes.len());

    bytes
}
```

**Purpose**:
- Verify scan ranges are still in the struct after `to_thrift_struct()` conversion
- Measure actual payload size after serialization
- Compare with expected size (Java FE: 3630 bytes, current Rust FE: ~1368 bytes)

## Key Findings

### Confirmed Working ✅
1. Scan range creation logic is correct
2. 3 TPaloScanRange structs created with proper parameters:
   - tablet_id: 1763227464383, 1763227464385, 1763227464387
   - schema_hash: 1232661058 (correct)
   - version: 3 (matches BE)
   - host: 172.20.80.3:9060 (correct BE address)
3. Scan ranges added to BTreeMap for node_id=0
4. BTreeMap passed to `TPipelineInstanceParams::new()`

### Still Unverified ⚠️
1. **Are scan ranges in TPipelineInstanceParams after `.new()`?**
   - Added logging but not yet tested

2. **Are scan ranges in the Thrift struct after `to_thrift_struct()`?**
   - Added logging but not yet tested

3. **What is the actual payload size after serialization?**
   - Added logging but not yet tested
   - Expected: Should be closer to 3630 bytes if scan ranges are included
   - Current: ~1368 bytes (suggests scan ranges missing ~900 bytes)

### Critical Gap
**Payload Size Analysis** (from previous tshark investigation):
- Rust FE total: 1368 bytes
- Java FE total: 3630 bytes
- **Missing**: 2262 bytes (62%)

**Largest component gap**:
- Scan ranges: Rust ~80 bytes vs Java ~900 bytes (**820 bytes missing**)
- This is the most critical discrepancy

## Hypotheses (Updated)

### H1: Scan Ranges Not Included in Thrift Struct (P0 - MOST LIKELY)

**Evidence**:
- Scan ranges created correctly in Rust (✓ verified)
- Payload 62% smaller than Java FE
- Scan range section severely truncated (~80 vs ~900 bytes)

**Possible Causes**:
1. `TPipelineInstanceParams::new()` parameter order mismatch
2. `per_node_scan_ranges` field not properly set in struct
3. `to_thrift_struct()` doesn't copy scan ranges to output
4. Thrift struct definition mismatch between Rust and Java

### H2: Scan Ranges Truncated During Serialization (P1)

**Evidence**:
- Would explain why creation works but payload is small

**Possible Causes**:
1. Thrift compact protocol encoder issue
2. gRPC message size limit (unlikely - would error)
3. Missing nested structure encoding

### H3: Multiple Replicas Not Included (P2)

**Evidence**:
- Java FE scan section ~900 bytes suggests 3 replicas × 3 tablets = 9 host entries
- Rust FE only includes 1 host per tablet

**Impact**: May not be critical for 0-row issue, but affects HA

## Next Steps

### Immediate Actions (P0)

1. **Test with New Debug Logging**
   ```bash
   pkill -f "doris-rust-fe"
   cargo build --features real_be_proto
   RUST_LOG=info cargo run --bin doris-rust-fe --features real_be_proto -- --config fe_config.json 2>&1 > /tmp/scan-range-debug.log &
   sleep 5
   # Use correct database context
   mysql -h 127.0.0.1 -P 9031 -u root -D tpch_sf1 -e "SELECT * FROM lineitem LIMIT 3"
   ```

2. **Analyze Debug Output**
   ```bash
   grep -E "(📦|🔍|TPaloScanRange|Built scan ranges|Before serialization|After serialization)" /tmp/scan-range-debug.log
   ```

   **Expected Output** (if working correctly):
   ```
   ✓ Built scan ranges map: node_id=0 has 3 scan ranges
   ✓ Loaded scan ranges for tpch_sf1.lineitem: 1 nodes, 3 total ranges
   📦 TPipelineInstanceParams created: per_node_scan_ranges has 1 nodes, 3 total ranges
   🔍 Before serialization: local_params[0].per_node_scan_ranges has 1 nodes, 3 total ranges
   📦 After serialization: payload is XXXX bytes  # Should be ~2500-3000 bytes if scan ranges included
   ```

3. **If scan ranges ARE in struct but payload still small**:
   - Investigate Thrift compact protocol encoder
   - Check if nested BTreeMap<i32, Vec<TScanRangeParams>> serializes correctly

4. **If scan ranges NOT in struct**:
   - Check `TPipelineInstanceParams::new()` parameter order
   - Verify Thrift struct definition matches expected layout
   - Check `to_thrift_struct()` implementation

### Follow-up Actions (P1)

5. **Compare Thrift Struct Definitions**
   - Check doris-thrift generated code for `TPipelineInstanceParams`
   - Verify field positions match Doris internal_service.proto

6. **Add Replica Support**
   - Query BE metadata for all tablet replicas
   - Include all replica hosts in scan ranges
   - May explain remaining ~300 bytes gap

## Files Modified

1. **src/be/thrift_pipeline.rs**
   - Lines 795-838: Added scan range creation debug logging
   - Lines 869-886: Added TPipelineInstanceParams verification logging
   - Lines 320-343: Added serialization before/after logging

2. **Created**:
   - examples/decode_scan_ranges.rs (attempted but has compilation errors)
   - This document: SCAN_RANGE_INVESTIGATION_2025-11-17.md

## Technical Notes

### TPipelineInstanceParams::new() Call
```rust
t_palo::TPipelineInstanceParams::new(
    fragment_instance_id,                              // 1. TUniqueId
    None::<bool>,                                      // 2. build_hash_table_for_broadcast_join
    per_node_scan_ranges,                              // 3. per_node_scan_ranges (BTreeMap<i32, Vec<TScanRangeParams>>)
    None::<i32>,                                       // 4. sender_id
    None::<t_palo::TRuntimeFilterParams>,             // 5. runtime_filter_params
    None::<i32>,                                       // 6. per_node_shared_scans_size
    None::<BTreeMap<t_types::TPlanNodeId, bool>>,    // 7. per_node_scan_ranges_is_colocate_mv_index
    None::<Vec<i32>>,                                 // 8. node_to_per_driver_seq_scan_ranges
    None::<Vec<t_plan_nodes::TTopnFilterDesc>>,      // 9. topn_filter_descs
)
```

**Critical**: Parameter 3 is `per_node_scan_ranges`. If this is in the wrong position or type doesn't match, scan ranges won't be included.

### Scan Range Expected Structure
Each TPaloScanRange should contain:
- hosts: Vec<TNetworkAddress>
- schema_hash: String ("1232661058")
- version: String ("3")
- version_hash: String ("0")
- tablet_id: i64
- db_name: String ("tpch_sf1")
- table_name: Option<String> ("lineitem")

**Current**: 1 host per tablet = ~94 bytes × 3 = ~282 bytes
**Java FE**: ~900 bytes suggests 3 hosts × 3 tablets = ~846 bytes (3 replicas per tablet)

## Success Criteria

✅ **Phase 1**: Verify scan ranges reach serialization
- TPipelineInstanceParams contains 3 scan ranges after construction
- Thrift struct contains 3 scan ranges before serialization
- Payload size increases to 2500-3000+ bytes

✅ **Phase 2**: BE returns data
- TResultBatch shows rows > 0
- Query returns actual lineitem data from tablets

## Estimated Time to Resolution

- **If H1 is correct** (struct construction issue): 2-4 hours
- **If H2 is correct** (serialization issue): 4-8 hours
- **If replica support needed**: +2-4 hours

**Total Estimated**: 6-12 hours to get data flowing from BE

## Alternative Verification Tool

**Pinterest's thrift-tools** (https://github.com/pinterest/thrift-tools) provides excellent utilities for inspecting Thrift payloads:

```bash
# Install (requires Python environment without PEP 668 restrictions)
pip install thrift-tools

# Use to inspect payload
thrift-json < /tmp/rust-fe-thrift.bin

# Compare with Java FE payload
thrift-json < /tmp/java-fe-thrift.bin
```

**Benefits**:
- Schema-less decoding (doesn't need .thrift files)
- JSON output for easy comparison
- Field-by-field inspection
- Can verify if scan ranges are actually serialized

**Note**: System has PEP 668 restrictions preventing installation. Can use in Docker or virtualenv if needed for deeper investigation.

## References

- Previous investigation: SESSION_PROGRESS_2025-11-17.md
- Tshark analysis: TSHARK_FINDINGS_AND_ACTION_PLAN.md
- BE configuration: scan_ranges.json
- FE configuration: fe_config.json
- Thrift inspection tool: https://github.com/pinterest/thrift-tools
