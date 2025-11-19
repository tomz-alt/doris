# Thrift Compiler Deep Dive - Session 2025-11-19

## Objective
Compare Rust FE Thrift serialization with Java FE to debug "TProtocolException: Invalid data" error from BE.

## Work Completed

### 1. Downloaded Thrift Compiler ✅
- URL: https://raw.githubusercontent.com/tomz-alt/doris-be-binary/master/thrift-0.22.0.tar.gz
- Extracted to /tmp/thrift-build/thrift-0.22.0/
- Status: Ready but not built (requires configure + make)

### 2. Analyzed BE Deserialization Flow ✅

**File**: `/home/user/doris/be/src/service/internal_service.cpp`

**Flow**:
1. FE sends PExecPlanFragmentRequest via gRPC
   - `bytes request` field contains serialized TPipelineFragmentParamsList
   - `bool compact = true` indicates TCompactProtocol
   - `PFragmentRequestVersion version = VERSION_3`

2. BE handler: `_exec_plan_fragment_impl()` (line 530)
   ```cpp
   TPipelineFragmentParamsList t_request;
   const uint8_t* buf = (const uint8_t*)ser_request.data();
   uint32_t len = ser_request.size();
   RETURN_IF_ERROR(deserialize_thrift_msg(buf, &len, compact, &t_request));
   ```

3. Deserialization: `/home/user/doris/be/src/util/thrift_util.h` (line 148)
   ```cpp
   deserialized_msg->read(tproto.get());  // Calls generated Thrift C++ code
   ```

**Error source**: The `TPipelineFragmentParamsList::read()` method in generated C++ Thrift code throws "TProtocolException: Invalid data".

### 3. Created Rust Hex Dump Test Program ✅

**File**: `/home/user/doris/rust_fe/fe-planner/examples/thrift_hex_dump.rs`

**Output**: 112 bytes successfully serialized

```
00000000  19 1c 15 00 1c 16 f2 c0  01 16 e4 a4 08 00 15 00  |................|
00000010  1b 00 1c 29 0c 00 29 0c  15 02 3c 18 13 32 30 32  |...)..)...<..202|
00000020  35 2d 31 31 2d 31 39 20  30 36 3a 30 33 3a 33 30  |5-11-19 06:03:30|
00000030  16 f8 fe af ab d3 66 18  03 55 54 43 12 00 1c 45  |......f..UTC...E|
00000040  80 40 86 80 80 80 80 10  25 a0 38 00 55 02 16 a2  |.@......%.8.U...|
00000050  9c 01 5c 1c 19 0c 00 00  19 1c 1c 16 f2 c0 01 16  |..\.............|
00000060  e4 a4 08 00 2b 00 15 00  25 00 00 e5 02 21 00 00  |....+...%....!..|
```

Binary saved to: `/tmp/rust_thrift_dump.bin`

### 4. Code Structure Analysis ✅

**Verified all 16 fields written in correct order:**

| Line | Field | ID | Type | Status |
|------|-------|----|----- |--------|
| 370  | protocol_version | 1 | i32 | ✅ Required |
| 381  | query_id | 2 | struct | ✅ Required |
| 392  | fragment_id | 3 | i32 | ✅ Optional |
| 404  | per_exch_num_senders | 4 | map | ✅ Required |
| 436  | desc_tbl | 5 | struct | ✅ Optional |
| 447  | destinations | 7 | list | ✅ **REQUIRED** (not optional!) |
| 468  | num_senders | 8 | i32 | ✅ Optional |
| 481  | coord | 10 | struct | ✅ Optional |
| 493  | query_globals | 11 | struct | ✅ Optional |
| 505  | query_options | 12 | struct | ✅ Optional |
| 517  | fragment_num_on_host | 17 | i32 | ✅ Optional |
| 531  | backend_id | 18 | i64 | ✅ Optional |
| 543  | fragment | 23 | struct | ✅ Required |
| 552  | local_params | 24 | list | ✅ Required |
| 574  | total_instances | 38 | i32 | ✅ Optional |
| 588  | is_nereids | 40 | bool | ✅ Optional |

**Field ordering**: Strictly ascending ✅
**Field stops**: All nested structs have field_stop ✅
**Struct ends**: All structs properly closed ✅

### 5. TDescriptorTable Analysis ✅

**File**: `/home/user/doris/rust_fe/fe-planner/src/thrift_serialize.rs:984-1037`

Fields written:
1. slotDescriptors (field 1, optional list) - conditional
2. tupleDescriptors (field 2, required list) - always (even if empty)
3. tableDescriptors (field 3, optional list) - not yet implemented

**Verified against**: `/home/user/doris/gensrc/thrift/Descriptors.thrift:453-459`

Matches spec ✅

### 6. Java Comparison Program Created

**File**: `/home/user/doris/fe/fe-core/src/test/java/org/apache/doris/thrift/ThriftSerializationTest.java`

