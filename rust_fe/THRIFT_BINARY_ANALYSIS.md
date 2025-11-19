# Thrift Binary Analysis - Rust FE Serialization

## Overview
Analyzing the 112-byte output from our Rust FE TPipelineFragmentParamsList serialization
to identify potential deserialization issues with the C++ BE.

## Hex Dump
```
00000000  19 1c 15 00 1c 16 f2 c0  01 16 e4 a4 08 00 15 00  |................|
00000010  1b 00 1c 29 0c 00 29 0c  15 02 3c 18 13 32 30 32  |...)..)...<..202|
00000020  35 2d 31 31 2d 31 39 20  30 36 3a 30 33 3a 33 30  |5-11-19 06:03:30|
00000030  16 f8 fe af ab d3 66 18  03 55 54 43 12 00 1c 45  |......f..UTC...E|
00000040  80 40 86 80 80 80 80 10  25 a0 38 00 55 02 16 a2  |.@......%.8.U...|
00000050  9c 01 5c 1c 19 0c 00 00  19 1c 1c 16 f2 c0 01 16  |..\.............|
00000060  e4 a4 08 00 2b 00 15 00  25 00 00 e5 02 21 00 00  |....+...%....!..|
```

## TCompactProtocol Decoding

### Structure: TPipelineFragmentParamsList
According to PaloInternalService.thrift:
```thrift
struct TPipelineFragmentParamsList {
  1: list<TPipelineFragmentParams> params_list
}
```

### Byte-by-Byte Analysis

#### Offset 0x00: Field 1 - params_list (List of TPipelineFragmentParams)
- `0x19`: Field header
  - Type: LIST (9) with field delta
  - Wire type: 0x09 (list)
  - Field ID delta: 1 (so field ID = 1) ✓
- `0x1c`: List metadata
  - Element type: STRUCT (0x0c)
  - Size encoding: follows
- `0x15`: List size = 1 ✓ (we have 1 TPipelineFragmentParams)

#### Offset 0x03: Start of TPipelineFragmentParams[0]
```thrift
struct TPipelineFragmentParams {
  1: required PaloInternalServiceVersion protocol_version
  2: required Types.TUniqueId query_id
  3: optional i32 fragment_id
  4: required map<Types.TPlanNodeId, i32> per_exch_num_senders
  5: optional Descriptors.TDescriptorTable desc_tbl
  7: list<DataSinks.TPlanFragmentDestination> destinations  // NOT optional!
  8: optional i32 num_senders
  ...
}
```

- `0x00`: Field stop? NO - this would be wrong!

**PROBLEM IDENTIFIED!**

Looking at offset 0x03, we see `0x15 0x00` which seems wrong. Let me recalculate:

Actually wait, let me re-examine the encoding. In TCompactProtocol:
- 0x15 = i32 type with field delta
- 0x00 = i32 value of 0

So offset 0x03-0x04 is:
- Field 1: protocol_version (i32) = 0 ✓

Let me continue the analysis more carefully...

#### Continuing from offset 0x04:
- `0x1c`: Field header for field 2 (TUniqueId - struct)
  - Type: STRUCT (0x0c)
  - Field delta: 1 (field 2) ✓

- `0x16 0xf2 0xc0 0x01`: TUniqueId.hi (i64) = 12345
  - Field 1, i64 type
  - Zigzag encoded: 0xf2c001 = 12345 ✓

- `0x16 0xe4 0xa4 0x08`: TUniqueId.lo (i64) = 67890
  - Field 2, i64 type
  - Zigzag encoded: 0xe4a408 = 67890 ✓

- `0x00`: Field stop for TUniqueId struct ✓

#### Offset 0x0D: Field 3 - fragment_id (optional i32)
- `0x15 0x00`: i32 field, value = 0 ✓

#### Offset 0x0F: Field 4 - per_exch_num_senders (map)
- `0x1b 0x00`: Map field, size = 0 (empty map) ✓

#### Offset 0x11: Field 5 - desc_tbl (TDescriptorTable struct)
- `0x1c`: Struct field, delta = 1 (field 5) ✓
- Descr table contents follow...

So far the encoding looks CORRECT!

## Key Question

The serialization appears to follow TCompactProtocol correctly. The issue might be:

1. **Field ordering** - Are we writing fields in strictly ascending order?
2. **Missing required fields** - Field 7 (destinations) is REQUIRED
3. **Nested structure encoding** - Issues in TDescriptorTable or other nested structs
4. **Field stop markers** - Missing or incorrect field stops

## Next Steps

1. Verify field 7 (destinations) is being written even when empty
2. Check all nested struct field stops
3. Compare field write order with Java FE
4. Examine TDescriptorTable serialization in detail

## Expected Field Order in TPipelineFragmentParams
According to thrift spec and Java FE:
1. protocol_version (i32) - field 1
2. query_id (struct) - field 2
3. fragment_id (i32) - field 3
4. per_exch_num_senders (map) - field 4
5. desc_tbl (struct) - field 5
7. destinations (list) - field 7 ← MUST be present!
8. num_senders (i32) - field 8
10. coord (struct) - field 10
11. query_globals (struct) - field 11
12. query_options (struct) - field 12
17. fragment_num_on_host (i32) - field 17
18. backend_id (i64) - field 18
23. fragment (struct) - field 23
24. local_params (list) - field 24
38. total_instances (i32) - field 38
40. is_nereids (bool) - field 40

**Note the gap from field 5 to 7** - field 6 is not defined, so we jump to field 7.

## Hypothesis

The BE deserializer may be expecting field 7 (destinations) but we might be:
1. Skipping it entirely when empty
2. Writing it with incorrect encoding
3. Writing fields out of order causing confusion

Need to verify our write_pipeline_fragment_params function writes field 7 correctly.
