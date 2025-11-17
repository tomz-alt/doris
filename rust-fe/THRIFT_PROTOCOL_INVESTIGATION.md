# Thrift Protocol Investigation Summary

## Problem Statement
Rust FE sending TPipelineFragmentParamsList to Doris BE resulted in deserialization errors.

## Investigation Timeline

### Phase 1: Binary Protocol (Initial Attempt)
- **Encoding**: Thrift Binary Protocol
- **Payload Size**: ~4000-4200 bytes
- **Error**: `TProtocolException: Invalid data`
- **Root Cause**: **Wrong serialization protocol**

### Phase 2: Protocol Comparison
Successfully extracted Java FE Thrift payload from network capture:
- Used tcpdump on BE container port 8060 (gRPC)
- Parsed gRPC/HTTP2 frames to extract protobuf message
- Extracted Thrift payload (field 1) from PExecPlanFragmentRequest protobuf

**Key Discovery**: Java FE uses **Thrift Compact Protocol**, not Binary Protocol!

Evidence:
- Java payload: 6196 bytes
- Compact protocol byte patterns: `19 3c 15 00 1c...`
- Column names embedded throughout
- Contains aggregation query structure (GROUP BY, COUNT, SUM)

### Phase 3: Compact Protocol Switch
- **Change**: Switched from `BinaryEncode` to `CompactEncode` in thrift_codec crate
- **Payload Size**: Reduced from ~4000 to 1660 bytes, then 948 bytes
- **Error**: Changed from "Invalid data" to `invalid TType`
- **Progress**: BE now starts deserialization but fails mid-stream

### Phase 4: Byte-Level Analysis

#### Structural Differences Found:

**Number of Fragment Params**:
- Java FE (byte 1): `0x3c` = 3 TPipelineFragmentParams (distributed execution across 3 BEs)
- Rust FE (byte 1): `0x1c` = 1 TPipelineFragmentParams (single-node execution)

**Query Complexity**:
- Java: Aggregation query with GROUP BY, COUNT(*), SUM(l_quantity)
- Rust: Simple OLAP scan
- This explains ~6x size difference

#### Root Cause: Invalid Type Codes

**Critical Finding**: Rust FE payload contains bytes with invalid Thrift type codes (type 14).

Valid Thrift Compact Protocol type codes: 0-12
- 0: STOP
- 1: TRUE
- 2: FALSE
- 3: BYTE
- 4: I16
- 5: I32
- 6: I64
- 7: DOUBLE
- 8: BINARY (strings)
- 9: LIST
- 10: SET
- 11: MAP
- 12: STRUCT

**Invalid type 14 found at multiple positions**:
- Position 321: `0x0e` in context `0c110015 [0e] 15001c19`
- Position 337: `0x0e` in context `00120015 [0e] 15001500`
- Position 352: `0x0e` in context `74617815 [0e] 11001510`
- Position 366: `0x1e` in context `15001c15 [1e] 00001200`
- Position 408: `0x1e` in context `15001c15 [1e] 00001200`

(Note: Many type-14 occurrences are within string data like "l_linenumber" and are benign)

## Potential Causes

1. **thrift_codec Crate Bug**: The `thrift_codec` crate's `CompactEncode` implementation may not be fully compatible with Apache Thrift's C++ compact protocol implementation used by Doris BE.

2. **Wrong Field Types**: We may be using incorrect Thrift types for certain fields (e.g., using BYTE where I16 expected, or vice versa).

3. **Encoding of Special Values**: Certain values like enum constants, boolean flags, or small integers may be encoded incorrectly.

4. **Nested Structure Issues**: The compact protocol encoding of nested structs (TDescriptorTable, TOlapScanNode, etc.) may have subtle bugs.

## Comparison with Java FE

### Similarities:
- Both start with `0x19` (Field 1, type LIST)
- Both use Compact Protocol
- Bytes 2-5 match for corresponding fields (protocol_version, query_id struct marker)

### Differences:
- **Fragment Count**: Java=3, Rust=1
- **Payload Size**: Java=6196 bytes, Rust=948 bytes (~6.5x difference)
- **Query Type**: Java=aggregation with grouping, Rust=simple scan
- **Byte Divergence**: Starts at byte 1 due to different fragment counts

## Next Steps

### Option 1: Fix thrift_codec Usage
- Review every field type we're using
- Check if BYTE, I16, I32, I64, DOUBLE are correctly chosen
- Verify enum encoding (should be I32)
- Check boolean encoding (should use TRUE/FALSE types in compact protocol)

### Option 2: Switch to Different Thrift Library
- Consider using `fbthrift` or official Apache Thrift Rust bindings
- Generate structs from .thrift files instead of manual encoding

### Option 3: Minimal Reproducer
- Create minimal TPipelineFragmentParamsList with only required fields
- Binary search to find which specific field/struct causes invalid TType

### Option 4: Capture Simple Java Query
- Run a simple `SELECT * FROM lineitem LIMIT 1` from Java FE
- Capture that payload instead of aggregation query
- More apples-to-apples comparison

## Files Modified

1. `/Users/zhhanz/Documents/velodb/doris/rust-fe/src/be/thrift_pipeline.rs`:
   - Changed from `BinaryEncode` to `CompactEncode`
   - Commented out top-level `desc_tbl` field (field 2)

2. `/Users/zhhanz/Documents/velodb/doris/rust-fe/src/mysql/opensrv_shim.rs`:
   - Fixed pre-existing compilation errors (missing `.await`, wrong type for `error()`)

## Payloads Captured

- `/tmp/java-fe-thrift-extracted.bin`: 6196 bytes, Compact Protocol, aggregation query
- `/tmp/rust-fe-thrift.bin`: 948 bytes, Compact Protocol, simple scan
- `/tmp/java-fresh-capture.pcap`: Network capture containing Java FE request

## Conclusion

The investigation successfully identified:
1. ✅ Correct protocol: Thrift Compact (not Binary)
2. ✅ Error location: Mid-deserialization, specific byte with invalid type code
3. ❌ Exact fix: Not yet determined

The "invalid TType" error is caused by bytes with type code 14 in the Rust FE payload. This is either:
- A bug in `thrift_codec` crate's compact protocol implementation
- Incorrect field type choices in our manual encoding
- Missing or extra bytes causing desynchronization

Further debugging requires either fixing the specific invalid type codes or switching to a different Thrift encoding approach.