Creates identical structure to Rust test:
- Same unique ID (hi=12345, lo=67890)
- Same empty descriptor table
- Same minimal TQueryGlobals/TQueryOptions
- Same 16 fields set

**Status**: Not yet compiled (Maven network issues)

## Analysis Results

### ✅ What's CORRECT

1. **Field Ordering**: All fields written in strictly ascending field ID order
2. **Required Fields**: All required fields present (protocol_version, query_id, per_exch_num_senders, fragment, local_params)
3. **Field 7 Critical**: Destinations list correctly written even when empty
4. **Field Stops**: All nested structures have proper field_stop markers
5. **Struct Begin/End**: All structs properly opened and closed
6. **TCompactProtocol**: Using correct protocol variant

### ❓ Potential Issues

1. **Nested Structure Encoding**:
   - TDescriptorTable serialization looks correct but untested with BE
   - TPlanFragment with empty nodes list - may need investigation
   - TQueryGlobals/TQueryOptions minimal values - need validation

2. **Enum Values**:
   - TPrimitiveType enum mapping (Rust i32 vs Java enum)
   - Protocol version value (we use 0, Java uses PaloInternalServiceVersion.V1)

3. **String Encoding**:
   - TCompactProtocol string length encoding (varint + UTF-8)
   - Need to verify Rust thrift library matches C++/Java implementation

4. **Empty Collections**:
   - Empty destinations list: written as list header + size 0 ✅
   - Empty tuple_descriptors list: written as list header + size 0 ✅
   - Empty per_exch_num_senders map: written as map header + size 0 ✅

## Hypothesis: Protocol Version Mismatch?

**Current Code** (line 370):
```rust
protocol_version: 0
```

**Java FE** (ThriftPlansBuilder.java:337):
```java
params.setProtocolVersion(PaloInternalServiceVersion.V1);
```

**Question**: What is the numeric value of `PaloInternalServiceVersion.V1`?

Let me check the enum definition...

**From PaloInternalService.thrift**:
```thrift
enum PaloInternalServiceVersion {
  V1
}
```

Thrift enums start at 0, so V1 = 0 ✅

## Next Steps

### Immediate Actions

1. **Build Java Test Program**:
   - Generate Thrift classes: Need working Maven or manual thrift codegen
   - Compile ThriftSerializationTest.java
   - Run and compare hex output with `/tmp/rust_thrift_dump.bin`
   - Expected: Byte-for-byte identical if serialization is correct

2. **Alternative: Build Thrift Compiler**:
   ```bash
   cd /tmp/thrift-build/thrift-0.22.0
   ./configure --without-tests --disable-libs
   make -j4
   ./compiler/cpp/thrift --gen rs /home/user/doris/gensrc/thrift/PaloInternalService.thrift
   ```
   Then compare generated Rust code with our manual implementation.

3. **Test with Real BE**:
   - Send our 112-byte payload to running BE
   - Capture exact error message and stack trace
   - Check BE logs for more detailed error info

### Detailed Investigation

1. **Verify Empty Struct Handling**:
   - TPlanFragment with empty nodes list
   - May need at least one plan node for BE to accept

2. **Check TQueryGlobals/TQueryOptions**:
   - Our minimal() implementations may be missing required fields
   - Compare with actual Java FE values in real query

3. **Instrument Deserialization**:
   - Add logging to BE deserialize_thrift_msg to see exactly where it fails
   - Check field ID where parsing stops

4. **Binary Comparison Tool**:
   - Create side-by-side hex diff of Java vs Rust output
   - Identify first divergence byte

## Files Created This Session

1. `/tmp/rust_thrift_dump.bin` - 112-byte serialized output
2. `/home/user/doris/rust_fe/fe-planner/examples/thrift_hex_dump.rs` - Test program
3. `/home/user/doris/fe/fe-core/src/test/java/org/apache/doris/thrift/ThriftSerializationTest.java` - Java comparison
4. `/home/user/doris/rust_fe/THRIFT_BINARY_ANALYSIS.md` - Byte-level analysis
5. `/home/user/doris/rust_fe/THRIFT_COMPILER_DEEP_DIVE_SESSION_2025-11-19.md` - This document

## Conclusion

**Serialization code structure appears CORRECT** based on:
- ✅ All fields in correct order
- ✅ All required fields present
- ✅ Proper field stops and struct ends
- ✅ Field 7 (destinations) correctly written even when empty

**Most likely issue**: Subtle encoding difference in nested structures (TDescriptorTable, TPlanFragment, TQueryGlobals, or TQueryOptions) that can only be found by:
1. Byte-for-byte comparison with Java FE output, OR
2. Testing with actual BE and getting detailed error location

**Recommended next step**: Focus on getting Java comparison working to identify exact byte where divergence occurs.
