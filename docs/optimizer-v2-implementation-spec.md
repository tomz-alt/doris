# Optimizer V2 Implementation Specification

> Status: normative implementation spec draft  
> Scope: delivery phases, module contracts, proof gates, benchmark acceptance, and no-debt rules for Optimizer V2 integration with runtime, operators, storage, and profiling.  
> Parent design: `docs/optimizer-v2-complete-design.md`.  
> Rule: demo success is not acceptance. Every hard gate must have reproducible raw evidence.

---

## 0. Definition of Done

Optimizer V2 is complete for this phase only when it can:

1. Plan native/local queries without using legacy memo, transpiler, Doris adapter, request-bin renderer, or shape compatibility.
2. Produce one `PhysicalDataflowPlan` containing `ReadView`, `ResourceEnvelope`, `TopologyEnvelope`, `AccessPathSpec`, `SplitSourcePlan`, `DynamicFilterSpec`, `AdaptivePoint`, `ProfileTagDictionary`, `PlanProof`, `PlanProvenance`, and `FallbackLedger`.
3. Lower the same `PhysicalDataflowPlan` to `LocalRuntimeGraph` and, where supported, `StageGraph` without changing query semantics.
4. Execute planned queries through Runtime-owned `QueryScope`, `Processor::step`, `MemoryPermit`, `IoPermit`, `OutputCredit`, cancellation/drain, and profile contracts.
5. Prove scan/operator/access-path quality on ClickBench, relational planning on TPC-H, join-order robustness on JOB, and integrated planning on TPC-DS.
6. Reject unsupported features explicitly rather than invoking silent repair or compatibility fallbacks.

---

## 1. Normative Language

```text
MUST / HARD:
  release-blocking requirement

SHOULD / TARGET:
  expected target; miss requires written root-cause and follow-up issue

MAY:
  optional behavior, never required for correctness
```

---

## 2. Implementation Phases

## O0 — Freeze Legacy and Add Observability

### Deliverables

```text
fallback ledger
planning trace
translator repair counter
request-bin oracle facade
native vs legacy path selector
native no-debt red-flag checks
```

### Hard gates

```text
legacy TPC-DS request-bin baseline remains stable
native path emits planning trace
native path records zero compatibility events
new product features are not added to legacy memo/transpiler/request-bin path
```

### Evidence

```text
optimizer_trace_legacy.json
optimizer_trace_native.json
dependency_wall_report.txt
fallback_ledger_report.txt
```

---

## O1 — QueryArena, SemanticView, NormalizedView, AlgebraIndex

### Deliverables

```text
fers-opt-core
fers-opt-bind
fers-algebra-index
fers-opt-normalize
basic rule kernel
structural validators
serial logical evaluator for small generated plans
```

### Scope

```text
SELECT/FROM/WHERE
projection
filter
basic scalar expressions
simple GROUP BY
LIMIT
basic CTE representation
subquery/apply representation
outer/semi/anti/mark barriers represented explicitly
```

### Hard gates

```text
no dependency on legacy memo/transpiler/Doris adapter
no unresolved names
no accidental free variables
field lineage complete
barrier tracking correct
normalized constructors remove identity project/filter true
```

### Tests

```text
unit tests for typed IDs and arena storage
proptest generated small plans
metamorphic rewrite tests
small-data differential evaluator
negative scope/free-variable tests
barrier legality tests
```

---

## O2 — AccessPathView and Local Physical Plan

### Deliverables

```text
fers-access-path
fers-lake-access
fers-mv-access skeleton
PruningPlan
SplitSourcePlan
LakeScanPath
TableScanPath
PhysicalDataflowPlan v0
LocalRuntimeGraph skeleton
```

### Scope

```text
TableScan
LakeScan local
Filter
Project
Aggregate basic
TopK basic
Segment/Parquet placeholder source adapters
synthetic lake metadata pruning
```

### Hard gates

