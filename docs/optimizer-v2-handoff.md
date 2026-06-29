# Optimizer V2 Handoff: Rust Rewrite, Doris Compatibility, and Lakehouse-Native Planning

## Executive summary

This handoff captures the current design discussion, code status quo, expected product direction, and migration plan for the Rust optimizer rewrite.

The high-level decision is:

> Do not keep evolving the current Doris-compatible Cascades/Memo/transpiler path as the product optimizer. Freeze it as a legacy oracle and optional compatibility adapter. Build a new lakehouse-native optimizer core with a hard dependency wall, query-local arena, low-overhead hot path, typed IR boundaries, and independent validation.

The current path remains valuable as:

```text
LegacyDorisPlanner
  -> request-bin oracle
  -> Java/Nereids compatibility reference
  -> regression baseline
  -> optional Doris adapter fallback
```

The new product optimizer should be:

```text
OptimizerV2
  -> embedded/local optimizer
  -> lakehouse-native optimizer
  -> distributed optimizer
  -> source of CanonicalPlanIR / PhysicalDataflowPlan
```

Most importantly:

> Native/local planning must not run Doris request-bin compatibility, shape rendering, Java identity emulation, descriptor repair, or old CascadesMemo side-table logic.

---

## Current status quo

The serious Rust production path today is approximately:

```text
SQL text
 -> frontend parser AST
 -> CascadesBinder
 -> CascadesMemo logical groups/scalars
 -> Java-style rewrite pipeline
 -> stats derivation
 -> DPHyp join enumeration
 -> Cascades property optimizer
 -> materialized physical winner tree
 -> runtime filter generation
 -> physical contract / PlanTranslator
 -> fragment graph / thrift request
 -> BE or request-bin fixture
```

The parity-critical crates/modules are mostly:

```text
fers-query-engine
fers-binder
fers-plan-ir
fers-cascades
fers-runtime-filter
fers-physical-contract
fers-transpiler
```

The current system has achieved valuable milestones:

```text
TPC-DS request-bin: near full parity / 99-query class parity depending on baseline
TPC-DS SF1000 shape: full shape parity against the current baseline
Rust engine shape parity with Java for broad TPC-DS coverage
request-bin fixture pipeline usable as an oracle
```

But the current architecture has accumulated compatibility debt:

```text
binder carries Java identity semantics
memo carries optimizer + Java compatibility + lowering + diagnostics state
rewrite order is fragile
shape has renderer-level compatibility logic
transpiler still repairs or infers too much
physical contract validates after the fact instead of being the optimizer product
DPHyp path can copy out of memo, rewrite, release memo, rebuild memo, and rederive stats
```

The current `CascadesMemo` is no longer just a memo. It is effectively a blackhole object that mixes:

```text
search state
logical alternatives
physical alternatives
Java ExprId/ObjectId/relation identity
scalar contracts
scalar text
statistics
cost/property tables
runtime filter state
distribution metadata
descriptor-related facts
shape/debug compatibility
statement-scope counters
CTE/MV state
translator repair assumptions
```

Small plan-shape changes therefore risk breaking invisible contracts:

```text
ExprId / Java ExprId mapping
output slot order
slot name / original column
type / nullable
tuple id / slot id
project output contract
join build/probe side
hash conjunct visible scope
fragment boundary output exprs
runtime filter source/target
CTE producer/consumer mapping
distribution property
```

This explains why request-bin similarity was achieved by adding many fallback paths, but the resulting planner feels unsafe to use or evolve.

---

## What must stop

Stop adding new product behavior to:

```text
old CascadesMemo
query-engine shape renderer
legacy physical contract repair
Doris request-bin compatibility path
PlanTranslator silent repair
frontend AST MV rewrite path
```

Stop treating EXPLAIN SHAPE or request-bin similarity as proof of executable correctness.

They remain important compatibility gates, but not the product optimizer contract.

---

## New design principle

The new optimizer must be compiler-style and latency-first.

Do not implement the design as a chain of fully materialized artifacts on every query:

```text
SemanticGraph
 -> NormalizedRelGraph
 -> AccessPathGraph
 -> JoinPlan
 -> PhysicalDataflowPlan
 -> StageGraph
 -> Adapter
```

That would be too slow. DuckDB-style local planning is fast because it avoids global memo creation and avoids heavyweight phase materialization for simple queries.

