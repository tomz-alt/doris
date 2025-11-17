# Fragment Generation Analysis - Java FE vs Rust FE

**Date**: 2025-11-17
**Status**: Analysis in progress
**Goal**: Understand why Java FE generates 3 fragments vs Rust FE's 1 fragment

## Java FE Fragment Structure (Decoded)

**Query**: `SELECT * FROM tpch_sf1.lineitem LIMIT 3`

**Payload**: 6196 bytes, 3 fragments, 17 backend instances

### Fragment Breakdown

```
Fragment [0] (fragment_id=2) - RESULT SINK
├─ Backend instances: 1 (backend_num=16)
├─ Scan ranges: 0
└─ Role: Collects results from Fragment[1] and returns to FE

Fragment [1] (fragment_id=1) - EXCHANGE/SHUFFLE
├─ Backend instances: 8 (backend_num=8-15)
├─ Scan ranges: 0
└─ Role: Receives data from Fragment[2], potentially shuffles/aggregates

Fragment [2] (fragment_id=0) - SCAN
├─ Backend instances: 8 (backend_num=0-7)
├─ Scan ranges: 3 (ONLY in backend_num=0)
│   ├─ tablet_id: 1763227464383
│   ├─ tablet_id: 1763227464385
│   └─ tablet_id: 1763227464387
├─ Other 7 instances (backend_num=1-7): 0 scan ranges
└─ Role: Scans tablets and sends data to Fragment[1]
```

### Key Observations

1. **Fragment IDs are reverse-ordered**: 2, 1, 0 (not 0, 1, 2)
2. **Backend numbering is sequential**: 0-16 across all fragments
3. **Scan ranges are concentrated**: Only Fragment[2], backend_num=0 has actual scan work
4. **Most backends are empty**: 16 out of 17 backends have NO scan ranges
5. **Pipeline structure**: SCAN (Fragment 2) → EXCHANGE (Fragment 1) → RESULT (Fragment 0)

## Rust FE Fragment Structure (Current)

**Query**: Same - `SELECT * FROM tpch_sf1.lineitem LIMIT 3`

**Payload**: 1368 bytes, 1 fragment, 1 backend instance

### Fragment Breakdown

```
Fragment [0] (fragment_id=0) - SCAN ONLY
├─ Backend instances: 1 (backend_num=None)
├─ Scan ranges: 3 (all in this single backend)
│   ├─ tablet_id: 1763227464383 (schema_hash="1232661058", version="3")
│   ├─ tablet_id: 1763227464385
│   └─ tablet_id: 1763227464387
├─ table_name: "lineitem" (populated, unlike Java)
└─ Role: Single-fragment execution (scan + return)
```

### Key Differences from Java FE

1. **No exchange layer**: Rust FE expects direct scan → result
2. **No backend numbering**: backend_num is None (vs 0-16 in Java)
3. **Better metadata**: schema_hash="1232661058", version="3", db_name="tpch_sf1", table_name="lineitem"
   - Java has: schema_hash="0", version="2", db_name="", table_name=None
4. **No replica distribution**: Only 1 backend vs 8 in Java's scan fragment
5. **Simpler structure**: Monolithic fragment vs multi-stage pipeline

## Hypotheses

### H1: BE Requires Multi-Fragment Pipeline (HIGH PROBABILITY)

**Evidence**:
- Java FE ALWAYS generates 3 fragments even for simple SELECT
- Fragment structure follows consistent pattern: SCAN → EXCHANGE → RESULT
- BE pipeline engine may expect this structure

**Implication**:
- Rust FE must implement 3-fragment generation
- Need to understand fragment dependencies and data flow

### H2: Empty Backends Are Placeholders (MEDIUM PROBABILITY)

**Evidence**:
- 16 out of 17 backends have NO scan ranges
- Backend numbering is sequential but sparse
- May be pre-allocated for parallel execution or failover

**Implication**:
- We may need to create "empty" backend instances
- Backend numbering scheme is important

### H3: Single-Fragment Works But Missing Other Metadata (LOW PROBABILITY)

