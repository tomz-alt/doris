# Rust FE Quick Reference

**Last Updated:** 2025-11-16

---

## TL;DR - Critical Facts

1. **Current State:** Phase 1.5 - Metadata working, query execution mocked
2. **Protocol Gap:** Using simplified mock protocol, NOT real Doris
3. **Next Decision:** Mock BE (3 days) vs Real Protocol (2-4 weeks)
4. **FE Role:** Metadata coordinator only, does NOT read segment files
5. **Architecture:** Sound and follows AGENTS.md principles

---

## What Works ✅

- MySQL protocol server (custom implementation)
- Query queuing with resource control
- BE client pool with gRPC
- Arrow parser (complete, just not wired up)
- TPC-H metadata (hardcoded schemas)
- Clean architecture boundaries

---

## What Doesn't Work ❌

- Real Doris BE protocol (using mock SQL-based)
- Dynamic metadata sync from BE
- Query execution (results are mocked)
- BEScanExec (returns NotImplemented)
- Actual data from BE (just byte count)

---

## Quick Commands

```bash
# Build
cargo build --release

# Run server
cargo run --release

# Test MySQL connection
mysql -h 127.0.0.1 -P 9030 -u root -D tpch

# Queries that work:
SHOW DATABASES;
SHOW TABLES;
DESCRIBE lineitem;

# Queries that DON'T work:
SELECT * FROM lineitem LIMIT 10;  # Returns mock "Received N bytes"
```

---

## Key Files

**Entry Point:**
- `src/main.rs` - Server startup
- `src/query/executor.rs:99` - execute_sql() core API

**BE Communication:**
- `src/be/client.rs` - gRPC client (mock protocol)
- `src/be/pool.rs:74` - Result parsing (TODO: wire up ArrowResultParser)

**Parser (Ready but not used):**
- `src/planner/arrow_parser.rs` - Complete Arrow → QueryResult

**Protocol:**
- `proto/backend_service.proto` - Mock (SQL-based)
- `proto/internal_service.proto` - Real Doris (NOT compiled)

---

## Architecture Hierarchy

```
Table
  └─ Partition (date range)
       └─ Tablet (logical, 1-10 GB, managed by FE)
            └─ Segment (physical file, 128-256 MB, managed by BE)

FE knows: Tablets
BE knows: Segments

FE does NOT read segment files!
```

---

## Protocol Reality

### Current (Mock)
```protobuf
message PExecPlanFragmentRequest {
    string query = 3;  // ← SQL string (custom!)
}
```
**Works with:** Mock/test BE only

### Real Doris
```protobuf
message PExecPlanFragmentRequest {
    bytes request = 1;  // ← Serialized fragment
}
```
**Works with:** Production Doris BE

**Gap:** Need fragment serialization (2-4 weeks)

---

## FE vs BE Responsibilities

| Task | FE | BE |
|------|----|----|
| Parse SQL | ✅ | |
| Plan query | ✅ | |
| Partition pruning | ✅ | |
| Fragment generation | ✅ | |
| Read segment files | | ✅ |
| Check bloom filters | | ✅ |
| Check bitmap indexes | | ✅ |
| Execute query | | ✅ |

**FE = Planner, BE = Executor**

---

## Doris Segment Indexes

From `proto/segment_v2.proto`:

1. **Zone Maps** (min/max per segment/page)
   - Prunes 90-99% of segments
   - Fastest: in-memory check

2. **Bloom Filters** (point lookups)
   - 10 bytes per 1000 rows
   - Negative lookup = 0 disk reads

3. **Bitmap Index** (low-cardinality)
   - 1 bit per row (compressed)
   - Fast: `WHERE country IN (...)`

4. **Short Key Index** (sorted data)
   - B-tree for range queries
   - Every 1024th row indexed

**Result:** 1000-10,000× faster than full scan

---

## Doris vs Parquet

| | Doris Segment | Parquet |
|---|---|---|
| **Indexes** | 4 types | 1 type (zone maps) |
| **Overhead** | 5-10% | 1-2% |
| **Speed** | 10-50× faster | Baseline |
| **Use** | Doris DB | Data lake |
| **Portability** | Doris only | Any engine |

**Summary:** Doris = "Parquet + database indexes"

---

## AGENTS.md Compliance Score: 85%

✅ **Principle 1:** Clean boundary (100%)
✅ **Principle 2:** Java FE as spec (90%)
✅ **Principle 3:** Hide transport (100%)
✅ **Principle 4:** Resource control (90%)

**Gaps:**
- Automated differential testing
- Prometheus metrics
- Real protocol implementation

---

## Next Steps (Choose One)

### Option A: Mock BE (PoC)
**Time:** 3 days
**Outcome:** Working demo
**Steps:**
1. Build mock BE accepting SQL
2. Return Arrow IPC format
3. Wire up ArrowResultParser

### Option B: Real Protocol (Production)
**Time:** 2-4 weeks
**Outcome:** Production-ready
**Steps:**
1. Switch to internal_service.proto
2. Implement fragment serialization
3. Implement PBlock parser
4. TPC-H differential tests

---

## Common Misconceptions ❌

**Myth:** FE reads segment files
**Reality:** FE only has metadata, BE reads files

**Myth:** Need to implement bloom filters in Rust
**Reality:** BE (C++) already has this

**Myth:** DataFusion needs to generate SQL
**Reality:** Should generate fragments (or use SQL as temporary hack)

**Myth:** segment_v2.proto needed in FE
**Reality:** Only BE uses this (storage format)

**Myth:** ArrowResultParser doesn't exist
**Reality:** It exists (283 lines), just not wired up!

---

## Troubleshooting

**Q: MySQL queries return "Received N bytes"**
A: Result parsing is mocked. Wire up ArrowResultParser in pool.rs:74

**Q: SELECT queries don't work**
A: BEScanExec returns NotImplemented. Need to implement or use SQL passthrough.

**Q: Can't connect to real Doris BE**
A: Using mock protocol. Real BE needs internal_service.proto + fragments.

**Q: Where are bloom filters?**
A: In BE's C++ code, not Rust. FE doesn't need them.

**Q: How to get table metadata?**
A: Currently hardcoded. Need to implement metadata sync (GetTabletMeta RPC).

---

## Performance Expectations

**With mock protocol (current):**
- Metadata queries: Fast (in-memory)
- Data queries: Mocked (no real execution)

**With real protocol (future):**
- Point query: ~10ms (bloom filter pruning)
- Range scan: ~50-500ms (zone map pruning)
- Full scan: ~5-10s (columnar + compression)
- TPC-H Q6: ~100-200ms (comparable to Java FE)

---

## Contact / Questions

See `TECHNICAL_FINDINGS.md` for:
- Detailed analysis
- Architecture diagrams
- Implementation roadmap
- Protocol specifications
- Doris internals

See `AGENTS.md` for:
- Design principles
- Architectural guidelines
- Testing philosophy

---

**Quick Start:**
1. Read this file (5 min)
2. Read TECHNICAL_FINDINGS.md (30 min)
3. Read AGENTS.md (10 min)
4. Decide: Mock or Real
5. Start implementing