Instead, implement:

```text
QueryArena
  + SemanticView
  + NormalizedView
  + AccessPathView
  + JoinRegionView
  + PhysicalView
```

Only two objects should exist on the hot path:

```text
1. QueryArena during planning
2. Frozen PhysicalPlan / PhysicalDataflowPlan after planning
```

Everything else is either:

```text
a view over the arena
or a debug/CI snapshot
or an optional adapter product
```

---

## Hard dependency wall

OptimizerV2 crates must not depend on:

```text
old CascadesMemo
fers-cascades legacy memo internals
fers-transpiler
fers-doris-compat
legacy descriptor repair
legacy request-bin renderer
```

Allowed direction:

```text
optimizer-v2 -> physical-dataflow -> local-lower
optimizer-v2 -> physical-dataflow -> stage-lower
optimizer-v2 -> physical-dataflow -> doris-adapter
```

Forbidden direction:

```text
optimizer-v2 -> doris-adapter
optimizer-v2 -> old memo
physical-dataflow -> doris-adapter
join-order -> property-search
normalize -> join-order
```

CI should enforce this:

```bash
cargo xtask check-opt-v2-deps
cargo deny check
```

Example forbidden dependency list:

```toml
[workspace.metadata.opt_v2.forbidden_deps]
crates = [
  "fers-cascades",
  "fers-transpiler",
  "fers-doris-compat",
  "fers-physical-contract-legacy"
]
```

---

## Proposed crate layout

```text
optimizer-v2/
  fers-opt-core
  fers-opt-bind
  fers-algebra-index
  fers-opt-normalize
  fers-rule-kernel
  fers-access-path
  fers-lake-access
  fers-mv-access
  fers-cardinality
  fers-join-legality
  fers-join-order
  fers-property-search
  fers-physical-dataflow
  fers-plan-validate
  fers-local-lower
  fers-stage-lower
  fers-doris-adapter
  fers-opt-bench
  fers-opt-fuzz
```

Dependency direction:

```text
bind
 -> algebra-index
 -> normalize
 -> access-path
 -> cardinality
 -> join-legality
 -> join-order
 -> property-search
 -> physical-dataflow
 -> local-lower / stage-lower / doris-adapter
```

---

## Hot-path data model

Use a query-local arena.

```rust
pub struct QueryArena {
    pub rels: PrimaryMap<RelNodeId, RelNode>,
    pub exprs: PrimaryMap<ExprId, Expr>,
    pub fields: PrimaryMap<FieldId, Field>,

    pub traits: PrimaryMap<RelNodeId, LogicalTraits>,
    pub stats: PrimaryMap<RelNodeId, StatsEstimate>,

    pub access_paths: PrimaryMap<AccessPathId, AccessPath>,
    pub phys: PrimaryMap<PhysNodeId, PhysNode>,

    pub child_edges: Vec<RelNodeId>,
    pub expr_edges: Vec<ExprId>,

    pub scratch: QueryScratch,
}
```

Recommended Rust implementation patterns:

```text
typed IDs: GroupId(u32), ExprId(u32), FieldId(u32), NodeId(u32)
dense storage: Vec<T>, Box<[T]>, PrimaryMap, SmallVec, ArrayVec
query-local scratch: bumpalo or custom arena
bitsets: u64/u128 RelSet for small joins, fixedbitset/bitvec for larger sets
hashing: rustc-hash/hashbrown for query-local maps; stable hash only for fingerprints
strings: interned, never rendered in hot path
```

Avoid:

```text
Arc<dyn PlanNode>
Box<dyn Expr>
HashMap everywhere
string rendering in hot path
full global memo for simple queries
Doris adapter in native planning
shape rendering in native planning
```

---

## Product IRs and views

### SemanticView / SemanticGraph

Output of binding.

Contains:

```text
relation IDs
field IDs
expression IDs
types
nullability
scope
CTE definitions
subqueries / apply nodes
logical operators
```

Must not contain:

```text
Doris slot IDs
Doris tuple IDs
Java ExprIds
request-bin order
fragment IDs
backend locations
```

### NormalizedView / NormalizedRelGraph

After semantic normalization.

Guarantees:

