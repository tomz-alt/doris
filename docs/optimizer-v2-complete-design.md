# Optimizer V2 Complete Design

> Status: implementation design draft  
> Scope: Rust Optimizer V2, PhysicalDataflowPlan, runtime/operator/storage integration, lakehouse-native pruning, distributed lowering, and proof strategy.  
> Parent handoff: `docs/optimizer-v2-handoff.md` at `a951f32f0c96b1afea5a0f4f981c4613af36eac2`.  
> Core decision: one product optimizer core, one physical dataflow contract, multiple lowerings. Doris/Nereids compatibility remains an oracle/adapter, not the native optimizer architecture.

---

## 0. Executive Summary

Optimizer V2 is a compiler from SQL semantics plus source-versioned access paths into a bounded `PhysicalDataflowPlan`.

It must serve three products without three optimizer frameworks:

```text
Embedded / Doris Lite
  SQL or prepared template
  -> OptimizerV2
  -> PhysicalDataflowPlan
  -> LocalRuntimeGraph

Doris distributed / server
  SQL / Nereids-exported logical plan / prepared template
  -> OptimizerV2
  -> PhysicalDataflowPlan
  -> StageGraph

Doris compatibility
  LegacyDorisPlanner / Nereids
  -> oracle, request-bin reference, adapter fallback
```

The design intentionally freezes the existing Doris-compatible Cascades/Memo/transpiler path as a compatibility oracle. Native planning must not execute request-bin compatibility, shape rendering, Java identity emulation, legacy descriptor repair, or silent translator repair.

The optimizer product is the `PhysicalDataflowPlan`. Runtime and operators consume that plan through typed contracts: `ReadView`, `ResourceEnvelope`, `TopologyEnvelope`, `AccessPathSpec`, `DynamicFilterSpec`, `AdaptivePoint`, `ProfileTagDictionary`, and `PlanProof`.

The key product rule is:

```text
One OptimizerV2 core.
One PhysicalDataflowPlan.
One Runtime contract.
One Operator contract.
One Join Framework.
Multiple lowerings and multiple execution algorithms are allowed.
Multiple optimizer frameworks are not allowed.
```

---

## 1. Current Status and Design Correction

The current Rust planning path has achieved valuable compatibility milestones: broad TPC-DS request-bin parity, SF1000 shape parity against the current baseline, and a useful Java/Nereids oracle. It also accumulated architectural debt: the legacy `CascadesMemo` mixes search state, Java identity, scalar contracts, statistics, runtime filters, distribution metadata, descriptor-related facts, shape/debug compatibility, CTE/MV state, and translator assumptions.

That path remains valuable as:

```text
LegacyDorisPlanner
  -> request-bin oracle
  -> Java/Nereids compatibility reference
  -> regression baseline
  -> optional Doris adapter fallback
```

It must stop being the product optimizer. New lakehouse-native behavior, embedded planning, dynamic filters, PruningPlan, access-path selection, adaptive points, and runtime/operator contracts belong in Optimizer V2.

---

## 2. Goals

Optimizer V2 must:

1. Plan embedded, lakehouse, and distributed queries through one core and one product IR.
2. Use a query-local arena and typed IDs, not a new global blackhole memo.
3. Make rewrite legality explicit through `AlgebraIndex` and barrier-aware semantic views.
4. Treat lake pruning, dynamic filters, MV/index/fragment-cache choices, and split sources as first-class access-path objects.
5. Use adaptive join-region optimization with shared join legality and cost envelopes.
6. Emit executable resource and topology envelopes instead of after-the-fact physical repair.
7. Integrate directly with Runtime `QueryScope`, `Processor::step`, memory/I/O/output credits, and cancellation/drain semantics.
8. Emit stable profile tags and plan provenance so E2E performance is explainable.
9. Use feedback for statistics confidence, dynamic-filter policy, pruning effectiveness, and plan-cache specialization without making feedback a correctness authority.
10. Prove correctness and performance separately on ClickBench, TPC-H, JOB, and TPC-DS.

---

## 3. Non-Goals

Optimizer V2 does not:

