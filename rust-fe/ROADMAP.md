# Rust FE Production Roadmap

**Version:** 1.0
**Date:** 2025-11-15
**Target:** Production-ready FE to replace Java FE
**Timeline:** 13 weeks (3 months)

---

## Executive Summary

**Current State:** Phase 1.5 - Architecture proven (30-40% complete)
**Main Blocker:** Mock protocol incompatible with production Doris BE
**Path Forward:** Implement real protocol (internal_service.proto + Thrift)
**Completion:** 7 milestones over 13 weeks

---

## Timeline Overview

```
Week  1-2:  Real Protocol Foundation      [Protobuf + Thrift + PBlock]
Week  3-4:  BEScanExec Implementation     [SQL gen + Arrow streaming]
Week  5-6:  TPC-H Validation (3 queries)  [Q1, Q3, Q6]
Week  7-8:  BE Catalog Integration        [Dynamic metadata]
Week  9-10: Distributed Planning          [Scan ranges + coordinator]
Week 11-12: Full TPC-H Suite              [All 22 queries]
Week 13:    Production Hardening          [Metrics + docs]
───────────────────────────────────────────────────────────────
Total: 13 weeks to production-ready
```

---

## Milestones

### M1: BE Accepts Thrift Fragments (Week 1)
**Acceptance Criteria:**
- ✅ internal_service.proto compiles
- ✅ Can serialize TExecPlanFragmentParams to bytes
- ✅ PBlockParser can decompress and parse simple types
- ✅ All unit tests pass

**Go/No-Go Decision:** If Thrift works → Week 2; else consider proxy/mock

---

### M2: First Query Returns Real Data (Week 2)
**Acceptance Criteria:**
- ✅ Can send real Thrift fragments to Doris BE
- ✅ BE executes query and returns PBlock
- ✅ Parser converts PBlock → Arrow RecordBatch
- ✅ End-to-end: `SELECT * FROM table` returns actual data

**Metrics:**
- Query latency: < 1 second
- Success rate: > 95%

---

### M3: TPC-H Q1/Q3/Q6 Match Java FE (Week 6)
**Acceptance Criteria:**
- ✅ Q1, Q3, Q6 return identical results to Java FE
- ✅ Differential tests pass (automated comparison)
- ✅ Latency: p95 < 2× Java FE
- ✅ Stability: 100 iterations without crash

**Performance Targets:**
| Query | Target Latency | Java FE Baseline | Max Ratio |
|-------|---------------|------------------|-----------|
| Q1    | <500ms        | Baseline         | <2.0×     |
| Q3    | <800ms        | Baseline         | <2.0×     |
| Q6    | <200ms        | Baseline         | <2.0×     |

---

### M4: Dynamic Metadata Working (Week 8)
**Acceptance Criteria:**
- ✅ Can fetch table/tablet metadata from BE
- ✅ Metadata cached and auto-refreshed
- ✅ Tablet selection logic functional
- ✅ `SHOW TABLES` returns BE tables (not hardcoded)

**Verification:**
```sql
SHOW DATABASES;           -- Should include BE databases
USE tpch;
SHOW TABLES;              -- Should match Java FE
DESC lineitem;            -- Should match BE schema
SELECT COUNT(*) FROM lineitem;  -- Should match BE row count
```

---

### M5: Distributed Joins Functional (Week 10)
**Acceptance Criteria:**
- ✅ Scan ranges generated with tablet metadata
- ✅ Partition pruning works (>90% tablets skipped)
- ✅ Multi-fragment coordination functional
- ✅ TPC-H Q3 (with JOIN) returns correct results

**Performance Targets:**
- Partition pruning: >90% effectiveness
- Parallel speedup: >2× on 4-core machine
- Q3 latency: <1.5× Java FE

---

### M6: All 22 TPC-H Queries Pass (Week 12)
**Acceptance Criteria:**
- ✅ All 22 TPC-H queries return correct results
- ✅ Differential tests: 100% pass rate
- ✅ Average latency: <1.5× Java FE
- ✅ No crashes or memory leaks in 100-iteration stress test

**TPC-H Coverage:** 22/22 queries (100%)

---

### M7: Production Deployment Ready (Week 13)
**Acceptance Criteria:**
- ✅ 48-hour stability test passes (no crashes)
- ✅ Performance within 20% of Java FE
- ✅ Observability: Prometheus metrics + structured logs
- ✅ Documentation: Deployment + troubleshooting guides
- ✅ Docker deployment working

**Production Readiness Checklist:**
- [ ] Functional: All features work
- [ ] Performance: <1.2× Java FE average latency
- [ ] Stability: 48+ hours without crash
- [ ] Observability: Metrics + logs
- [ ] Documentation: Complete
- [ ] Testing: >80% code coverage
- [ ] Security: No SQL injection, proper error handling

---

## Phase Details

### Phase 2: Core BE Integration (Weeks 1-6)

#### Week 1-2: Real Protocol Foundation
**Tasks:**
1. Update build.rs for internal_service.proto
2. Add Thrift support (thrift = "0.17")
3. Implement PBlock parser (SNAPPY/LZ4 decompression)
4. Update BackendClient for real protocol
5. Integration test with real BE

