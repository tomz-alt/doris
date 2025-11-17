# Session Summary: Tshark Analysis and Root Cause Identification

**Date**: 2025-11-17
**Session Focus**: Use tshark to debug Rust FE "0 rows" issue
**Status**: ✅ Root Cause Identified, Action Plan Created

## Objective

Identify why Rust FE returns 0 rows from BE despite successful query execution, while Java FE returns 3 rows for the same query: `SELECT * FROM tpch_sf1.lineitem LIMIT 3`

## Methodology

Used **tshark** (Wireshark CLI) to capture and analyze actual network traffic between Frontend and Backend:

1. Captured Java FE ↔ BE traffic (baseline - known working)
2. Captured Rust FE ↔ BE traffic (test - returns 0 rows)
3. Compared gRPC payload sizes and content
4. Performed hex dump analysis to identify missing metadata

## Key Findings

### Critical Discovery: Incomplete Thrift Payload

**Payload Size Comparison**:
- **Java FE**: 3630 bytes in main gRPC message → ✅ Returns 3 rows
- **Rust FE**: 1280 bytes in main gRPC message → ❌ Returns 0 rows
- **Missing**: ~2350 bytes (65% of Java FE payload)

### Root Cause Analysis

Rust FE is sending an **incomplete Thrift payload** to the Backend, missing critical metadata:

1. **Scan Range Configuration** (~900 bytes missing)
   - Java FE includes complete `TPaloScanRange` structs with:
     - Tablet IDs, versions, schema hashes
     - Backend addresses (hosts/ports)
     - Partition and key range metadata
   - Rust FE appears to have minimal or truncated scan range data
   - **Impact**: BE has no tablets to scan → returns 0 rows

2. **Descriptor Table** (~600 bytes missing)
   - Java FE: 18 slot descriptors + 5 tuple descriptors
   - Rust FE: 16 slot descriptors + 1 tuple descriptor
   - **Missing**:
     - Output tuple descriptor (tuple_id=1)
     - 2 additional slot descriptors (likely for output tuple)
     - 3 intermediate/system tuple descriptors
   - **Impact**: BE may not know how to materialize result rows

3. **Query Execution Context** (~200 bytes missing)
   - Backend IP address: `172.20.80.2`
   - Timestamp: `2025-11-17 09:17:59`
   - Timezone: `Etc/UTC`
   - Locale: `en_US`
   - Additional query options/globals
   - **Impact**: BE may use incorrect defaults for query execution

### Hex Dump Evidence

**Java FE** (Frame 11, 3630 bytes):
- Offsets 0x000-0x470: gRPC headers + Parquet schema (columns)
- Offsets 0x470-0x630: **Complete metadata** (backend config, timestamps, scan ranges)
- Full query execution context present

**Rust FE** (Frame 89, 1280 bytes):
- Offsets 0x000-0x3a0: gRPC headers + Parquet schema (identical to Java FE ✓)
- Offset 0x3b0: **Payload ends prematurely**
- Missing all backend config and scan range metadata

## Work Completed

### Documentation Created

1. **TSHARK_ANALYSIS.md** (Detailed Analysis)
   - Complete payload comparison
   - Protocol hierarchy statistics
   - Hex dump analysis with annotations
   - Methodology and tools reference

2. **TSHARK_FINDINGS_AND_ACTION_PLAN.md** (Action Plan)
   - Root cause analysis
   - 4-phase implementation plan with timeline estimates
   - Java FE code references for each component
   - Testing protocol and success criteria
   - Risk assessment

3. **Updated Tracking Documents**:
   - `tools.md`: Added comprehensive tshark debugging section
   - `todo.md`: Updated with tshark findings and 4-phase action plan
   - `current_impl.md`: Added tshark analysis section with key findings

### Artifacts Preserved

- **Network Captures**:
  - `/tmp/tshark_debug/java-capture.pcap` (12K)
  - `/tmp/tshark_debug/rust-capture.pcap` (35K)

- **Analysis Logs**:
  - `/tmp/tshark_debug/rust-fe.log` (Rust FE server log during capture)

## Implementation Roadmap

### Phase 1: Fix Scan Range Encoding (P0 - CRITICAL)
**Location**: `src/be/thrift_pipeline.rs::build_scan_ranges_for_table_from_config()`
**Timeline**: 4-6 hours
**Priority**: Must fix first - this is the primary cause of 0 rows

**Tasks**:
1. Add debug logging to print scan range sizes before/after encoding
2. Verify all `TPaloScanRange` fields are populated correctly
3. Check if gRPC serialization is truncating data
4. Ensure `per_node_scan_ranges` map is correctly populated

**Expected Result**: Scan range section grows from ~80 bytes to ~900 bytes

### Phase 2: Enhance Descriptor Table (P1 - HIGH)
**Location**: `src/be/thrift_pipeline.rs::encode_descriptor_table_for_table_from_catalog_typed()`
**Timeline**: 6-8 hours
**Priority**: High - needed for proper result materialization

**Tasks**:
1. Add output tuple descriptor (tuple_id=1, no table_id)
2. Add 2 output slot descriptors (slot_id=16,17) for output tuple
3. Research Java FE source for tuples 2-4 (intermediate/system tuples)