```text
no unresolved names
no accidental free variables
subqueries either unnested or explicitly represented
CTE maps explicit
field lineage explicit
outer/semi/anti/mark barriers explicit
```

Holistic unnesting belongs here, before join ordering and before property Cascades.

### AlgebraIndex

This is the rewrite maintainability layer.

It answers:

```text
where is this field produced?
where is it consumed?
can this predicate move below this join?
does this node cross an outer/semi/anti/mark barrier?
what is the lowest common ancestor of this expression's inputs?
```

Sketch:

```rust
pub struct AlgebraIndex {
    pub producer_of_field: PrimaryMap<FieldId, RelNodeId>,
    pub consumers_of_field: PrimaryMap<FieldId, SmallVec<[RelNodeId; 4]>>,
    pub parent: PrimaryMap<RelNodeId, Option<RelNodeId>>,
    pub children: PrimaryMap<RelNodeId, SmallVec<[RelNodeId; 2]>>,
    pub barriers: PrimaryMap<RelNodeId, BarrierSet>,
    pub free_vars: PrimaryMap<RelNodeId, FieldSet>,
}
```

### AccessPathView / AccessPathGraph

Lake-native starts here.

For each base relation, enumerate:

```text
TableScan
LakeScan
IndexScan
MVScan
FragmentCache
```

Example:

```rust
pub enum AccessPath {
    TableScan(TableScanPath),
    LakeScan(LakeScanPath),
    IndexScan(IndexScanPath),
    MVScan(MVScanPath),
    FragmentCache(FragmentCachePath),
}

pub struct LakeScanPath {
    pub table: TableId,
    pub snapshot: SnapshotId,
    pub projection: FieldSet,
    pub predicate_domain: PredicateDomain,
    pub pruning: PruningPlan,
    pub split_source: SplitSourcePlan,
    pub dynamic_filter_consumers: SmallVec<[DynamicFilterId; 4]>,
    pub metadata_cache_policy: MetadataCachePolicy,
    pub estimate: ScanEstimate,
}
```

### JoinPlan

Only the join skeleton.

Contains:

```text
join tree
join predicates
join legality proof
estimated rows/confidence
runtime-filter opportunities
risk envelope
```

Does not contain:

```text
exchange
fragment
Doris node
descriptor tuple
backend id
```

### PhysicalDataflowPlan

The product physical plan.

Contains:

```text
operators
schemas
field lineage
ordering/distribution properties
exchange nodes
runtime filter edges
resource envelope
split source
cache policy
```

Local:

```text
PhysicalDataflowPlan
  -> LocalRuntimeGraph
```

Distributed:

```text
PhysicalDataflowPlan
  -> StageGraph
```

Doris compatibility:

```text
PhysicalDataflowPlan
  -> DorisExecutableContract
```

---

## Rewrite design

### Do not use global fixpoint traversal for hot path

Do not do:

```text
for rule in rules:
  traverse whole tree
  apply if match
repeat until fixed point
```

Do:

```text
normalized constructors
dirty worklist
node-kind indexes
incremental AlgebraIndex updates
```

### Normalized constructors

```rust
impl NormBuilder<'_> {
    pub fn filter(&mut self, input: RelNodeId, pred: ExprId) -> RelNodeId {
        let pred = self.normalize_predicate(pred);

        if self.exprs.is_true(pred) {
            return input;
        }

        if let Some(new_input) = self.try_push_filter(input, pred) {
            return new_input;
        }

        self.intern_rel(RelNode::Filter { input, pred })
    }

    pub fn project(&mut self, input: RelNodeId, exprs: ExprList) -> RelNodeId {
        let exprs = self.normalize_projection(exprs);

        if self.is_identity_project(input, exprs) {
            return input;
        }

        if let Some(merged) = self.try_merge_project(input, exprs) {
            return merged;
        }

        self.intern_rel(RelNode::Project { input, exprs })
    }
}
```

### Rule classification

```text
Always-on cheap:
  constant folding
  filter merge
  project merge
  column pruning

Costed:
  eager aggregation
  groupjoin
  CTE materialize
  MV rewrite
  join-dependent dynamic filter

Debug/compat only:
  Doris shape canonicalization
  request-bin ordering
  Java ExprId ordering
```

Debug/compat rules are not allowed in native planning.

---

## Join optimization design

### Join legality first

