# Tshark Network Traffic Analysis - Java FE vs Rust FE

**Date**: 2025-11-17
**Objective**: Compare Thrift payloads between Java FE (working - returns 3 rows) and Rust FE (broken - returns 0 rows)

## Summary of Findings

### Payload Size Comparison

| Metric | Java FE | Rust FE | Difference |
|--------|---------|---------|------------|
| Total pcap size | 12K | 35K | Rust FE larger (more TCP overhead) |
| Main gRPC message | **3630 bytes** | **1280 bytes** | **Java FE 2.8x larger** |
| gRPC protobuf total | 6485 bytes | 2869 bytes | Java FE 2.3x larger |

**Critical Finding**: Java FE sends almost **3x more data** (3630 vs 1280 bytes) in the main fragment submission request.

### Traffic Breakdown

**Java FE**:
- Total frames: 69
- HTTP/2 frames: 30 (8535 bytes)
- gRPC frames: 10 (6485 bytes)
- Main exec_plan_fragment request: Frame 11 (3630 bytes)

**Rust FE**:
- Total frames: 126
- HTTP/2 frames: 16 (3426 bytes)
- gRPC frames: 10 (2869 bytes)
- "data" frames: 64 (26625 bytes) - unclassified TCP data
- Main exec_plan_fragment request: Frame 89 (1280 bytes)

### gRPC Message Comparison

**Java FE Messages** (frame.number: grpc.message_length):
```
11:  3630 bytes  ← Main fragment submission (exec_plan_fragment)
13:  25 bytes
14:  24 bytes
15:  25 bytes
26:  26 bytes
28:  384 bytes  ← Likely fetch_data response
59:  55 bytes
61:  394 bytes  ← Likely another fetch_data response
63:  4 bytes
64:  2 bytes
```

**Rust FE Messages** (frame.number: grpc.message_length):
```
89:  1280 bytes  ← Main fragment submission (exec_plan_fragment)
98:  25 bytes
100: 23 bytes
101: 25 bytes
103: 23 bytes
104: 34 bytes
106: 23 bytes
107: 34 bytes
109: 23 bytes
110: 12 bytes
```

## Hex Dump Analysis

### Common Elements (Both Payloads)

Both payloads contain Parquet schema metadata with all 16 lineitem columns:

1. `l_orderkey` (BIGINT)
2. `l_partkey` (BIGINT)
3. `l_suppkey` (BIGINT)
4. `l_linenumber` (INT)
5. `l_quantity` (DECIMAL)
6. `l_extendedprice` (DECIMAL)
7. `l_discount` (DECIMAL)
8. `l_tax` (DECIMAL)
9. `l_returnflag` (VARCHAR)
10. `l_linestatus` (VARCHAR)
11. `l_shipdate` (DATE)
12. `l_commitdate` (DATE)
13. `l_receiptdate` (DATE)
14. `l_shipinstruct` (VARCHAR)
15. `l_shipmode` (VARCHAR)
16. `l_comment` (VARCHAR)

### Key Differences

**Java FE** (Frame 11, offsets shown):
- Contains full Parquet schema (0x100-0x470)
- **Additional metadata after schema** (0x470-0x630+):
  - Tablet IDs and version information
  - Backend address: `172.20.80.2` (in hex: `31 37 32 2e 32 30 2e 38 30 2e 32`)
  - Timestamp: `2025-11-17 09:17:59`
  - Timezone: `Etc/UTC`
  - Locale: `en_US`
  - Query options and globals
  - Plan node configuration
  - **Scan range parameters** (likely including tablet IDs, versions, etc.)

**Rust FE** (Frame 89, offsets shown):
- Contains similar Parquet schema (0x0f0-0x3a0)
- **Ends prematurely** around offset 0x3b0
- Missing:
  - Detailed backend configuration
  - Complete query globals/options
  - Scan range details
  - Additional plan node metadata

## Detailed Hex Comparison