```text
PruningPlan no-false-negative property tests pass
PhysicalDataflowPlan validates in O(nodes + exprs)
AccessPathView contains no Doris descriptor IDs
native simple scan/filter/project has memo_groups = 0
ClickBench simple subset plans without legacy path
```

---

## O3 — JoinRegion, JoinLegalityOracle, DPHyp, Runtime Filter Opportunities

### Deliverables

```text
fers-join-legality
fers-join-order
JoinHyperGraph
JoinLegalityOracle
brute-force join oracle for n <= 8
DPHypExact
RuntimeFilter candidate extraction
JOB harness skeleton
```

### Scope

```text
inner join
left join
semi/anti constraints
mark/apply barriers represented
broadcast hash join plan
partitioned hash join plan shape
runtime filter candidate edges
```

### Hard gates

```text
JoinLegalityOracle shared by all algorithms
n <= 8 brute-force valid-plan set is exact
DPHypExact == brute-force optimum under matching objective
invalid outer/semi/anti/mark reorder emitted = 0
TPC-H q3/q5/q10/q14 local plans validate
```

---

## O4 — Adaptive Join Algorithms and CostEnvelope

### Deliverables

```text
AdaptiveLinDP
DPconvCcap
GooLocalRepair
MPDP interface
CostEnvelope
StatsEvidence
StatsTrustLevel
risk-aware comparator
join algorithm routing trace
```

### Hard gates

```text
synthetic graph corpus routes to expected algorithm family
low-confidence estimates choose robust comparator
memory-sensitive dense graphs can choose DPconv Ccap
fallback plan is always legal
routing decision is traceable with reasons
```

### Required graph corpus

```text
chain
star
snowflake
cycle
clique
random sparse
random dense
outer/semi/anti/mark constrained
high-cardinality error variants
```

---

## O5 — Dynamic Filters, Predicate Transfer, and Compact Property Search

### Deliverables

```text
fers-dynamic-filter
fers-property-search
DynamicFilterSpec
DynamicFilterWaitPolicy
DynamicFilterProof
PropertySearch for distribution/exchange/ordering/RF/local-global-aggregate/lake-scan alternatives
AdaptivePoint v0
```

### Scope

```text
Bloom filter
ValueSet
MinMax
DictionarySet skeleton
RF producer/consumer edges
wait/apply-late/bypass policy
static property search after JoinPlan
```

### Hard gates

```text
RF-enabled result == RF-disabled result
RF false-negative injected test fails loudly
RF cost participates before final join/property choice
late RF arrival is profiled
TPC-H 22 local plan generation passes
TPC-DS star subset plans validate
```

---

## O6 — Runtime and Operator Integration

### Deliverables

```text
PhysicalOperatorSpec -> OperatorFactory
LocalRuntimeGraph lowering
ProfileTagDictionary
runtime feedback schema
EncodedBatch ABI freeze
Scan/Filter/Project/Agg/Join/TopK operator specs
```

### Hard gates

```text
operators allocate large memory only through MemoryPermit/PagePool
operators submit I/O only through RuntimeIo/ReadIntent
no operator creates private runtime/thread/queue
cancel/drain terminal-zero gates pass
forced-yield and forced-spill differential tests pass
profile tags map every runtime operator to plan node IDs
```

### Runtime integration invariants

```text
QueryScope owns lifecycle
TaskRegistry owns task state
IoRegistry owns physical I/O drain
ResourceLedger owns CPU/memory/I/O/output credits
Processor::step owns CPU work
Future::poll never performs heavy operator CPU
```

---

## O7 — Lake TPC-DS and Layout/Stats Integration

### Deliverables

```text
AMETA-backed column/file/row-group statistics adapter
LayoutProfile input
PruningPlan using file/row-group/page stats
source-versioned StatsEvidence
OptimizerFeedbackStore v0
lake TPC-DS representative subset harness
```

### Hard gates

```text
stats source version checked
stale stats cannot cause false-negative pruning
missing stats choose robust plan or less pruning
static pruning bytes avoided reported
runtime-filter pruning bytes avoided reported
lake fallback scan correctness preserved
```

---

## O8 — StageGraph and Doris Adapter

