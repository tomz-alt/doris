# Option B: DataFusion Planning + Doris BE Execution

## Goal

Create a production-ready Doris FE replacement that follows the proper architecture:
- **FE (Rust)**: SQL parsing, query planning, result coordination
- **BE (C++)**: Distributed query execution on tablet data

## Current State (Option A)

```
MySQL Client → Rust FE → DataFusion (Plan + Execute) → Results
                              └─ Arrow execution
                              └─ CSV data source
```

**Problem**: FE does all execution, BE not used at all.

## Target State (Option B)

```
MySQL Client → Rust FE → DataFusion (Parse + Plan)
                              ├─ Logical Plan
                              ├─ Optimized Plan
                              ├─ Physical Plan
                              └─ Convert to Doris Plan Fragments
                                    ↓
                            Doris BE (Execute via gRPC)
                              ├─ Scan tablets
                              ├─ Execute fragments
                              ├─ Aggregate results
                              └─ Return Arrow batches
                                    ↓
                            Rust FE (Coordinate)
                              └─ Convert to MySQL protocol
```

## Architecture Components

### 1. DataFusion Layer (Planning Only)

**Purpose**: Parse SQL and generate logical/physical plans

```rust
pub struct QueryPlanner {
    ctx: SessionContext,
}

impl QueryPlanner {
    // Parse SQL to DataFusion logical plan
    pub async fn parse_to_logical_plan(&self, sql: &str)
        -> Result<LogicalPlan>;

    // Optimize logical plan
    pub async fn optimize_logical_plan(&self, plan: LogicalPlan)
        -> Result<LogicalPlan>;

    // Convert to physical plan (DataFusion format)
    pub async fn create_physical_plan(&self, logical: LogicalPlan)
        -> Result<Arc<dyn ExecutionPlan>>;
}
```

### 2. Plan Converter (DataFusion → Doris)

**Purpose**: Convert DataFusion plans to Doris BE plan fragments

```rust
pub struct PlanFragmentBuilder {
    // Convert DataFusion ExecutionPlan to Doris PlanFragment
    pub fn build_fragments(&self, df_plan: Arc<dyn ExecutionPlan>)
        -> Result<Vec<PlanFragment>>;

    // Map DataFusion operators to Doris operators
    fn convert_operator(&self, op: &dyn ExecutionPlan)
        -> Result<PlanNode>;
}

// Mapping examples:
// DataFusion TableScan    → Doris OlapScanNode
// DataFusion Projection   → Doris ProjectNode
// DataFusion Filter       → Doris SelectNode
// DataFusion Aggregate    → Doris AggregationNode
// DataFusion Join         → Doris HashJoinNode
// DataFusion Sort         → Doris SortNode
```

### 3. Fragment Scheduler

**Purpose**: Distribute plan fragments to BE nodes

```rust
pub struct FragmentScheduler {
    be_pool: Arc<BackendClientPool>,

    // Assign fragments to BE nodes
    pub fn schedule_fragments(&self, fragments: Vec<PlanFragment>)
        -> Result<Vec<(BackendNode, PlanFragment)>>;

    // Execute fragments on assigned BEs
    pub async fn execute_distributed(&self, assignments: Vec<(BackendNode, PlanFragment)>)
        -> Result<Vec<BatchResult>>;
}
```

### 4. Result Coordinator

**Purpose**: Collect and merge results from multiple BEs

```rust
pub struct ResultCoordinator {
    // Merge Arrow batches from multiple BEs
    pub fn merge_results(&self, batches: Vec<RecordBatch>)
        -> Result<RecordBatch>;

    // Convert final Arrow results to MySQL protocol
    pub fn to_mysql_result(&self, batch: RecordBatch)
        -> Result<QueryResult>;
}
```

## Implementation Phases

### Phase 1: Plan Extraction ✓ (Already Done)

DataFusion already provides access to logical and physical plans:

```rust
let df = ctx.sql("SELECT * FROM lineitem WHERE l_shipdate < '1998-12-01'").await?;
let logical_plan = df.logical_plan();  // ✓ Available
let physical_plan = df.create_physical_plan().await?;  // ✓ Available
```

### Phase 2: Operator Mapping (New)

Create mappings for common operators:

| DataFusion Operator | Doris BE Node | Priority |
|---------------------|---------------|----------|
| TableScan           | OlapScanNode  | P0       |
| Filter              | SelectNode    | P0       |
| Projection          | ProjectNode   | P0       |
| Aggregate           | AggregationNode | P0     |
| HashJoin            | HashJoinNode  | P1       |
| Sort                | SortNode      | P1       |
| Limit               | TopNNode      | P1       |
| Union               | UnionNode     | P2       |
| Window              | AnalyticNode  | P2       |

### Phase 3: Fragment Construction (New)

Convert DataFusion physical plan to Doris plan fragments:

```rust
// DataFusion plan tree:
//   Aggregate
//     └─ Filter
//          └─ TableScan
//
// Converts to Doris fragment:
// Fragment 0 (on BE):
//   - OlapScanNode (read tablets)
//   - SelectNode (apply filter)
//   - AggregationNode (local aggregation)
//
// Fragment 1 (on FE coordinator):
//   - ExchangeNode (receive from Fragment 0)
//   - AggregationNode (final merge)
```

### Phase 4: gRPC Communication (Blocked by protoc)

**Current Blocker**: No protoc available for compiling `.proto` files

