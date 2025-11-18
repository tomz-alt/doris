# PBlock Format Specification
## Reverse-Engineered from Doris C++ BE Source Code

**Session**: claude/rust-fe-todos-migration-012mCiokw5gZWbgBtbTPkHJr
**Date**: 2025-11-18
**Source**: `/home/user/doris/be/src/vec/core/block.cpp`
**CLAUDE.md Principle #2**: Using Java FE (C++ BE) behavior as specification

---

## Overview

PBlock is the wire format used by Doris C++ BE to send query results to FE. It consists of:
1. **Protobuf envelope** (`PBlock` message)
2. **Column metadata** (repeated `PColumnMeta`)
3. **Columnar data** (sequential byte stream in `column_values`)

## PBlock Protobuf Structure

```protobuf
message PBlock {
  optional int32 be_exec_version = 1;
  optional bool compressed = 2;
  optional int32 compression_type = 3;  // 0=Snappy, 1=LZ4, 2=ZSTD
  optional int32 uncompressed_size = 4;
  repeated PColumnMeta column_metas = 5;
  optional bytes column_values = 6;     // The actual columnar data
}

message PColumnMeta {
  optional string name = 1;
  optional PGenericType type = 2;
  // ... other fields
}
```

## Deserialization Flow

**From**: `Block::deserialize()` in `/home/user/doris/be/src/vec/core/block.cpp:117-167`

```
1. Check be_exec_version
2. Decompress column_values if compressed (Snappy/LZ4/ZSTD)
3. For each column_meta:
   a. Create DataType from meta
   b. Call type->deserialize(buf, &column, be_exec_version)
   c. Advance buffer pointer
4. Columns are now ready (still in columnar format)
5. Transpose to rows if needed
```

---

## Column Data Format (Sequential in column_values)

Each column's data is serialized sequentially. The buffer pointer advances through:
- Column 1 data
- Column 2 data
- Column 3 data
- ...

### Modern Format (be_exec_version >= USE_CONST_SERDE)

All types start with a common header:

#### Common Header (17 bytes on 64-bit)

```rust
struct ColumnHeader {
    is_const_column: bool,      // 1 byte
    row_num: usize,             // 8 bytes (size_t)
    real_need_copy_num: usize,  // 8 bytes (values saved: 1 if const, else row_num)
}
```

**From**: `serialize_const_flag_and_row_num()` in `/home/user/doris/be/src/vec/data_types/data_type.cpp`

---

## Type-Specific Formats

### 1. Number Types (INT, BIGINT, FLOAT, DOUBLE, DATE, DATETIME)

**From**: `DataTypeNumberBase::deserialize()` in `/home/user/doris/be/src/vec/data_types/data_type_number_base.cpp:187-209`

**Format**:
```
[ColumnHeader: 17 bytes]
[Data: variable]
```

**Data Section**:
```rust
let mem_size = real_need_copy_num * sizeof(T);

if mem_size <= SERIALIZED_MEM_SIZE_LIMIT {
    // Direct memcpy - just raw bytes
    // For BIGINT: 8 bytes per value, little-endian
    // For INT: 4 bytes per value, little-endian
    // For DATE: 8 bytes (stored as int64 internally)
    buf[0..mem_size]
} else {
    // StreamVByte compressed
    encode_size: size_t      // 8 bytes
    compressed_data: [u8]    // encode_size bytes
}
```

**Example - BIGINT with 3 rows**:
```
Offset  | Content                    | Description
--------|----------------------------|---------------------------
0       | 0x00                       | is_const_column = false
1-8     | 0x03 0x00 ... (little-end) | row_num = 3
9-16    | 0x03 0x00 ... (little-end) | real_need_copy_num = 3
17-24   | 0x01 0x00 ... (BIGINT)     | value[0] = 1
25-32   | 0x02 0x00 ... (BIGINT)     | value[1] = 2
33-40   | 0x03 0x00 ... (BIGINT)     | value[2] = 3
```

**Constants**:
- `SERIALIZED_MEM_SIZE_LIMIT`: Usually 1MB
- `USE_CONST_SERDE`: be_exec_version threshold (check BeExecVersionManager)

---

### 2. String Types (VARCHAR, CHAR, TEXT)

**From**: `DataTypeString::deserialize()` in `/home/user/doris/be/src/vec/data_types/data_type_string.cpp:206-248`

**Format**:
```
[ColumnHeader: 17 bytes]
[Offsets Array: variable]
[Total Value Length: 8 bytes]
[String Values: variable]
```

**Offsets Array**:
```rust
let mem_size = real_need_copy_num * sizeof(IColumn::Offset);  // Offset = u64

if mem_size <= SERIALIZED_MEM_SIZE_LIMIT {
    // Direct array of u64 offsets
    offsets: [u64; real_need_copy_num]
} else {
    // StreamVByte compressed
    encode_size: size_t
    compressed_offsets: [u8]
}
```

**String Values**:
```rust
value_len: size_t  // 8 bytes - total length of all strings

if value_len <= SERIALIZED_MEM_SIZE_LIMIT {
    // Raw string bytes concatenated
    values: [u8; value_len]
} else {
    // LZ4 compressed
    encode_size: size_t
    compressed_values: [u8]
}
```