- embed Java FE/Nereids as a required runtime dependency;
- keep adding product behavior to the legacy memo/transpiler/request-bin path;
- make ML or history feedback a correctness authority;
- silently repair plans after optimization;
- use a scalar-only cost model for lake/distributed decisions;
- maintain separate embedded, lake, and distributed optimizers;
- maintain separate vectorized and compiled join frameworks;
- perform arbitrary mid-query reoptimization outside declared `AdaptivePoint`s.

---

## 4. Hard Dependency Wall

Native Optimizer V2 crates must not depend on:

```text
fers-cascades legacy memo internals
fers-transpiler
fers-doris-compat
legacy descriptor repair
request-bin renderer
legacy physical-contract repair
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

CI gate:

```bash
cargo xtask check-opt-v2-deps
cargo deny check
```

Native planning fails if it observes:

```text
shape_us > 0
doris_adapter_us > 0
compat_events > 0
memo_groups > 0 for simple native queries
SilentRepair in FallbackLedger
LegacyDescriptorLayout in native mode
DorisDisplayCompatibility in native mode
```

---

## 5. Product IR and Data Model

### 5.1 QueryArena

Only two objects exist on the native hot path:

```text
1. QueryArena during planning
2. Frozen PhysicalDataflowPlan after planning
```

Everything else is a view over the arena, a debug/CI snapshot, or an adapter artifact.

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

Required implementation style:

```text
typed IDs: RelNodeId, ExprId, FieldId, AccessPathId, PhysNodeId
dense storage: PrimaryMap, Vec, Box<[T]>, SmallVec, ArrayVec
bitsets: RelSet as u64/u128 for small joins; fixed bitsets for larger graphs
strings: interned; no rendering in hot path
scratch: query-local arena/bump allocation
hashing: query-local fast hash; stable hash only for fingerprints
```

Avoid:

```text
Arc<dyn PlanNode>
Box<dyn Expr>
HashMap everywhere
shape rendering in native planning
request-bin ordering in native planning
full Cascades memo for simple queries
Doris adapter in product planning
```

### 5.2 SemanticView

The bound semantic view contains:

```text
relation IDs
field IDs
expression IDs
types
nullability
scope
CTE definitions
subquery/apply nodes
logical operators
```

It must not contain:

```text
Doris slot IDs
Doris tuple IDs
Java ExprIds
request-bin order
fragment IDs
backend locations
```

### 5.3 NormalizedView

After normalization, guarantees:

```text
no unresolved names
no accidental free variables
subqueries either unnested or explicitly represented
CTE maps explicit
field lineage explicit
outer/semi/anti/mark barriers explicit
```

Holistic unnesting belongs here, before join ordering and before property search.

### 5.4 AlgebraIndex

`AlgebraIndex` is the rewrite maintainability layer.

It answers:

```text
where is a field produced?
where is it consumed?
can this predicate move below this join?
does this node cross an outer/semi/anti/mark barrier?
what is the lowest common ancestor of an expression's inputs?
```

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

Semantic rules must use `AlgebraIndex`; they may not rediscover scope and lineage through ad hoc full-tree walks.

---

## 6. Access Paths and Lake-Native Planning

### 6.1 AccessPathView

For each base relation, enumerate:

```text
TableScan
LakeScan
IndexScan
MVScan
FragmentCache
JoinInputIndexScan
```

```rust
pub enum AccessPath {
    TableScan(TableScanPath),
    LakeScan(LakeScanPath),
    IndexScan(IndexScanPath),
    MVScan(MVScanPath),
    FragmentCache(FragmentCachePath),
    JoinInputIndexScan(JoinInputIndexPath),
}
```

### 6.2 LakeScanPath

```rust
pub struct LakeScanPath {
    pub table: TableId,
    pub source_version: SourceVersionRef,
    pub projection: FieldSet,
    pub predicate_domain: PredicateDomain,
    pub pruning: PruningPlan,
    pub split_source: SplitSourcePlan,
    pub dynamic_filter_consumers: SmallVec<[DynamicFilterId; 4]>,
    pub metadata_cache_policy: MetadataCachePolicy,
    pub late_materialization: LateMaterializationPlan,
    pub estimate: ScanEstimate,
}
```

`LakeScanPath` is the bridge to AMETA and storage. It is not a low-level file iterator.

### 6.3 PruningPlan

```rust
pub struct PruningPlan {
    pub table: TableId,
    pub source_version: SourceVersionRef,
    pub static_predicates: Vec<PredicateId>,
    pub partition_pruning: Vec<PartitionPruneExpr>,
    pub file_pruning: Vec<FilePruneExpr>,
    pub row_group_pruning: Vec<RowGroupPruneExpr>,
    pub page_pruning: Vec<PagePruneExpr>,
    pub dynamic_filter_consumers: Vec<DynamicFilterId>,
    pub metadata_budget: MetadataBudget,
    pub fallback_policy: FallbackPolicy,
    pub proof: PruningProof,
}
```

Correctness invariant:

```text
Pruning may produce false positives.
Pruning must never produce false negatives.
Residual predicate must be preserved unless a proof says the unit is FullyMatch.
```

### 6.4 PruningProof

```rust
pub struct PruningProof {
    pub source_version: SourceVersionRef,
    pub predicate: PredicateId,
    pub pruned_units: u64,
    pub residual_preserved: bool,
    pub no_false_negative_reason: ProofKind,
}
```

The proof appears in debug/profile output and in CI-generated random metadata pruning tests.

### 6.5 Metadata Scale Rule

FE/optimizer must not expand PB-scale lake metadata into millions of heap objects.

```text
Optimizer:
  source version, pruning expressions, FileGroupManifest references, dynamic filter specs