### Deliverables

```text
stage-lower
exchange plan
runtime filter route
split source route
Doris adapter subset
legacy oracle comparison harness
```

### Hard gates

```text
adapter failure does not block native path
PhysicalDataflowPlan remains adapter-independent
Doris adapter emits no silent repair
local-cluster distributed TPC-H subset executes or fails with typed unsupported feature
```

---

## O9 — Benchmark Integration and Release Evidence

### Deliverables

```text
ClickBench harness
TPC-H harness
JOB planning harness
TPC-DS harness
planning-only benchmark
E2E execution benchmark
Perfetto/fastrace trace export
operator attribution report
acceptance index
```

### Hard gates

```text
all hard correctness gates pass
all native no-debt gates pass
all required benchmark reports contain raw data and environment manifests
all unsupported features are typed failures
performance misses have per-query root-cause breakdown
```

---

## 3. No-Debt Gates

These gates are hard for native planning.

| ID | Gate | Threshold |
|---|---|---:|
| ND-01 | dependency on old memo/transpiler/Doris compat | 0 |
| ND-02 | shape rendering in native mode | 0 us |
| ND-03 | Doris adapter time in native mode | 0 us |
| ND-04 | compatibility events in native mode | 0 |
| ND-05 | silent repair fallback | 0 |
| ND-06 | request-bin ordering in native plan | 0 |
| ND-07 | legacy descriptor layout dependency | 0 |
| ND-08 | full-tree traversals for simple queries | <= 3 |
| ND-09 | memo groups for simple native queries | 0 |
| ND-10 | hot-path string rendering | 0 |

Fallback kinds:

```rust
pub enum FallbackKind {
    MissingSemanticFeature,
    MissingPhysicalOperator,
    MissingBackendCapability,
    DorisDisplayCompatibility,
    LegacyDescriptorLayout,
    SilentRepair,
}
```

Native optimizer must reject:

```text
DorisDisplayCompatibility
LegacyDescriptorLayout
SilentRepair
```

---

## 4. Planner Trace Schema

Every planning request emits a trace.

```json
{
  "query": "tpcds/q64",
  "mode": "native",
  "total_us": 8421,

  "parse_us": 310,
  "bind_us": 1180,
  "normalize_us": 1460,
  "algebra_index_us": 280,
  "access_path_us": 410,
  "join_order_us": 2310,
  "property_search_us": 800,
  "physical_build_us": 950,
  "freeze_us": 320,
  "validate_us": 140,

  "doris_adapter_us": 0,
  "shape_us": 0,
  "compat_events": 0,
  "memo_groups": 0,

  "alloc_count": 9412,
  "alloc_bytes": 1048576,

  "rules_matched": 123,
  "rules_fired": 31,
  "full_tree_traversals": 2,

  "join_algorithm": "DPHypExact",
  "join_relations": 8,
  "dynamic_filters": 3,
  "adaptive_points": 1,

  "fallbacks": []
}
```

---

## 5. Correctness Gates

## C-OPT-01 — Semantic normalization

HARD:

```text
no unresolved names
no accidental free variables
field lineage complete
outer/semi/anti/mark barriers explicit
NormalizedView validates structurally
```

## C-OPT-02 — Rewrite rule proof

Every semantic rule must have:

```text
structural validator
small-data differential test
metamorphic test
A/B benchmark
rule telemetry
```

Example invariant:

```text
apply(rule, plan) result == original result under bag semantics
```

## C-OPT-03 — Join legality

HARD:

```text
JoinLegalityOracle never permits invalid reorder
all join algorithms use same oracle
invalid outer/semi/anti/mark constrained plan count = 0
```

## C-OPT-04 — Join exactness for small graphs

For `n <= 8`:

```text
brute-force all binary join trees
filter by JoinLegalityOracle
compute exact cost
compare DPHyp / LinDP / MPDP / DPconv under matching objective
```

HARD:

```text
DPHypExact == brute-force optimum
DPconv Cmax/Ccap == brute-force constrained optimum where applicable
adaptive fallback emits a legal plan
```