**Example - 3 strings: "abc", "de", "f"**:
```
Offset  | Content                    | Description
--------|----------------------------|---------------------------
0       | 0x00                       | is_const_column = false
1-8     | 0x03 0x00 ...              | row_num = 3
9-16    | 0x03 0x00 ...              | real_need_copy_num = 3
17-24   | 0x03 0x00 ... (offset[0])  | offset = 3 (end of "abc")
25-32   | 0x05 0x00 ... (offset[1])  | offset = 5 (end of "de")
33-40   | 0x06 0x00 ... (offset[2])  | offset = 6 (end of "f")
41-48   | 0x06 0x00 ...              | value_len = 6 total bytes
49-51   | 'a' 'b' 'c'                | string 0
52-53   | 'd' 'e'                    | string 1
54      | 'f'                        | string 2
```

**Offset Calculation**:
- `offset[i]` is the END position (cumulative)
- String `i` = `values[offset[i-1]..offset[i]]` (where offset[-1] = 0)

---

### 3. Nullable Columns

**Not yet fully documented** - requires reading `data_type_nullable.cpp`

Expected format:
```
[ColumnHeader]
[Null Bitmap: variable]
[Nested Type Data]
```

Null bitmap is likely a bitset indicating which rows are NULL.

---

## Compression Support

### Snappy Compression (compression_type = 0)

**From**: `Block::deserialize()` lines 142-148

```rust
use snap::raw::{decompress_len, Decoder};

let compressed_data = pblock.column_values();
let uncompressed_size = decompress_len(compressed_data)?;
let mut decompressed = vec![0u8; uncompressed_size];
let decoder = Decoder::new();
decoder.decompress(compressed_data, &mut decompressed)?;
```

### LZ4 Compression (compression_type = 1)

Used for large string values.

---

## Legacy Format (be_exec_version < USE_CONST_SERDE)

### Numbers
```
mem_size: u32          // 4 bytes
data: [T; row_count]   // or compressed
```

### Strings
```
mem_size: u32          // 4 bytes (for offsets array size)
offsets: [u64; N]      // or compressed
value_len: u64         // 8 bytes
values: [u8]           // or compressed
```

---

## Implementation Notes for Rust

### Reading Primitives

```rust
use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt};

fn read_bool(cursor: &mut Cursor<&[u8]>) -> bool {
    let mut buf = [0u8; 1];
    cursor.read_exact(&mut buf).unwrap();
    buf[0] != 0
}

fn read_usize(cursor: &mut Cursor<&[u8]>) -> usize {
    cursor.read_u64::<LittleEndian>().unwrap() as usize
}

fn read_i64(cursor: &mut Cursor<&[u8]>) -> i64 {
    cursor.read_i64::<LittleEndian>().unwrap()
}
```

### Column Header

```rust
struct ColumnHeader {
    is_const_column: bool,
    row_num: usize,
    real_need_copy_num: usize,
}

fn read_column_header(cursor: &mut Cursor<&[u8]>) -> Result<ColumnHeader> {
    let is_const_column = read_bool(cursor);
    let row_num = read_usize(cursor);
    let real_need_copy_num = read_usize(cursor);

    Ok(ColumnHeader {
        is_const_column,
        row_num,
        real_need_copy_num,
    })
}
```

### BIGINT Decoder

```rust
fn decode_bigint_column(cursor: &mut Cursor<&[u8]>, be_exec_version: i32) -> Result<Vec<Value>> {
    let header = read_column_header(cursor)?;
    let mut values = Vec::with_capacity(header.row_num);

    // Read data
    for _ in 0..header.real_need_copy_num {
        let val = cursor.read_i64::<LittleEndian>()?;
        values.push(Value::BigInt(val));
    }

    // Expand const column if needed
    if header.is_const_column && header.row_num > 1 {
        let const_val = values[0].clone();
        values = vec![const_val; header.row_num];
    }

    Ok(values)
}
```

### STRING Decoder

```rust
fn decode_string_column(cursor: &mut Cursor<&[u8]>, be_exec_version: i32) -> Result<Vec<Value>> {
    let header = read_column_header(cursor)?;

    // Read offsets
    let mut offsets = vec![0u64; header.real_need_copy_num];
    for i in 0..header.real_need_copy_num {
        offsets[i] = cursor.read_u64::<LittleEndian>()?;
    }

    // Read total value length
    let value_len = read_usize(cursor);

    // Read concatenated string data
    let mut string_data = vec![0u8; value_len];
    cursor.read_exact(&mut string_data)?;

    // Extract individual strings
    let mut values = Vec::with_capacity(header.row_num);
    let mut prev_offset = 0;
    for &offset in &offsets {
        let str_bytes = &string_data[prev_offset..offset as usize];
        let s = String::from_utf8_lossy(str_bytes).to_string();
        values.push(Value::String(s));
        prev_offset = offset as usize;
    }

    // Expand const if needed
    if header.is_const_column && header.row_num > 1 {
        let const_val = values[0].clone();
        values = vec![const_val; header.row_num];
    }

    Ok(values)
}
```

---

## Testing Strategy

1. **Capture real PBlock bytes** from a simple query
2. **Log hex dump** of column_values after decompression
3. **Parse manually** to verify format understanding
4. **Implement decoders** incrementally (BIGINT → STRING → DATE)
5. **Compare results** with Java FE for identical queries

---

## References

- `/home/user/doris/be/src/vec/core/block.cpp` - Block serialization
- `/home/user/doris/be/src/vec/data_types/data_type_number_base.cpp` - Number types
- `/home/user/doris/be/src/vec/data_types/data_type_string.cpp` - String types
- `/home/user/doris/be/src/vec/data_types/data_type.cpp` - Common header functions

---

## Status

- ✅ Format specification complete
- ✅ Number type format understood
- ✅ String type format understood
- ⏳ Rust implementation in progress
- ⏳ Testing with real data needed

**Next**: Implement decoders in `fe-backend-client/src/pblock_parser_v2.rs`