Runtime/storage:
  lazy expand file groups into split/morsel units under I/O and memory credits

AMETA:
  large source-versioned metadata, statistics, layout health, coverage, heat, debt
```

---

## 7. Dynamic Filters and Predicate Transfer

Runtime filters and predicate transfer must be optimizer objects, not post-plan decorations.

```rust
pub enum DynamicFilterKind {
    Bloom,
    ValueSet,
    MinMax,
    DictionarySet,
    PerfectBitset,
}

pub enum DynamicFilterWaitPolicy {
    WaitBriefly { max_wait_us: u32 },
    StartAndApplyLate,
    Bypass,
}

pub struct DynamicFilterSpec {
    pub id: DynamicFilterId,
    pub producer: OperatorId,
    pub consumers: Vec<DynamicFilterConsumer>,
    pub key_expr: ExprId,
    pub kind: DynamicFilterKind,
    pub max_bytes: u64,
    pub wait_policy: DynamicFilterWaitPolicy,
    pub expected_benefit: CostEnvelope,
    pub profile_tag: DynamicFilterTagId,
}
```

Consumers include:

```text
LakeScan file/row-group/page pruning
Segment mark/zone-map pruning
dictionary filtering
hash join probe-side prefilter
late payload fetch gating
```

Runtime behavior:

```text
small build side and high predicted benefit:
  scan waits briefly

uncertain or late filter:
  scan starts and applies filter to remaining work

negative measured benefit:
  bypass and record feedback
```

Correctness invariant:

```text
Dynamic filters may have false positives.
False negatives are correctness failures.
DynamicFilter-enabled and disabled execution must return identical logical results.
```

---

## 8. Join Planning and Join Framework

### 8.1 Join legality first

All join enumeration algorithms share one semantic legality oracle.

```rust
pub struct JoinHyperGraph {
    pub rels: RelSet,
    pub edges: Vec<JoinHyperEdge>,
    pub operators: Vec<JoinOperator>,
}

pub trait JoinLegalityOracle {
    fn can_join(&self, left: RelSet, right: RelSet) -> Option<ApplicableJoin>;
}
```

All algorithms consume this oracle:

```text
DPHypExact
AdaptiveLinDP
MPDP
DPconvCcap
GOO + local repair
ConstrainedDPHyp
```

Join legality is semantic. Join enumeration is algorithmic. They must not be mixed.

### 8.2 Join algorithm router

```rust
pub enum JoinEnumerationAlgorithm {
    BruteForceOracle,
    DPHypExact,
    AdaptiveLinDP,
    MPDP,
    DPconvCcap,
    GooLocalRepair,
    ConstrainedDPHyp,
}
```

Router policy:

```text
n <= 8:
  brute-force oracle for proof and small planning