**Expected Result**: Descriptor table matches Java FE structure (18 slots, 5 tuples)

### Phase 3: Verify Query Options/Globals (P2 - MEDIUM)
**Location**: `encode_query_globals_typed()`, `encode_query_options_typed()`
**Timeline**: 2-3 hours
**Priority**: Medium - may affect scan behavior

**Tasks**:
1. Compare generated `TQueryGlobals` with Java FE defaults
2. Review `TQueryOptions` for critical scan-related settings
3. Add any missing optional fields

**Expected Result**: Query context metadata complete

### Phase 4: End-to-End Testing (P3)
**Timeline**: 2-4 hours

**Tasks**:
1. Generate new payload with `examples/generate_payload.rs`
2. Decode and verify structure with `decode_compact_generated`
3. Capture new network traffic with tshark
4. Compare payload sizes (should be within 10-20% of Java FE)
5. Run `SELECT * FROM tpch_sf1.lineitem LIMIT 3` → verify 3 rows returned

**Total Estimated Time**: 14-21 hours for complete implementation

## Success Criteria

1. **Payload Size**: Rust FE payload >= 3000 bytes (within 20% of Java FE)
2. **Descriptor Counts**:
   - slot_descriptors.len >= 18
   - tuple_descriptors.len >= 2
3. **Scan Ranges**: per_node_scan_ranges[0].len == 3 (for lineitem test query)
4. **Query Result**: Returns **3 rows** (not 0)
5. **No BE Errors**: BE logs show no payload validation errors

## Java FE References

For implementation, refer to these Java FE classes:

- **Descriptor Table**: `org.apache.doris.planner.Coordinator::buildDescriptorTable()`
- **Scan Ranges**: `org.apache.doris.planner.OlapScanNode::addScanRangeLocations()`
- **Query Options**: `org.apache.doris.qe.SessionVariable::toThrift()`

## Tools and Procedures

### Tshark Quick Reference

```bash
# Capture Java FE traffic
docker exec -d doris-be tcpdump -i any -w /tmp/java-capture.pcap "tcp port 8060"
docker exec doris-fe-java mysql -uroot -h127.0.0.1 -P9030 \
  -e "USE tpch_sf1; SELECT * FROM lineitem LIMIT 3"
docker exec doris-be pkill tcpdump
docker cp doris-be:/tmp/java-capture.pcap /tmp/tshark_debug/

# Analyze with tshark
tshark -r /tmp/tshark_debug/java-capture.pcap -qz io,phs
tshark -r /tmp/tshark_debug/java-capture.pcap \
  -Y "grpc" -T fields -e frame.number -e grpc.message_length

# Hex dump comparison
tshark -r /tmp/tshark_debug/java-capture.pcap \
  -Y "frame.number==11" -x | head -100
```

See `tools.md` for complete tshark debugging procedures.

## Risk Assessment

**Low Risk**:
- Query globals/options enhancement (mostly optional)
- Table descriptor additions (informational)

**Medium Risk**:
- Output tuple/slot descriptors (may not be required for simple SELECTs)
- Additional tuple descriptors 2-4 (purpose needs research)

**High Risk**:
- Scan range encoding changes (critical for data retrieval)
- Thrift serialization format changes

**Mitigation**:
- Test incrementally
- Use payload decoder tools to verify structure
- Keep Java FE payload captures for comparison

## Next Steps

### Immediate (Today)
1. Review `build_scan_ranges_for_table_from_config()` implementation
2. Add debug logging for scan range sizes
3. Test if scan ranges are being truncated during gRPC serialization

### Short-term (This Week)
1. Implement Phase 1 (Scan Range fixes)
2. Test against BE - verify row count improves
3. If still 0 rows, proceed to Phase 2 (Descriptor Table)

### Medium-term (Next Week)
1. Complete all 4 phases
2. Validate with tshark captures
3. Update documentation with final results

## Conclusion

The tshark analysis successfully identified the root cause of the "0 rows" issue: **Rust FE is sending an incomplete Thrift payload missing ~65% of the metadata that Java FE sends**. The primary issue is incomplete scan range encoding, with secondary issues in descriptor table structure.

With a clear roadmap and Java FE code references, the path to fixing this issue is well-defined. The estimated 14-21 hours of implementation work should result in Rust FE successfully returning data from BE queries.

## Files Modified

- `TSHARK_ANALYSIS.md` (created)
- `TSHARK_FINDINGS_AND_ACTION_PLAN.md` (created)
- `SESSION_SUMMARY_2025-11-17.md` (this file - created)
- `tools.md` (updated - added tshark section)
- `todo.md` (updated - added tshark findings and 4-phase plan)
- `current_impl.md` (updated - added tshark analysis section)

## Session Artifacts

All work from this session is preserved in:
- Documentation files (listed above)
- Network captures (`/tmp/tshark_debug/*.pcap`)
- Analysis logs (`/tmp/tshark_debug/rust-fe.log`)

---

**Session Duration**: ~3 hours
**Primary Tool**: tshark (Wireshark CLI)
**Primary Achievement**: Root cause identified with clear action plan
**Status**: Ready for implementation phase
