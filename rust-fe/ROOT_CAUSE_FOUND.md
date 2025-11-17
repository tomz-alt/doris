# Root Cause: thrift_codec Compact Protocol Bug

## TL;DR

The `thrift_codec` crate has a **critical bug** in its Compact Protocol implementation: it adds STOP bytes (`0x00`) after **nested structs**, which violates the Thrift Compact Protocol specification.

## Evidence

### Byte-by-Byte Comparison

**Java FE (working) at position 27:**
```
Position 26: 0x01  (end of varint for TUniqueId.lo)
Position 27: 0x15  (Field 3 of parent struct - fragment_id)
```

**Rust FE (failing) at position 27:**
```
Position 26: 0x01  (end of varint for TUniqueId.lo)
Position 27: 0x00  (STOP - SHOULD NOT BE HERE!)
Position 28: 0x15  (trying to be Field 3, but parent struct already ended)
```

### What Should Happen

In Thrift Compact Protocol:
1. **Top-level struct**: Fields are encoded, then a STOP byte (0x00) marks the end
2. **Nested struct** (field of another struct): Fields are encoded, NO STOP byte - the parent continues reading its next field

### What Actually Happens

The `thrift_codec` crate's `CompactEncode` adds STOP bytes after **all** structs, including nested ones. This causes:

1. TUniqueId (nested in TPipelineFragmentParams) ends with field 2 (lo)
2. `thrift_codec` writes **0x00 STOP byte** ❌ (BUG!)
3. Parent struct (TPipelineFragmentParams) thinks it's ended
4. BE tries to continue reading, encounters garbage data
5. BE throws "TProtocolException: Invalid data" or "invalid TType"

## Verification

Captured Java FE payload for identical simple scan query:
- **File**: `/tmp/java-simple-scan-thrift.bin` (3618 bytes)
- **Query**: `SELECT * FROM tpch_sf1.lineitem LIMIT 3`
- **No premature STOP bytes** in nested structs ✓
- **BE accepts it successfully** ✓

Rust FE payload:
- **File**: `/tmp/rust-fe-thrift.bin` (951 bytes)
- **Has premature STOP byte** at position 27 ❌
- **BE rejects with deserialization error** ❌

## Impact

This bug makes `thrift_codec` **incompatible** with Apache Thrift's C++ implementation (used by Doris BE) for Compact Protocol when using nested structs.

## Solutions

### Option 1: Use Different Thrift Library ⭐ RECOMMENDED

Use a library with correct Compact Protocol implementation:
- **Apache Thrift official Rust bindings** (if available)
- **`compact-thrift` crate** (specifically designed for Apache Thrift compatibility)
- Generate code from `.thrift` files using `thrift` compiler

### Option 2: Fix `thrift_codec` Crate

Submit PR to fix Compact Protocol nested struct encoding:
- Remove STOP byte emission for nested structs
- Only emit STOP for top-level message encoding
- Add test cases for nested struct scenarios

### Option 3: Manual Encoding Workaround

Bypass `ThriftStruct::compact_encode()` for nested structures:
- Manually encode bytes following Compact Protocol spec
- Only use `thrift_codec` for primitive types
- Complex and error-prone ❌

## Next Steps

1. **Try `compact-thrift` crate** - Add to Cargo.toml and test if it correctly encodes without premature STOP bytes

2. **Or use official Apache Thrift** - Generate Rust code from Doris `.thrift` files:
   ```bash
   thrift --gen rs PaloInternalService.thrift
   ```

3. **Verify fix** - Compare generated payload with Java FE payload byte-by-byte

## Timeline

- **2025-11-16 01:00-02:00**: Switched from Binary to Compact Protocol
- **2025-11-16 02:00-02:10**: Captured Java FE simple scan payload
- **2025-11-16 02:10**: **Identified premature STOP byte at position 27**
- **2025-11-16 02:15**: **Confirmed `thrift_codec` bug**

## Files

- `/tmp/java-simple-scan-thrift.bin` - Working Java FE payload (3618 bytes)
- `/tmp/rust-fe-thrift.bin` - Broken Rust FE payload (951 bytes)
- `/tmp/simple-scan-capture.pcap` - Network capture of Java FE → BE communication
- `THRIFT_PROTOCOL_INVESTIGATION.md` - Detailed investigation notes