outer/semi/anti/mark constraints:
  ConstrainedDPHyp

bounded connected-subgraph count:
  DPHypExact

tree/star/snowflake:
  AdaptiveLinDP

dense and memory-sensitive:
  DPconvCcap

large analytical graph with parallel budget:
  MPDP

huge or budget-constrained:
  GOO + local repair
```

### 8.3 One Join Framework

Optimizer V2 and operators maintain one join framework.

```text
Join framework:
  semantic spec
  physical spec
  lifecycle
  memory/spill contract
  runtime filter contract
  profile/cancel/drain contract

Join methods:
  BroadcastHash
  PartitionedHash
  GraceHash
  MergeJoin
  IndexLookupJoin
  NestedLoopTiny
  SemiFilterOnly
  MergedInputIndex

Execution backends:
  Vectorized
  StaticFused
  Optional JIT in later phase
```

Not allowed:

```text
separate vectorized join framework
separate compiled join framework
separate embedded join framework
separate distributed join framework
separate hash table / spill / RF / profile per backend
```

### 8.4 JoinPhysicalSpec

```rust
pub struct JoinPhysicalSpec {
    pub semantic: JoinSemanticSpec,
    pub algorithm: JoinMethod,
    pub build_side: BuildSide,
    pub build_keys: Vec<ExprId>,
    pub probe_keys: Vec<ExprId>,
    pub build_payload: ProjectionSet,
    pub probe_payload: ProjectionSet,
    pub output_projection: ProjectionSet,
    pub dynamic_filters: Vec<DynamicFilterSpec>,
    pub memory_policy: JoinMemoryPolicy,
    pub spill_policy: JoinSpillPolicy,
    pub dop_policy: JoinDopPolicy,
    pub profile_tag: OperatorTagId,
}
```

### 8.5 JoinInputIndex as access path

`JoinInputIndex` / merged input index is an optional physical access path for repeated binary equality joins. It is not a second join framework and not a full materialized join view.

```rust
pub struct JoinInputIndexPath {
    pub index_id: JoinInputIndexId,
    pub left: RelationRef,
    pub right: RelationRef,
    pub join_keys: Vec<JoinKeyExpr>,
    pub source_coverage: CoverageProof,
    pub covering: CoveringLevel,
    pub freshness: FreshnessContract,
    pub estimate: JoinInputIndexEstimate,
}
```

Planner may choose:

```text
HashJoin + DynamicFilter
MergeJoin
IndexLookupJoin
MergedInputIndexJoinMethod
MaterializedView
```

Promotion into a `JoinInputIndex` requires workload heat, stable join keys, complete coverage, maintenance-cost acceptance, and equivalence against base HashJoin.

---

## 9. Cost, Risk, and Statistics

### 9.1 CostEnvelope

Use a cost envelope, not one scalar.

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
broadcast requires memory envelope
low-confidence estimates prefer robust plans
hash-table build side must be runtime-resizable or spill-ready
runtime filter waiting requires expected benefit and max wait
lake scan alternatives must account for object-store requests and bytes
```

### 9.2 StatsTrustLevel

```rust
pub enum StatsTrustLevel {
    ExactConstraint,
    FreshColumnStats,
    FreshFileStats,
    FreshRowGroupStats,
    RuntimeFeedback,
    Sampled,
    StaleStats,
    Missing,
}

pub struct StatsEvidence {
    pub trust: StatsTrustLevel,
    pub source_version: Option<SourceVersionRef>,
    pub collected_at: Option<Timestamp>,
    pub confidence: f32,
    pub staleness: StatsStaleness,
}
```

Every estimate carries evidence and confidence. Missing or stale lake stats reduce pruning confidence and push the planner toward robust plans.

### 9.3 OptimizerFeedbackStore

Feedback is advisory evidence, never correctness authority.