Build a shared legality oracle before cost or DP.

```rust
pub struct JoinHyperGraph {
    pub rels: RelSet,
    pub edges: Vec<JoinHyperEdge>,
    pub operators: Vec<JoinOperator>,
}

pub struct JoinHyperEdge {
    pub left_required: RelSet,
    pub right_required: RelSet,
    pub predicate_terms: SmallVec<[PredicateId; 2]>,
    pub source_join: JoinOpId,
}

pub trait JoinLegalityOracle {
    fn can_join(&self, left: RelSet, right: RelSet) -> Option<ApplicableJoin>;
}
```

All algorithms must use the same oracle:

```text
DPHyp
Adaptive LinDP
MPDP
DPconv
GOO + local repair
```

### Adaptive join algorithm router

```rust
pub enum JoinAlgorithm {
    BruteForceOracle,
    DPHypExact,
    AdaptiveLinDP,
    MPDP,
    DPconvCcap,
    GooLocalRepair,
    ConstrainedDPHyp,
}
```

Router:

```rust
pub fn choose_join_algorithm(
    graph: &JoinHyperGraph,
    oracle: &dyn JoinLegalityOracle,
    stats: &StatsContext,
    budget: &JoinBudget,
    objective: JoinObjective,
) -> JoinAlgorithm {
    if graph.rel_count() <= 8 {
        return JoinAlgorithm::BruteForceOracle;
    }

    if graph.has_outer_or_mark_constraints() {
        return JoinAlgorithm::ConstrainedDPHyp;
    }

    if graph.connected_subgraph_count_bounded(budget.cc_limit) <= budget.cc_limit {
        return JoinAlgorithm::DPHypExact;
    }

    if graph.is_sparse_or_tree_like() {
        return JoinAlgorithm::AdaptiveLinDP;
    }

    if graph.is_dense()
        && graph.rel_count() >= 17
        && objective.memory_sensitive()
    {
        return JoinAlgorithm::DPconvCcap;
    }

    if graph.rel_count() >= budget.mpdp_min_relations
        && budget.allow_parallel_join_dp
    {
        return JoinAlgorithm::MPDP;
    }

    JoinAlgorithm::GooLocalRepair
}
```

Guidance:

```text
DPHyp:
  small/sparse exact

Adaptive LinDP:
  large tree/star/snowflake

DPconv Ccap:
  dense + memory-sensitive

MPDP:
  large analytical join region with parallel budget

GOO + repair:
  huge or budget-constrained
```

---

## Property search

Do not use Cascades as the entire optimizer universe.

Use compact property search only for:

```text
distribution
exchange
ordering
local/global aggregate
runtime filter
lake scan implementation alternatives
```

Property search consumes JoinPlan.

It does not:

```text
do unnesting
decide SQL semantics
do Java compatibility
generate Doris descriptors
render shape
```

---

## Cost and risk model

Use cost envelope, not one scalar.

```rust
pub struct CostEnvelope {
    pub expected_rows: Interval<u64>,
    pub expected_bytes: Interval<u64>,

    pub cpu_ns: Interval<u64>,
    pub network_bytes: Interval<u64>,
    pub object_store_bytes: Interval<u64>,
    pub object_store_requests: Interval<u64>,

    pub peak_memory: Interval<u64>,
    pub spill_risk: f32,

    pub confidence: f32,
    pub worst_case_regret: Option<f64>,
}
```

Hard policies:

```text
no unindexed nested-loop for ordinary equi-joins
hash table build side must be runtime-resizable
broadcast requires memory envelope
low-confidence estimates prefer robust plans
DPconv Ccap may be selected under memory pressure
```

---

## Native hot-path planner

Default local/embedded/TPC-H/TPC-DS SF1 planning should be:

```text
parse
 -> bind into QueryArena
 -> normalize constructors
 -> derive traits/stats once
 -> enumerate access paths
 -> optimize regions
 -> build physical nodes
 -> freeze physical plan
```

Pseudo-code:

```rust
pub fn plan_native_fast(req: PlanRequest) -> Result<Arc<PhysicalPlan>> {
    let mut arena = QueryArena::with_capacity(req.estimated_size());

    let root = bind_into_arena(&mut arena, req.sql, req.catalog)?;
    let root = normalize_in_place(&mut arena, root)?;

    derive_traits_and_stats_once(&mut arena, root, req.stats)?;
    attach_access_paths(&mut arena, root, req.catalog, req.lake_metadata)?;

    let physical_root = optimize_regions(&mut arena, root, req)?;
    let plan = freeze_physical_plan(&mut arena, physical_root)?;

    Ok(Arc::new(plan))
}
```

No:

```text
Doris adapter
request-bin
shape renderer
distributed stage graph
legacy descriptor
full Cascades memo
global validators
```

unless explicitly requested.

---

## Verification design

### Compile-time proof

```text
optimizer-v2 crates cannot import legacy memo / transpiler / doris compat
```

### Rule proof

Each semantic rule gets:

```text
structural validator
small-data differential test
metamorphic test
A/B benchmark
rule telemetry
```

Example:

```rust
proptest! {
    #[test]
    fn push_filter_through_project_is_semantic_preserving(case in arb_small_plan()) {
        let before = case.plan.clone();
        let after = apply_rule(PushFilterThroughProject, before.clone())?;

        validate_normalized_graph(&after)?;

        let r1 = eval_logical(&before, &case.data)?;
        let r2 = eval_logical(&after, &case.data)?;

        prop_assert_eq!(bag_equal(r1, r2), true);
    }
}
```

### Join proof

For `n <= 8`:

```text
brute-force all binary join trees
filter by JoinLegalityOracle
compute exact cost
compare with DPHyp / LinDP / MPDP / DPconv under matching objective
```

Graph corpus:

```text
chain
star
snowflake
cycle
clique
random sparse
random dense
outer/semi/anti/mark constrained
```

Assertions:

```text
DPHyp exact == brute force
DPconv Cmax == brute force Cmax
Ccap == brute force constrained Cout
JoinLegalityOracle never produces invalid plans
Adaptive fallback emits valid plan
```

### Lake pruning proof

Generate random file metadata and predicates.

Assert:

```text
no false-negative pruning
false positives measured
residual predicate preserved
snapshot/delete semantics preserved
```

### Physical plan proof

Hot path final validation should be `O(nodes + exprs)`:

```rust
pub fn validate_physical_dataflow(plan: &PhysicalDataflowPlan) -> Result<()> {
    validate_graph_is_acyclic(plan)?;
    validate_root_exists(plan)?;
    validate_child_schema_compatibility(plan)?;
    validate_expr_scope(plan)?;
    validate_field_lineage(plan)?;
    validate_join_keys(plan)?;
    validate_distribution_properties(plan)?;
    validate_runtime_filter_edges(plan)?;
    validate_resource_envelope(plan)?;
    validate_split_sources(plan)?;
    Ok(())
}
```

Deep validators run in CI/debug, not every planning request.

---

## Benchmark design

### Native planning benchmark command

```bash
fers-opt-bench \
  --suite tpcds \
  --scale sf1 \
  --mode planning-only \
  --profile native \
  --warm-catalog \
  --warm-stats \
  --no-render \
  --no-doris-adapter \
  --json out.json
```

### Trace schema

```json
{
  "query": "tpcds/q64",
  "total_us": 8421,

  "parse_us": 310,
  "bind_us": 1180,
  "normalize_us": 1460,
  "algebra_index_us": 280,
  "access_path_us": 410,
  "join_order_us": 2310,
  "physical_build_us": 950,
  "freeze_us": 320,

  "doris_adapter_us": 0,
  "shape_us": 0,

  "alloc_count": 9412,
  "alloc_bytes": 1048576,

  "rules_matched": 123,
  "rules_fired": 31,
  "full_tree_traversals": 2,

  "join_algorithm": "DPHyp",
  "join_relations": 8,
  "memo_groups": 0,
  "compat_events": 0
}
```

Red flags:

```text
shape_us > 0 in native mode
doris_adapter_us > 0 in native mode
compat_events > 0 in native mode
memo_groups > 0 for simple query
full_tree_traversals > 3
```

### Latency expectations

Native local planner:

```text
simple scan/filter/project:
  p50 < 0.5 ms
  p95 < 1.5 ms

simple agg/topn:
  p50 < 1 ms
  p95 < 3 ms

TPC-H-style joins:
  p50 < 3–5 ms
  p95 < 10 ms

TPC-DS SF1 planning-only:
  median < 10 ms
  p95 < 25 ms
```