## C-OPT-05 — Lake pruning no false negative

Generate random file, row-group, page metadata and predicates.

HARD:

```text
no false-negative pruning
false positives measured
residual predicate preserved
snapshot/delete semantics preserved
```

## C-OPT-06 — Dynamic filter correctness

HARD:

```text
RF enabled result == RF disabled result
false-negative RF is detected by oracle tests
late/bypassed RF preserves correctness
filter memory budget enforced
```

## C-OPT-07 — Physical plan linear validation

HARD:

```text
acyclic graph
root exists
child schema compatible
expr scope valid
field lineage valid
join keys valid
distribution/order properties valid
dynamic filter edges valid
resource envelope valid
split sources valid
```

Validation cost:

```text
O(nodes + exprs) on native hot path
```

---

## 6. Runtime and Operator Gates

| ID | Gate | HARD threshold |
|---|---|---|
| RT-01 | Operator private threads/runtimes/queues | 0 |
| RT-02 | Heavy work in unrestricted Future::poll | 0 |
| RT-03 | Large allocation without MemoryPermit | 0 |
| RT-04 | I/O submit without IoPermit/ReadIntent | 0 |
| RT-05 | Terminal query live children/tickets/IoOps/buffers/spill files | 0 |
| RT-06 | Operator profile tag missing | 0 |
| RT-07 | Forced-yield result divergence | 0 |
| RT-08 | Forced-spill result divergence | 0 |
| RT-09 | Cancel during RF wait/join build/spill hangs | 0 |
| RT-10 | OutputCredit bypass | 0 |

---

## 7. Benchmark Matrix

## 7.1 ClickBench

Purpose:

```text
scan/access-path/operator hot/cold proof
```

Required:

```text
43/43 parse, bind, lower
43/43 result equality
Parquet path
Native Segment path when bridge exists
hot Parquet profile
cold Parquet profile
hot Segment profile
```

Hard native no-debt gates:

```text
memo_groups = 0
compat_events = 0
doris_adapter_us = 0
shape_us = 0
silent repair = 0
```

Required profile per query:

```text
planning breakdown
source pruning
logical bytes
physical bytes
useful bytes
overread ratio
decode CPU
filter CPU
agg/distinct/topk CPU
memory wait
I/O wait
output time
profile overhead
```

## 7.2 TPC-H

Purpose:

```text
core relational planning and local execution proof
```

Required:

```text
22/22 parse/bind/lower
22/22 result equality where operators supported
join graph extraction
runtime filter metrics
hash join / aggregation / sort profile
```

Planning targets:

```text
simple scan/filter/project: p50 < 0.5 ms, p95 < 1.5 ms
simple agg/topn: p50 < 1 ms, p95 < 3 ms
TPC-H-style joins: p50 < 3-5 ms, p95 < 10 ms
```

## 7.3 JOB

Purpose:

```text
join-order robustness and cardinality-error resilience
```

Required:

```text
JoinHyperGraph extraction
legality oracle proof
algorithm routing trace
plan regret report
estimated vs actual cardinality report
```

Hard:

```text
valid plan emitted for every supported JOB query
illegal reorder count = 0
n <= 8 oracle cases exact
unsupported SQL is typed unsupported, not silent fallback
```

## 7.4 TPC-DS

Purpose:

```text
integration proof: unnesting, CTE, star/snowflake join, property search, dynamic filters, lake pruning, resource envelope
```

Stages:

```text
star subset
representative subset
99/99 parse/bind/normalize
99/99 correctness where operators supported
no-spill performance
spill performance
lake/Parquet performance
distributed StageGraph subset
```

Required report per query:

```text
join order
join algorithms
dynamic filters built/consumed/waited/bypassed
estimated vs actual rows
static pruning bytes avoided
dynamic pruning bytes avoided
object-store bytes and requests
peak memory and spill bytes
operator CPU
ready queue wait
memory/I/O/output wait
adaptive decisions
```

---

## 8. Performance Gates