```rust
pub struct PlanFeedback {
    pub plan_digest: Hash,
    pub query_fingerprint: Hash,
    pub source_versions: Vec<SourceVersionRef>,

    pub estimated_rows: Vec<RowEstimate>,
    pub actual_rows: Vec<RowActual>,

    pub estimated_bytes: Vec<ByteEstimate>,
    pub actual_bytes: Vec<ByteActual>,

    pub dynamic_filter_selectivity: Vec<FilterActual>,
    pub pruning_effectiveness: Vec<PruningActual>,

    pub memory_peak: u64,
    pub spill_bytes: u64,
    pub runtime_waits: RuntimeWaitSummary,
    pub operator_profile: OperatorProfileSummary,
}
```

Feedback drives:

```text
stats confidence update
runtime-filter wait/bypass policy
join algorithm routing
lake pruning effectiveness
layout/statistics maintenance recommendations
plan-template specialization
```

---

## 10. PhysicalDataflowPlan Contract

`PhysicalDataflowPlan` is the only product physical IR.

```rust
pub struct PhysicalDataflowPlan {
    pub plan_id: PlanDigest,

    pub read_view: ReadView,
    pub resource_envelope: ResourceEnvelope,
    pub topology_envelope: TopologyEnvelope,

    pub nodes: Vec<PhysicalOperatorSpec>,
    pub edges: Vec<DataflowEdge>,

    pub access_paths: Vec<AccessPathSpec>,
    pub split_sources: Vec<SplitSourcePlan>,

    pub dynamic_filters: Vec<DynamicFilterSpec>,
    pub adaptive_points: Vec<AdaptivePoint>,

    pub profile_tags: ProfileTagDictionary,
    pub proof: PlanProof,
    pub provenance: PlanProvenance,
    pub fallback_ledger: FallbackLedger,
}
```

### 10.1 ResourceEnvelope

```rust
pub struct ResourceEnvelope {
    pub workload_class: WorkloadClass,
    pub min_progress_memory: u64,
    pub max_memory: Option<u64>,
    pub expected_peak_memory: CostInterval<u64>,
    pub io_classes: Vec<IoClassBudget>,
    pub output_credit: OutputCreditSpec,
    pub spill_policy: SpillPolicy,
    pub scheduling_class: SchedulingClass,
}
```

The optimizer estimates; runtime enforces. A plan that cannot declare memory, I/O, output, or spill behavior is not executable.

### 10.2 TopologyEnvelope

```rust
pub struct TopologyEnvelope {
    pub mode: TopologyMode,
    pub distribution: DistributionRequirement,
    pub ordering: OrderingRequirement,
    pub locality: LocalityRequirement,
    pub exchange: ExchangePlan,
    pub placement_hints: Vec<PlacementHint>,
}
```

Local and distributed lowerings consume the same envelope:

```text
LocalRuntimeGraph:
  exchange -> bounded local channel
  placement -> NUMA/cache-domain target

StageGraph:
  exchange -> network exchange
  placement -> BE / failure domain / shard owner
```

### 10.3 AdaptivePoint

```rust
pub enum AdaptivePointKind {
    AfterDimensionBuild,
    AfterRuntimeFilterBuild,
    AfterFirstNRowGroups,
    AfterCteMaterialization,
    AfterHashBuild,
    AfterPartialAggregate,
    OnMemoryPressure,
    OnSpillBoundary,
}

pub enum AdaptiveDecisionKind {
    Continue,
    WaitOrBypassDynamicFilter,
    ApplyFilterLate,
    SwitchPayloadMaterialization,
    ReduceDop,
    EnableEarlySpill,
    MaterializeSubplan,
    SwitchRemainingJoinAlgorithm,
}

pub struct AdaptivePoint {
    pub id: AdaptivePointId,
    pub kind: AdaptivePointKind,
    pub allowed_decisions: SmallVec<[AdaptiveDecisionKind; 4]>,
    pub decision_budget_us: u32,
    pub profile_tag: AdaptivePointTagId,
}
```

No arbitrary optimizer recursion inside pipelines. Adaptivity is legal only at declared points.

