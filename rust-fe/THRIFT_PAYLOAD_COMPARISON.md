# Thrift Payload Comparison: Rust FE vs Java FE

## Overview

This document describes the tooling and process for comparing Thrift payloads between the Rust FE and Java FE implementations for TPC-H lineitem scan queries.

## What's Been Implemented

### 1. Rust FE Payload Generation

**File:** `src/be/pool.rs:129-134`

The `execute_tpch_lineitem_scan_fragments()` method now automatically dumps the generated Thrift payload to `/tmp/rust-fe-thrift.bin` before sending it to the Backend.

**File:** `examples/generate_payload.rs`

A standalone example that generates the Thrift payload without requiring a running Backend:

```bash
cargo run --no-default-features --features real_be_proto --example generate_payload
```

Output:
- `/tmp/rust-fe-thrift.bin` - Raw Thrift binary (2000 bytes)
- Console output showing payload size

### 2. Thrift Decoder

**File:** `examples/decode_thrift.rs`

Enhanced recursive Thrift decoder that prints the complete structure of any TPipelineFragmentParamsList payload:

```bash
cargo run --no-default-features --features real_be_proto \
  --example decode_thrift -- /tmp/rust-fe-thrift.bin
```

Features:
- Recursive expansion of Struct, List, and Map fields
- Field ID numbering for easy Thrift schema mapping
- Human-readable output showing full payload structure

### 3. Automated Comparison Script

**File:** `scripts/compare_thrift_payloads.sh`

One-command tool that:
1. Generates Rust FE payload
2. Decodes Rust FE payload
3. Decodes Java FE payload (if available)
4. Produces side-by-side diff
5. Shows size and structural comparison

```bash
./scripts/compare_thrift_payloads.sh
```

## Rust FE Payload Structure

The current Rust FE implementation generates a TPipelineFragmentParamsList with:

### Top-Level Fields
- **Field 1:** `params_list` - List containing 1 TPipelineFragmentParams
- **Field 9:** `is_nereids` - Bool(true)
- **Field 11:** `query_id` - Struct with hi/lo i64 values

### TPipelineFragmentParams (Field 1, Element [0])
- **Field 1:** `protocol_version` = 0 (V1)
- **Field 2:** `query_id` - Struct with hi/lo i64
- **Field 3:** `fragment_id` = 0
- **Field 4:** `per_exch_num_senders` - Empty Map (no exchanges in simple scan)
- **Field 5:** `desc_tbl` - Full TDescriptorTable (see below)
- **Field 21:** `is_simplified_param` = false
- **Field 23:** `fragment` - TPlanFragment (see below)
- **Field 28:** `table_name` = "lineitem"
- **Field 40:** `is_nereids` = true

### TDescriptorTable (Field 5)
- **Field 1:** `slotDescriptors` - List of 16 TSlotDescriptor (one per column):
  - l_orderkey, l_partkey, l_suppkey, l_linenumber (INT)
  - l_quantity, l_extendedprice, l_discount, l_tax (DECIMAL128)
  - l_returnflag, l_linestatus (CHAR)
  - l_shipdate, l_commitdate, l_receiptdate (DATE)
  - l_shipinstruct, l_shipmode (VARCHAR)
  - l_comment (VARCHAR)
- **Field 2:** `tupleDescriptors` - List with 1 TTupleDescriptor:
  - id = 0 (matches row_tuples in TPlanNode)
  - tableId: **NOT SET** (potential gap vs Java FE)
- **Field 3:** `tableDescriptors` - **NOT SET** (potential gap vs Java FE)

### TPlanFragment (Field 23)
- **Field 2:** `plan` - TPlan with single-node list
- **Field 6:** `partition` - UNPARTITIONED

### TPlanNode (inside TPlan)
- **Field 1:** `node_id` = 0
- **Field 2:** `node_type` = 0 (OLAP_SCAN_NODE)
- **Field 3:** `num_children` = 0
- **Field 4:** `limit` = -1
- **Field 5:** `row_tuples` = [0]
- **Field 6:** `nullable_tuples` = [false]
- **Field 8:** `compact_data` = true
- **Field 18:** `olap_scan_node` - TOlapScanNode (see below)

### TOlapScanNode (Field 18)
- **Field 1:** `tuple_id` = 0
- **Field 2:** `key_column_name` = [] (empty list)
- **Field 3:** `key_column_type` = [] (empty list)
- **Field 4:** `is_preaggregation` = true
- **Field 7:** `table_name` = "lineitem"

## Next Steps: Capturing Java FE Payload

### Prerequisites
- Running docker compose with Java FE (4.0.1) and BE
- tcpdump installed on BE container

### Option 1: Network Capture (Recommended)

1. **Start docker compose:**
   ```bash
   ./docker/quickstart.sh
   ```

2. **Capture BE traffic:**
   ```bash
   # In one terminal
   docker exec -it doris-be bash
   tcpdump -i any -w /tmp/be-capture.pcap tcp port 8060
   ```

3. **Run query via Java FE:**
   ```bash
   # In another terminal
   docker exec -it doris-mysql-client mysql -h doris-fe-java -P 9030 -u root \
     -e "USE tpch; SELECT * FROM lineitem LIMIT 1;"
   ```