Planning results must be measured with warm catalog/stats and native mode unless explicitly testing legacy adapter.

| ID | Metric | HARD / TARGET |
|---|---|---|
| P-PLAN-01 | simple scan/filter/project planning | TARGET p50 < 0.5 ms, p95 < 1.5 ms |
| P-PLAN-02 | simple agg/topn planning | TARGET p50 < 1 ms, p95 < 3 ms |
| P-PLAN-03 | TPC-H-style join planning | TARGET p50 < 3-5 ms, p95 < 10 ms |
| P-PLAN-04 | TPC-DS SF1 planning-only | TARGET median < 10 ms, p95 < 25 ms |
| P-PLAN-05 | rewrite-only simple queries | TARGET < 300 us |
| P-PLAN-06 | hot-path physical validation | HARD O(nodes + exprs) |
| P-PLAN-07 | full-tree traversals simple native | HARD <= 3 |
| P-PROF-01 | basic optimizer/runtime profile overhead | HARD <= 2%, TARGET <= 1% |
| P-RF-01 | repeated negative-benefit RFs | MUST be bypassed or re-costed |
| P-LAKE-01 | pruning false negatives | HARD 0 |
| P-LAKE-02 | adaptive read/pruning regression on unfavorable workload | TARGET <= 5% |

---

## 9. Evidence Package

Every accepted run includes:

```text
git SHA
dependency lock hash
hardware/kernel/storage/network manifest
optimizer/runtime config
query suite version
dataset generation command
source table versions
raw result hashes
raw histograms
planning traces
runtime profiles
Perfetto/fastrace traces when enabled
operator attribution reports
pass/fail gate index
root cause for every miss
```

---

## 10. PR Checklist

Every optimizer PR must answer:

```text
Does this touch old CascadesMemo?
Does this add a fallback? Which FallbackKind?
Does this run in native hot path?
Does this render strings in hot path?
Does this add a whole-tree traversal?
Does this add a side table?
Does it have rule telemetry?
Does it have property/differential tests?
Does it affect Doris adapter only or product optimizer?
Does it change planning latency by phase?
Does it add profile tags?
Does runtime/operator/storage consume the new field?
Does PhysicalDataflowPlan validation remain O(nodes + exprs)?
```

---

## 11. Release Blockers

A release candidate is blocked by:

```text
wrong query result
illegal join reorder
false-negative pruning
RF false negative
native silent repair
native legacy dependency
unbounded planner memory growth
unbounded full-tree traversal loop
Doris adapter use in native planning
operator resource contract bypass
host OOM
cancel/drain hang
missing raw benchmark evidence
benchmark result without profile decomposition
unsupported feature silently falling back
```

---

## 12. Implementation Order

```text
1. dependency wall + planning trace + fallback ledger
2. QueryArena + SemanticView + AlgebraIndex
3. normalized constructors and basic rewrite proof
4. AccessPathView + PruningPlan + PhysicalDataflowPlan v0
5. LocalRuntimeGraph skeleton
6. JoinHyperGraph + JoinLegalityOracle + brute-force oracle
7. DPHypExact + TPC-H/JOB seed cases
8. DynamicFilterSpec + RF proof + consumer scan gates
9. CostEnvelope + StatsEvidence + adaptive join router
10. compact property search
11. operator factory + profile tags + runtime contract
12. ClickBench correctness and access-path/operator profiles
13. TPC-H 22 local planning and execution
14. JOB robustness campaign
15. TPC-DS star subset
16. StageGraph and Doris adapter subset
17. lake TPC-DS representative subset with AMETA stats/pruning
18. acceptance evidence package
```

---

## 13. Final Acceptance

Final acceptance for this phase requires:

```text
all no-debt gates pass
all correctness gates pass
all runtime/operator integration gates pass
ClickBench correctness baseline passes
TPC-H supported-query baseline passes
JOB join-order proof passes
TPC-DS star subset validates
planning traces are complete
performance results are profile-explained
legacy path remains available as oracle but is absent from native hot path
```