**Required protobuf messages**:
```protobuf
message PExecPlanFragmentRequest {
    bytes query_id = 1;
    int32 fragment_instance_id = 2;
    PlanFragment fragment = 3;
    QueryOptions query_options = 4;
}

message PFetchDataRequest {
    bytes query_id = 1;
    int32 fragment_instance_id = 2;
}

message PFetchDataResponse {
    int32 status_code = 1;
    string message = 2;
    bytes data = 3;  // Arrow RecordBatch serialized
}
```

**Workaround Options**:
1. Pre-compile proto files on another machine
2. Use JSON-RPC instead of gRPC (less efficient)
3. Write manual protobuf encoding (complex)

### Phase 5: Distributed Execution (New)

Execute fragments across multiple BE nodes:

```rust
async fn execute_query_distributed(&self, sql: &str) -> Result<QueryResult> {
    // 1. Parse SQL to logical plan
    let logical_plan = self.planner.parse_to_logical_plan(sql).await?;

    // 2. Optimize plan
    let optimized = self.planner.optimize_logical_plan(logical_plan).await?;

    // 3. Create physical plan
    let physical = self.planner.create_physical_plan(optimized).await?;

    // 4. Convert to Doris plan fragments
    let fragments = self.converter.build_fragments(physical)?;

    // 5. Schedule fragments to BE nodes
    let assignments = self.scheduler.schedule_fragments(fragments)?;

    // 6. Execute on all BEs in parallel
    let results = self.scheduler.execute_distributed(assignments).await?;

    // 7. Merge results
    let merged = self.coordinator.merge_results(results)?;

    // 8. Convert to MySQL protocol
    self.coordinator.to_mysql_result(merged)
}
```

## Challenges & Solutions

### Challenge 1: No Protobuf Compiler

**Problem**: Cannot compile `.proto` files without protoc

**Solutions**:
1. **Pre-compile** proto files on development machine, commit generated Rust code
2. **Use crate**: Use pre-built `doris-proto` crate if available
3. **JSON fallback**: Use JSON over HTTP instead of protobuf over gRPC (slower)
4. **Manual encoding**: Write protobuf wire format manually (complex but possible)

**Recommendation**: Option 1 (pre-compile) or Option 3 (JSON) for quick progress

### Challenge 2: Schema Mapping

**Problem**: DataFusion uses Arrow schema, Doris uses custom schema

**Solution**: Create schema translator

```rust
fn arrow_schema_to_doris(arrow: &ArrowSchema) -> DorisSchema {
    arrow.fields().iter().map(|field| {
        DorisColumn {
            name: field.name().clone(),
            data_type: match field.data_type() {
                ArrowDataType::Int32 => DorisType::Int,
                ArrowDataType::Utf8 => DorisType::Varchar,
                ArrowDataType::Decimal128(p, s) => DorisType::Decimal(*p, *s),
                // ... more mappings
            },
            nullable: field.is_nullable(),
        }
    }).collect()
}
```

### Challenge 3: Tablet Metadata

**Problem**: Doris distributes data across tablets, need metadata

**Solution**: Query BE for tablet information

```rust
async fn get_tablet_info(&self, table: &str) -> Result<Vec<TabletInfo>> {
    // Option 1: Query FE metadata (need FE-FE communication)
    // Option 2: Query BE directly for tablet list
    // Option 3: Maintain local cache with periodic refresh
}
```

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_convert_table_scan() {
    let df_scan = create_df_table_scan("lineitem");
    let doris_node = converter.convert_operator(&df_scan).unwrap();
    assert!(matches!(doris_node, PlanNode::OlapScan(_)));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_simple_query_with_be() {
    let sql = "SELECT COUNT(*) FROM lineitem";
    let result = executor.execute_distributed(sql).await.unwrap();
    assert_eq!(result.rows.len(), 1);
}
```

### TPC-H Validation

Run all 22 TPC-H queries and compare results:
- Option A (DataFusion only) vs
- Option B (DataFusion + BE)

Results should be identical.

## Migration Path

### Week 1: Foundation
- [ ] Extract logical/physical plans from DataFusion
- [ ] Create basic operator mapper (TableScan, Filter, Project)
- [ ] Build simple PlanFragment structure

### Week 2: Protobuf
- [ ] Solve protoc issue (pre-compile or JSON fallback)
- [ ] Implement gRPC client for BE communication
- [ ] Test single-fragment execution

### Week 3: Distribution
- [ ] Implement fragment scheduler
- [ ] Add result coordinator
- [ ] Test multi-fragment execution

### Week 4: Validation
- [ ] Run TPC-H benchmarks
- [ ] Compare Option A vs Option B results
- [ ] Performance testing

## Success Criteria

✅ **Correctness**: All TPC-H queries return same results as Java FE
✅ **Performance**: Within 20% of Java FE performance
✅ **Scalability**: Can distribute across multiple BE nodes
✅ **Architecture**: FE only plans, BE executes (proper separation)

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Protobuf blocker | High | Pre-compile or use JSON |
| Complex plan conversion | High | Start with simple queries |
| BE compatibility | Medium | Test with actual Doris BE |
| Performance regression | Medium | Benchmark continuously |

## Next Immediate Steps

1. **Document Option A success** ✓ DONE
2. **Commit current state** ✓ DONE
3. **Create plan converter scaffold** ← START HERE
4. **Test plan extraction** from DataFusion
5. **Build simple operator mapper**

## References

- DataFusion ExecutionPlan: https://docs.rs/datafusion/latest/datafusion/physical_plan/trait.ExecutionPlan.html
- DataFusion LogicalPlan: https://docs.rs/datafusion/latest/datafusion/logical_expr/enum.LogicalPlan.html
- Doris BE Protocol: `be/src/gen_cpp/PaloInternalService_types.h`
- Arrow IPC Format: https://arrow.apache.org/docs/format/Columnar.html