4. **Stop tcpdump** (Ctrl+C in BE terminal)

5. **Extract payload from pcap:**
   - Use Wireshark or tshark to filter for gRPC `PExecPlanFragmentRequest`
   - Extract the `request` field bytes (TPipelineFragmentParamsList)
   - Save as `/tmp/java-fe-thrift.bin`

   Example with tshark:
   ```bash
   # Copy pcap from container
   docker cp doris-be:/tmp/be-capture.pcap /tmp/

   # Extract HTTP/2 data frames
   tshark -r /tmp/be-capture.pcap -Y "http2.data.data" -T fields -e http2.data.data
   ```

### Option 2: BE Logging

If Doris BE supports logging request payloads:
1. Enable DEBUG logging for fragment execution
2. Look for hex dumps of TPipelineFragmentParamsList
3. Convert hex string to binary: `/tmp/java-fe-thrift.bin`

### Option 3: Java FE Instrumentation

Modify Java FE code to dump the Thrift payload:
```java
// In CoordinatorContext or similar
byte[] thriftBytes = TSerializer.serialize(pipelineParamsList);
Files.write(Paths.get("/tmp/java-fe-thrift.bin"), thriftBytes);
```

## Running the Comparison

Once `/tmp/java-fe-thrift.bin` exists:

```bash
./scripts/compare_thrift_payloads.sh
```

This will output:
- Size comparison (bytes and decoded lines)
- Structural diff showing field-by-field differences
- Summary of key differences

## Expected Differences (Java FE vs Rust FE)

Based on the Thrift schema analysis, expect these differences:

### At TPipelineFragmentParams Level
- Java FE may set additional fields not currently in Rust FE:
  - Field 7: `destinations` (for multi-backend routing)
  - Field 10: `coord` (coordinator info)
  - Field 11: `query_globals` (timezone, query options)
  - Field 12: `query_options` (session variables)
  - Fields 20-39: Workload group, parallelism, bucket shuffle maps

### At TDescriptorTable Level
- Java FE likely sets:
  - `TTupleDescriptor.tableId` (non-zero table ID from metadata)
  - `TDescriptorTable.tableDescriptors` (TTableDescriptor for tpch.lineitem)

### At TOlapScanNode Level
- Java FE may populate:
  - `key_column_name` and `key_column_type` with actual key columns
  - Tablet/partition distribution fields

### At PExecPlanFragmentRequest Level
- Java FE version field (VERSION_3 vs VERSION_4)
- Compact flag value

## Alignment Strategy

After comparing payloads:

1. **Must-have alignments** (likely required by BE):
   - Ensure `protocol_version` matches Java
   - Add `TTupleDescriptor.tableId` if BE requires it
   - Add `TDescriptorTable.tableDescriptors` if BE requires it

2. **Nice-to-have alignments** (for feature parity):
   - Populate `key_column_name` / `key_column_type`
   - Add `query_options` for session variable support
   - Add `query_globals` for timezone support

3. **Deferred alignments** (advanced features):
   - Workload group / resource management fields
   - Bucket shuffle optimization maps
   - Runtime filter builder state

## Files Modified

- `src/be/pool.rs` - Added payload dumping (+6 lines)
- `src/be/thrift_pipeline.rs` - Fixed string list encoding for key columns (+4 lines)
- `examples/generate_payload.rs` - New standalone payload generator (+44 lines)
- `examples/decode_thrift.rs` - Enhanced with recursive expansion (+65 lines)
- `scripts/compare_thrift_payloads.sh` - New comparison automation script (+165 lines)

## Quick Reference

```bash
# Generate Rust FE payload
cargo run --no-default-features --features real_be_proto --example generate_payload

# Decode any Thrift payload
cargo run --no-default-features --features real_be_proto \
  --example decode_thrift -- /path/to/payload.bin

# Full comparison (requires Java payload)
./scripts/compare_thrift_payloads.sh

# View decoded Rust FE structure
cat /tmp/rust-thrift-decoded.txt

# Check payload sizes
ls -lh /tmp/{rust,java}-fe-thrift.bin
```

## Troubleshooting

**Q: Compilation errors about `execute_query` method not found**
A: Use `--no-default-features --features real_be_proto` to avoid MySQL shim

**Q: Can't capture Java FE payload from tcpdump**
A: Ensure BE is using port 8060 for gRPC (not 9060), or adjust tcpdump filter

**Q: Thrift decode fails with "unexpected EOF"**
A: Verify the binary file contains only the TPipelineFragmentParamsList payload, not the full gRPC frame

**Q: How do I know which fields to add to Rust FE?**
A: Run the comparison script, note missing field IDs in the diff, then consult `PaloInternalService.thrift` schema

## References

- Doris Thrift Schema: `../gensrc/thrift/PaloInternalService.thrift`
- Rust Thrift Encoder: `src/be/thrift_pipeline.rs`
- Catalog Schema: `src/catalog/tpch_tables.rs`
- BE Client: `src/be/client.rs`
