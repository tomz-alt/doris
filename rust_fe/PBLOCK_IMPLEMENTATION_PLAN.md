# PBlock Parser Implementation Plan
## Critical for TPC-H E2E Goal

**Goal**: Parse real query results from C++ BE to achieve "mysql java jdbc to rust fe to C++ BE e2e TPC_H fully passed"

**CLAUDE.md Principle #2**: Use Java FE behavior as specification
**CLAUDE.md Principle #4**: Hide transport details behind clean Row/Value API

---

## Current Status

### ✅ What Works
- PBlock protobuf deserialization
- Column metadata extraction (names, types)
- Basic structure parsing
- Placeholder row generation

### ⏳ What's Needed (CRITICAL)
- **Actual columnar data parsing**
- Type-specific value decoding
- Null bitmap handling
- Compression support (Snappy)

### ❌ Current Blocker
Without proper PBlock parsing, we cannot:
- Get real query results from BE
- Test Rust FE → BE pipeline
- Compare with Java FE results
- Run TPC-H queries

---

## Implementation Strategy

### Phase 1: Understand the Format (CLAUDE.md #2)

**Approach**: Query Java FE, capture actual PBlock bytes, reverse-engineer format

**Steps**:
1. Run query through Java FE → BE
2. Capture PBlock protobuf bytes (add logging to BE or FE)
3. Analyze byte structure
4. Document format specification
5. Implement parser matching that format

**Alternative**: Read Doris C++ BE source code for PBlock encoding logic

### Phase 2: Implement Basic Types

**Priority Order** (based on TPC-H schema):
1. **BIGINT** - l_orderkey, l_partkey, l_suppkey (most common)
2. **INT** - l_linenumber
3. **DECIMAL** - l_quantity, l_extendedprice, l_discount, l_tax
4. **CHAR/VARCHAR** - l_returnflag, l_linestatus, l_shipinstruct, l_comment
5. **DATE** - l_shipdate, l_commitdate, l_receiptdate

**For each type**:
- Determine byte encoding
- Implement decoder
- Handle null values
- Add tests

### Phase 3: Handle Advanced Features

**Compression**:
- Add `snap` crate for Snappy
- Implement decompress_data()
- Test with compressed responses

**Null Handling**:
- Parse null bitmap
- Convert to Value::Null

**Performance**:
- Optimize for large result sets
- Memory-efficient transposition

---

## Code Structure (CLAUDE.md #4)

### Clean API (User-Facing)
```rust
// Users only see this - no protobuf/columnar details
pub fn parse_pblock(bytes: &[u8]) -> Result<PBlock>;
pub fn pblock_to_rows(block: &PBlock) -> Result<Vec<Row>>;

pub struct Row {
    pub values: Vec<Value>,
}

pub enum Value {
    Null, Boolean(bool), Int(i32), BigInt(i64),
    Decimal(String), String(String), Date(String),
}
```

### Internal Implementation (Hidden)
```rust
// These are internal - users never see columnar format
fn parse_columnar_data(data: &[u8], metas: &[PColumnMeta]) -> Result<Vec<Row>>;
fn parse_column(cursor: &mut Cursor, meta: &PColumnMeta) -> Result<Vec<Value>>;
fn decode_bigint_column(cursor: &mut Cursor, count: usize) -> Result<Vec<Value>>;
fn decode_string_column(cursor: &mut Cursor, count: usize) -> Result<Vec<Value>>;
fn transpose_columns_to_rows(columns: Vec<Vec<Value>>) -> Result<Vec<Row>>;
```

---

## Testing Strategy

### Test 1: Empty Result Set
```sql
SELECT * FROM lineitem WHERE 1=0
```
**Expected**: 0 rows
**Tests**: Empty PBlock handling

### Test 2: Single Row, Simple Types
```sql
SELECT l_orderkey, l_partkey FROM lineitem LIMIT 1
```
**Expected**: 1 row, 2 columns (BIGINT, BIGINT)
**Tests**: Basic type parsing

### Test 3: Multiple Rows
```sql
SELECT l_orderkey FROM lineitem LIMIT 10
```
**Expected**: 10 rows, 1 column (BIGINT)
**Tests**: Row count handling, transposition

### Test 4: Mixed Types
```sql
SELECT l_orderkey, l_quantity, l_returnflag, l_shipdate FROM lineitem LIMIT 1
```
**Expected**: 1 row, 4 columns (BIGINT, DECIMAL, CHAR, DATE)
**Tests**: Multiple type decoders