### 10.4 PlanProof

```rust
pub struct PlanProof {
    pub semantic_validation: SemanticProof,
    pub lineage_validation: LineageProof,
    pub join_legality: JoinLegalityProof,
    pub pruning: Vec<PruningProof>,
    pub dynamic_filter_edges: DynamicFilterProof,
    pub resource_envelope: ResourceEnvelopeProof,
    pub split_sources: SplitSourceProof,
}
```

Hot-path validation is `O(nodes + exprs)`. Deep validators run in CI/debug.

---

## 11. Runtime and Operator Integration

### 11.1 Local lowering

```text
PhysicalDataflowPlan
  -> LocalRuntimeGraph
  -> QueryScope
  -> Pipelines
  -> Drivers
  -> Processor::step
```

Runtime owns:

```text
QueryScope
TaskRegistry
IoRegistry
MemoryPermit
OutputCredit
WaitSet
Cancel/Pause/Kill
Profile and wait-for graph
```

Operators do not create threads, futures, queues, page pools, or private I/O paths.

### 11.2 Operator factory

```rust
pub trait PhysicalOperatorFactory {
    fn create(
        &self,
        spec: &PhysicalOperatorSpec,
        ctx: &mut OperatorInitContext,
    ) -> Result<Box<dyn Processor>>;
}
```

Every operator declares:

```rust
pub struct OperatorRuntimeContract {
    pub min_progress_memory: u64,
    pub memory_classes: Vec<MemoryClass>,
    pub can_spill: bool,
    pub output_schema: Schema,
    pub preserves_ordering: OrderingProperty,
    pub distribution: DistributionProperty,
    pub cancel_safe_points: Vec<SafePointKind>,
    pub profile_tag: OperatorTagId,
}
```

### 11.3 Processor contract

```rust
trait Processor {
    fn step(
        &mut self,
        cx: &mut StepContext,
        cpu: &mut CpuBudget,
        memory: &mut MemoryGrant,
    ) -> StepResult;
}

enum StepResult {
    Progress,
    Yield(YieldReason),
    Await(WaitSet),
    NeedMemory(MemoryRequest),
    NeedSpill(SpillPlan),
    Finished,
    Failed(QueryError),
}
```

Heavy operators are synchronous, resumable state machines. Async handles waiting; morsels handle CPU.

### 11.4 Internal batch ABI

Optimizer and operators assume one internal batch contract.

```rust
pub struct EncodedBatch {
    pub row_count: u32,
    pub selection: Selection,
    pub row_ids: Option<RowIdVector>,
    pub columns: Box<[ColumnVectorRef]>,
    pub lease: BatchLease,
    pub provenance: BatchProvenance,
}

pub enum ColumnVectorRef {
    Flat(FlatVectorRef),
    Constant(ConstantVectorRef),
    Dictionary(DictionaryVectorRef),
    Sequence(SequenceVectorRef),
    RunLength(RleVectorRef),
    Lazy(LazyVectorRef),
    EncodedPage(EncodedPageRef),
}
```

Arrow is the external interchange boundary. It is not mandatory between every internal operator.

### 11.5 Storage integration

Optimizer emits `AccessPathSpec` and `PruningPlan`. Storage emits `ReadIntent`s and decodes into runtime-owned `BatchLease`s.

```text
AccessPathSpec
  -> SourceAdapter
  -> SplitSourcePlan
  -> ReadIntent
  -> AdaptiveRead
  -> ExtentBuffer
  -> EncodedBatch
```

Storage does not own scheduling, memory, I/O, or cancellation. Runtime does not own source semantics or format correctness.

---

## 12. Profiling, Trace, and Plan Attribution

Optimizer V2 emits a `ProfileTagDictionary` so runtime/operator/storage profiles can map events and samples to plan nodes.

```rust
pub struct ProfileTagDictionary {
    pub plan_digest: Hash,
    pub operators: Vec<OperatorTag>,
    pub pipelines: Vec<PipelineTag>,
    pub access_paths: Vec<AccessPathTag>,
    pub dynamic_filters: Vec<DynamicFilterTag>,
    pub adaptive_points: Vec<AdaptivePointTag>,
    pub kernels: Vec<KernelTag>,
}
```