**Deliverables:**
- Compiling proto/Thrift bindings
- Working PBlock parser
- End-to-end query execution

#### Week 3-4: BEScanExec Implementation
**Tasks:**
1. Create SQL generator (DataFusion Expr → Doris SQL)
2. Implement BEScanExec ExecutionPlan
3. Create fragment builder
4. Wire up ArrowParser (pool.rs:74)

**Deliverables:**
- Functional BEScanExec
- Filter/projection pushdown
- Arrow streaming from BE

#### Week 5-6: TPC-H Validation
**Tasks:**
1. Implement Q1, Q3, Q6
2. Create differential test framework
3. Performance benchmarking
4. Bug fixes and tuning

**Deliverables:**
- 3 TPC-H queries passing
- Automated differential tests
- Performance report

---

### Phase 3: Distributed Planning (Weeks 7-10)

#### Week 7-8: BE Catalog Integration
**Tasks:**
1. Create BECatalog module
2. Implement tablet/replica metadata structures
3. Metadata sync from BE
4. Periodic refresh mechanism

**Deliverables:**
- Dynamic metadata from BE
- Tablet selection logic
- Auto-discovery of new tables

#### Week 9-10: Multi-Fragment Queries
**Tasks:**
1. Implement scan range builder
2. Create fragment coordinator
3. Partition pruning logic
4. Multi-fragment execution

**Deliverables:**
- Distributed query execution
- Partition pruning (>90%)
- JOIN queries working

---

### Phase 4: Production Hardening (Weeks 11-13)

#### Week 11-12: Full TPC-H Suite
**Tasks:**
1. Implement all 22 TPC-H queries
2. Full differential testing
3. Performance benchmarking
4. Optimization

**Deliverables:**
- 22/22 TPC-H queries passing
- Performance within 20% of Java FE
- No regressions

#### Week 13: Production Readiness
**Tasks:**
1. Add Prometheus metrics
2. Structured logging with tracing
3. Configuration externalization
4. Documentation (deployment, migration, troubleshooting)
5. 48-hour stability test
6. Load testing

**Deliverables:**
- Production-ready binary
- Docker image
- Complete documentation
- Stability validation

---

## Critical Path

```
Proto/Thrift (Week 1)
    ↓ [BLOCKS]
BE Client (Week 2)
    ↓ [BLOCKS]
BEScanExec (Week 3-4)
    ↓ [BLOCKS]
TPC-H Validation (Week 5-6)
    ↓
BE Catalog (Week 7-8) ← [Can start after Week 4]
    ↓
Distributed (Week 9-10)
    ↓ [BLOCKS]
Full TPC-H (Week 11-12)
    ↓ [BLOCKS]
Production (Week 13)
```

**Parallelization:**
- BE Catalog (Week 7-8) can start after BEScanExec basics (Week 4)
- Documentation can be written throughout
- Testing can be incremental

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Thrift integration complexity | Medium | High | 2-day spike to validate |
| PBlock format undocumented | High | High | Study BE source code |
| Performance regression | Low | Medium | Continuous benchmarking |
| Protocol version mismatch | Medium | High | Support multiple versions |
| Complex query failures | Medium | Medium | Start with simple queries |

**Contingency Plans:**
- Week 1 fails: Consider proxy or mock BE
- Week 6 latency >3×: Aggressive profiling/optimization
- Week 12 queries fail: +1 week buffer allowed

---

## Success Metrics

### Functional Completeness
- TPC-H coverage: 22/22 queries (100%)
- Differential test pass rate: 100%
- Feature parity: MySQL protocol, catalog, query execution

### Performance
- Average latency: <1.5× Java FE
- Peak latency: <2.0× Java FE
- Throughput: >90% of Java FE QPS

### Quality
- Code coverage: >80%
- Stability: 48h continuous operation
- Memory: No leaks, bounded growth

### Production Readiness
- Observability: Metrics + logs
- Documentation: Complete guides
- Deployment: Docker image working

---

## Next Steps (This Week)

**Immediate Actions:**
1. ✅ Review and approve roadmap
2. Create feature branch: `feature/production-be-protocol`
3. Schedule 2-day Thrift spike
4. Start Task 1.1: Update build.rs

**Week 1 Goals:**
- Complete M1 (BE accepts Thrift fragments)
- Validate technical approach
- Identify blockers early

**Decision Point (End Week 1):**
- **GO:** Thrift works → Week 2 (BE client)
- **NO-GO:** Blocked → pivot to proxy/mock

---

## Long-Term Vision

**3-Month Horizon:**
- Rust FE fully replaces Java FE for TPC-H workloads
- Production-grade stability and performance
- Clear path to TPC-DS and advanced features

**6-12 Month Vision:**
- Rust FE becomes primary FE for new deployments
- Feature parity with Java FE
- Additional frontends (pgwire, HTTP analytics)
- Advanced optimizations (cost-based, runtime filters)

---

**Document Status:** Production roadmap based on codebase research
**Confidence:** 80% (Thrift integration unproven, rest well-defined)
**Last Updated:** 2025-11-15