### Test 5: Null Values
```sql
SELECT CAST(NULL AS BIGINT), l_orderkey FROM lineitem LIMIT 1
```
**Expected**: 1 row with null in first column
**Tests**: Null bitmap parsing

### Test 6: Large Result Set
```sql
SELECT * FROM lineitem
```
**Expected**: All 4 rows (or more if data is added)
**Tests**: Performance, memory usage

### Test 7: Aggregations (TPC-H Goal!)
```sql
SELECT l_returnflag, COUNT(*), SUM(l_quantity) FROM lineitem GROUP BY l_returnflag
```
**Expected**: Grouped results
**Tests**: Complex query results

---

## Implementation Timeline

### Immediate (2-3 hours)
1. Research PBlock format (read Doris source or capture bytes)
2. Implement BIGINT decoder
3. Test with simple query

### Short-term (4-6 hours)
4. Implement INT, STRING, DATE decoders
5. Add null bitmap handling
6. Test with lineitem queries

### Medium-term (2-3 hours)
7. Implement DECIMAL decoder
8. Add Snappy decompression
9. Optimize transposition

### Final (1-2 hours)
10. Test with all TPC-H query result types
11. Performance tuning
12. Documentation

**Total**: ~10-14 hours to complete

---

## Knowledge Gaps (Need to Answer)

### Format Questions
1. How is row count encoded?
2. Where is the null bitmap?
3. How are variable-length strings stored?
4. What's the exact byte layout?

### Type-Specific Questions
5. BIGINT: Little-endian? Big-endian?
6. DECIMAL: Fixed-point? String? How many bytes?
7. DATE: Epoch days? String? Format?
8. CHAR: Fixed-width? Null-terminated?

### Implementation Questions
9. Does BE always compress? Or only large results?
10. How does BE handle LIMIT? Send all rows or just N?
11. What happens with empty columns?

---

## How to Get Answers (CLAUDE.md #2)

### Option 1: Read C++ BE Source (Authoritative)
```bash
# Find PBlock encoding code in Doris C++ BE
grep -r "PBlock" /path/to/doris/be/src/
# Look for serialization/encoding logic
```

### Option 2: Capture Real Bytes (Empirical)
```rust
// Add logging in fetch_data()
eprintln!("PBlock bytes (hex): {:02X?}", row_batch_bytes);
// Run query, analyze output
```

### Option 3: Test with Java FE (Behavioral)
```java
// JDBC client gets same PBlock data
// Compare with Rust parsing
// Ensure identical results
```

**Recommended**: Combine all three approaches

---

## Success Criteria

### Minimum Viable (Can proceed with TPC-H)
- ✅ Parse BIGINT correctly
- ✅ Parse STRING correctly
- ✅ Parse DATE correctly
- ✅ Handle nulls
- ✅ Transpose to rows
- ✅ Match Java FE results for simple queries

### Complete (100% Parity)
- ✅ All Doris types supported
- ✅ Compression working
- ✅ Performance optimized
- ✅ All TPC-H queries return correct results
- ✅ Byte-for-byte match with Java FE

---

## Files to Modify

1. **fe-backend-client/src/pblock_parser.rs** - Main implementation
2. **fe-backend-client/src/pblock_parser_v2.rs** - New parser (if needed)
3. **fe-backend-client/src/lib.rs** - Use new parser
4. **fe-backend-client/Cargo.toml** - Add `snap` crate
5. **fe-backend-client/examples/test_pblock_parsing.rs** - Test suite

---

## Dependencies Needed

```toml
[dependencies]
snap = "1.1"  # Snappy compression/decompression
byteorder = "1.5"  # Endian-aware reading
```

---

## Next Steps

1. **Immediate**: Add detailed logging to see actual PBlock bytes
2. **Then**: Implement BIGINT decoder based on observed format
3. **Then**: Test with `SELECT l_orderkey FROM lineitem LIMIT 1`
4. **Iterate**: Add more types as needed for TPC-H queries

---

## CLAUDE.md Compliance Checklist

- [x] **Principle #1**: Parser is separate module, clean boundary
- [x] **Principle #2**: Using Java FE behavior as specification
- [x] **Principle #3**: Error handling and logging throughout
- [x] **Principle #4**: Wire format hidden behind Row/Value API

---

**Status**: Implementation plan complete, ready to execute
**Blocker**: Need to understand actual PBlock byte format
**Unblocks**: All TPC-H testing once complete

---
Session: claude/rust-fe-todos-migration-012mCiokw5gZWbgBtbTPkHJr
Date: 2025-11-18
Priority: CRITICAL for TPC-H goal