Rewrite-only:

```text
simple queries < 300 us
TPC-H < 1–2 ms
TPC-DS normal < 5 ms
```

Compare fairly against DuckDB:

```text
Native planning only vs DuckDB planning only
```

Do not compare:

```text
legacy Doris request-bin full path vs DuckDB planning
```

---

## Migration roadmap

### Phase 0: Freeze and observe

```text
freeze old memo product logic
add fallback ledger
add planning trace
add translator repair counter
add request-bin oracle facade
```

Gate:

```text
TPC-DS request-bin remains stable
legacy path emits phase trace and compatibility events
```

### Phase 1: SemanticGraph + normalization

```text
SELECT/FROM/WHERE/GROUP/LIMIT
SemanticGraph
AlgebraIndex
basic normalization
simple validators
```

Gate:

```text
TPC-H q1/q6 semantic + normalize
no dependency on old memo
```

### Phase 2: AccessPath + local physical plan

```text
TableScan
LakeScan local
Filter/Project/Aggregate
PhysicalDataflowPlan
LocalRuntimeGraph skeleton
```

Gate:

```text
TPC-H q1/q6/q12/q14 local planning
synthetic lake metadata pruning
```

### Phase 3: JoinRegion

```text
hash join
brute-force oracle
DPHyp
JoinLegalityOracle
JOB harness
```

Gate:

```text
TPC-H q3/q5/q10/q14
n <= 8 exact oracle matches
JOB planning-only
```

### Phase 4: Adaptive join + risk cost

```text
Adaptive LinDP
DPconv Ccap
GOO repair
CostEnvelope
StatsConfidence
```

Gate:

```text
synthetic graph corpus
dense/sparse routing correct
risk-aware comparator traceable
```

### Phase 5: Property search

```text
distribution
exchange
ordering
runtime filter edges
compact property memo
```

Gate:

```text
TPC-H 22 local plan generation
TPC-DS star subset
PhysicalDataflowPlan validator
```

### Phase 6: StageGraph and Doris adapter

```text
StageGraph
ExchangePlan
SplitSourcePlan
RuntimeFilterRoute
Doris adapter subset
```

Gate:

```text
local-cluster distributed TPC-H subset
Doris adapter scans/projects/filters/joins
adapter failure does not block native path
```

---

## PR checklist for future work

Every optimizer PR should answer:

```text
Does this touch old CascadesMemo?
Does this add a fallback? Which FallbackKind?
Does this run in native hot path?
Does this render strings in hot path?
Does this add a whole-tree traversal?
Does this add a new side table?
Does it have rule telemetry?
Does it have property/differential tests?
Does it affect Doris adapter only, or product optimizer?
Does it change planning latency by phase?
```

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

## Expected outcome

Architecture:

```text
old path:
  stable legacy oracle
  request-bin reference
  no new native features

new path:
  query-local arena
  typed views, not full artifact pipeline
  direct planning for simple queries
  adaptive join-region algorithms
  optional compact property search
  final PhysicalDataflowPlan
  local/distributed share physical plan
  Doris compatibility is optional adapter
```

Performance:

```text
simple native planning: sub-ms to low-ms
TPC-H native planning: single-digit ms
TPC-DS SF1 native planning: low tens of ms, not hundreds
rewrite: small fraction of total planning
legacy request-bin path measured separately
```

Engineering:

```text
rule changes become testable
join algorithms become independently benchmarkable
lake pruning becomes no-false-negative testable
Doris adapter becomes optional and inspectable
request-bin parity no longer drives core optimizer design
```

---

## Final decision

The product optimizer should not be the current Doris-compatible Cascades/Memo path.

It should be:

```text
query-local arena
normalized constructors
AlgebraIndex
AccessPathView
Adaptive JoinRegionOptimizer
optional compact PropertySearch
PhysicalDataflowPlan
LocalRuntimeGraph / StageGraph
optional DorisAdapter
```

The current Doris-compatible path remains as:

```text
oracle
baseline
adapter fallback
migration safety net
```

Native planning must be latency-first. If DuckDB can finish planning while this system is still materializing debug artifacts, the design is wrong. The phase names are useful for reasoning and CI snapshots, but the hot path must be a single arena compiler pass with targeted region optimization.