### Java FE - Post-Schema Data (0x470+)
```
0470  28 e7 00 00 45 00 0f 2f e4 b5 40 00 40 06 4e e5   (...E../..@.@.N.
...
04a0  02 12 1c 18 0b 31 37 32 2e 32 30 2e 38 30 2e 32   .....172.20.80.2
04b0  15 f8 8c 01 00 1c 18 13 32 30 32 35 2d 31 31 2d   ........2025-11-
04c0  31 37 20 30 39 3a 31 37 3a 35 39 16 e2 e0 da 91   17 09:17:59.....
04d0  d2 66 18 07 45 74 63 2f 55 54 43 12 15 9c 84 c7   .f..Etc/UTC.....
04e0  b7 05 18 05 65 6e 5f 55 53 00 1c 45 c0 3f 35 00   ....en_US..E.?5.
```

This section contains:
- Backend IP address
- Timestamp
- Timezone info
- Locale
- Query execution metadata

### Rust FE - End of Payload (0x3a0+)
```
03a0  16 85 b5 a0 f7 0e 15 00 00 19 1c 16 85 b5 a0 f7   ................
```

The payload appears to end much sooner, missing the detailed metadata that Java FE includes.

## Analysis and Conclusions

### 1. Missing Data in Rust FE Payload

The Rust FE payload is missing **~2350 bytes** of data compared to Java FE. This missing data likely includes:

- ✅ **Descriptor table metadata** (table IDs, tuple descriptors)
- ✅ **Scan range parameters** (tablet IDs, versions, BE addresses)
- ✅ **Query globals** (timestamp, timezone, locale)
- ✅ **Query options** (batch size, timeout, pipeline engine settings)
- ✅ **Plan node configuration** (scan node parameters)

### 2. Why BE Returns 0 Rows

The BE likely receives an incomplete fragment specification that:
1. Lacks proper scan range configuration
2. Missing tablet metadata
3. Incomplete descriptor table
4. Missing query execution context

As a result, the BE:
- Accepts the fragment (no errors)
- Creates an execution context
- But has **no tablets to scan** → returns 0 rows
- Sends empty `TResultBatch` and EOS

### 3. Root Cause Hypothesis

Based on the tracking files (`todo.md`), the issue is likely in the **descriptor table generation**:

From `todo.md`:
```
- Descriptor table (`TDescriptorTable`) is still minimal compared to Java FE:
  - Rust: `slot_descriptors.len = 16`, `tuple_descriptors.len = 1`
  - Java: `slot_descriptors.len = 18`, `tuple_descriptors.len = 5`
```

The missing 2+ tuple descriptors and 2+ slot descriptors likely correspond to:
- Internal/system tuples
- Intermediate result tuples
- Materialization descriptors
- Output sink descriptors

### 4. Next Steps

1. **Examine descriptor table encoding** in `src/be/thrift_pipeline.rs:encode_descriptor_table_for_table_from_catalog_typed()`
   - Compare against Java FE's `DescriptorTable` generation
   - Ensure all tuple descriptors are created (not just the scan tuple)
   - Add system/internal descriptors if needed

2. **Verify scan range encoding** in `src/be/thrift_pipeline.rs:build_scan_ranges_for_table_from_config()`
   - Ensure `TPaloScanRange` includes all necessary fields
   - Verify tablet IDs, versions, schema_hash are correctly encoded
   - Check that BE host/port are properly set

3. **Review query globals/options** in `src/be/thrift_pipeline.rs:encode_query_globals_typed()` and `encode_query_options_typed()`
   - Ensure timestamp, timezone, locale are set
   - Verify query options (batch_size, timeout, etc.) match Java FE

4. **Use existing payload decoder tools** to verify structure:
   ```bash
   cargo run --example decode_compact_generated -- /tmp/rust-fe-thrift.bin
   ```

## File References

- Captured pcap files:
  - `/tmp/tshark_debug/java-capture.pcap` (12K)
  - `/tmp/tshark_debug/rust-capture.pcap` (35K)

- Analysis performed with:
  - `tshark` (Wireshark CLI) at `/opt/homebrew/bin/tshark`
  - Protocol hierarchy analysis (`-qz io,phs`)
  - gRPC message length extraction
  - Hex dump comparison

## Conclusion

The tshark analysis confirms that **Rust FE is sending an incomplete Thrift payload** compared to Java FE. The missing ~2350 bytes of data contains critical metadata needed for the BE to execute the scan correctly.

**Priority**: Fix descriptor table generation to include all necessary tuple/slot descriptors, which should add the missing metadata and resolve the "0 rows" issue.