**Evidence**:
- Rust FE scan ranges are MORE complete than Java (better schema_hash, version)
- BE accepts the fragment without errors
- Issue might be elsewhere (tuples, query options)

**Implication**:
- Could try fixing replica support first before multi-fragment
- Multi-fragment may not be strictly required

## Critical Questions

1. **Does BE require 3 fragments for pipeline execution?**
   - Or can it work with 1 fragment if properly configured?

2. **What is the role of Fragment[1] (exchange)?**
   - Data redistribution? Aggregation? Load balancing?

3. **Why 8 backend instances in Fragment[2] if only 1 has scan ranges?**
   - Replica-based distribution? Parallel scan capability?

4. **What determines fragment_id ordering (2, 1, 0)?**
   - Execution order? Dependency graph? Naming convention?

## Next Steps

### Investigation Tasks

1. **Read Java FE Source Code**:
   - [ ] `ThriftPlansBuilder.plansToThrift()` - Main fragment generation
   - [ ] `ThriftPlansBuilder.fragmentToThriftIfAbsent()` - Per-fragment logic
   - [ ] `ThriftPlansBuilder.instanceToThrift()` - Backend instance creation
   - [ ] `PipelineDistributedPlan` - Nereids distributed plan structure

2. **Analyze Fragment Dependencies**:
   - [ ] How are fragments linked? (parent/child relationships)
   - [ ] What is the data flow between fragments?
   - [ ] How does BE execute multi-fragment plans?

3. **Examine Rust FE Current Implementation**:
   - [ ] `src/planner/fragment_splitter.rs` - Current fragment generation
   - [ ] `src/be/thrift_pipeline.rs` - Thrift serialization
   - [ ] Where is fragment_id assigned?
   - [ ] How are backend instances created?

### Implementation Strategy (Tentative)

**Option A: Minimal Multi-Fragment (Conservative)**
- Create 3 fragments matching Java structure
- Keep scan ranges in Fragment[2], backend_num=0
- Add empty Fragment[1] (exchange) and Fragment[0] (result sink)
- Replicate backend instance pattern (17 instances, most empty)

**Option B: Simplified Multi-Fragment (Optimistic)**
- Create 3 fragments with minimal backends
- Fragment[2]: 1 backend with scan ranges
- Fragment[1]: 1 backend (exchange)
- Fragment[0]: 1 backend (result sink)
- Test if BE accepts reduced instance count

**Option C: Fix Replicas First (Alternative)**
- Keep 1-fragment structure
- Add tablet replica support (3 hosts per tablet)
- Test if replica support alone fixes 0-row issue
- Only add multi-fragment if still failing

## Comparative Summary

| Aspect | Java FE | Rust FE | Gap |
|--------|---------|---------|-----|
| **Fragments** | 3 (SCAN → EXCHANGE → RESULT) | 1 (SCAN only) | +2 |
| **Backend Instances** | 17 (1 + 8 + 8) | 1 | +16 |
| **Fragment IDs** | 2, 1, 0 | 0 | Different ordering |
| **Backend Numbering** | 0-16 sequential | None | No numbering |
| **Scan Ranges** | 3 (concentrated in 1 backend) | 3 (in single backend) | ✅ Same count |
| **Scan Metadata** | Incomplete (schema_hash="0") | Complete (schema_hash="1232661058") | Rust better |
| **Empty Backends** | 16 instances | 0 | +16 placeholders |
| **Payload Size** | 6196 bytes | 1368 bytes | +354% |

## Recommendations

**Immediate Priority**: Option A (Minimal Multi-Fragment)

**Rationale**:
1. Java FE's consistent 3-fragment pattern suggests BE expects it
2. Safest approach is to match Java structure exactly
3. Can optimize later once we have working end-to-end query

**Estimated Effort**: 8-12 hours
- Research Java FE code: 2-3 hours
- Design multi-fragment generation: 2-3 hours
- Implementation: 3-4 hours
- Testing and debugging: 1-2 hours

---

**Next Actions**:
1. Read Java FE `ThriftPlansBuilder` source code
2. Understand fragment dependency/linking mechanism
3. Design Rust FE multi-fragment generation
4. Implement and test