Default profile stack:

```text
RuntimeProfileSummary:
  always-on counters and histograms; resource truth

FlightRecorder:
  bounded event ring; dumped on slow/error/cancel/OOM

fastrace:
  optional sampled Rust spans behind TraceSink

Perfetto:
  trace export and SQL analysis

Umbra-style attribution:
  PlanIR/operator/pipeline/kernel mapping for hardware samples and fused kernels
```

Performance results without profile evidence are invalid.

---

## 13. Planning Pipeline

Native hot path:

```rust
pub fn plan_native_fast(req: PlanRequest) -> Result<Arc<PhysicalDataflowPlan>> {
    let mut arena = QueryArena::with_capacity(req.estimated_size());

    let root = bind_into_arena(&mut arena, req.sql, req.catalog)?;
    let root = normalize_in_place(&mut arena, root)?;

    derive_traits_and_stats_once(&mut arena, root, req.stats)?;
    attach_access_paths(&mut arena, root, req.catalog, req.lake_metadata)?;

    let join_regions = extract_join_regions(&mut arena, root)?;
    optimize_join_regions(&mut arena, &join_regions, req.budget)?;

    run_compact_property_search(&mut arena, root, req)?;

    let physical_root = build_physical_dataflow(&mut arena, root, req)?;
    let plan = freeze_physical_plan(&mut arena, physical_root)?;

    validate_physical_dataflow_linear(&plan)?;
    Ok(Arc::new(plan))
}
```

Native planning does not call:

```text
Doris adapter
request-bin renderer
shape renderer
legacy descriptor repair
full Cascades memo
global validators
```

---

## 14. Crate Layout

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
  fers-dynamic-filter
  fers-property-search
  fers-physical-dataflow
  fers-plan-validate
  fers-profile-tags
  fers-feedback
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
 -> dynamic-filter
 -> property-search
 -> physical-dataflow
 -> local-lower / stage-lower / doris-adapter
```

---

## 15. Benchmark Roles

One benchmark cannot prove the whole optimizer.

```text
ClickBench:
  access path, scan, pruning, expression, aggregation, distinct, Top-K, hot/cold path

TPC-H:
  core relational planning, join graph, runtime filters, aggregation, ordering, local physical planning

JOB:
  join-order robustness, cardinality error, legality oracle, algorithm routing, regret

TPC-DS:
  integrated SQL/rewrite/join/property/search/lake pruning/resource behavior
```

---

## 16. Architecture Decisions

### ADR-OPT-001 — Freeze legacy as oracle

The Doris-compatible Cascades/Memo/transpiler path remains request-bin oracle, Java/Nereids compatibility reference, regression baseline, and optional adapter fallback. It no longer receives product optimizer behavior.

### ADR-OPT-002 — QueryArena and typed views

Native planning uses a query-local arena and typed views. It does not create a new global memo.

### ADR-OPT-003 — PhysicalDataflowPlan is the product IR

Local, distributed, and Doris compatibility lowerings consume the same `PhysicalDataflowPlan`.

### ADR-OPT-004 — Runtime filters are costed plan objects

Dynamic filters are modeled during access-path selection, join enumeration, property search, and adaptive execution.

### ADR-OPT-005 — One Join Framework

Join semantics, state lifecycle, memory/spill, dynamic filters, cancellation/drain, and profile IDs are single-source-of-truth. Algorithms and execution backends are pluggable.

### ADR-OPT-006 — Feedback is advisory

Runtime feedback improves estimates, policies, and templates. It never changes correctness or prunes data without a proof.

---

## 17. References

The research inventory for this design includes unified optimizer/lakehouse optimizer work, Cascades/ORCA/Columbia optimizer architecture, lake-native execution, adaptive query execution, cardinality/robustness work, large join enumeration, indexed algebra, holistic unnesting, runtime-filter-aware optimization, predicate transfer, history-based optimization, DBSP, Velox/Umbra execution, and lake pruning. See the companion research inventory used during design review for direct links.
