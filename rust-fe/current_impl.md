# Current Implementation Reference

**Last Updated**: 2025-11-17

## Architecture

### Key Components

**BE Communication** (`src/be/`)
- `client.rs` - gRPC client for BE interaction (exec_pipeline_fragments, fetch_data)
- `pool.rs` - BE connection pooling and query routing
- `thrift_pipeline.rs` - Thrift payload generation for pipeline execution

**Query Execution Flow**
```
MySQL Client → opensrv_server → QueryExecutor → BackendPool → BE (gRPC)
                                                             ↓
                                          TPipelineFragmentParamsList
                                                             ↓
                                                    fetch_data loop
                                                             ↓
                                                      TResultBatch
```

### Critical Implementation Details

**Fragment Structure** (`src/be/thrift_pipeline.rs`)
- 3-fragment execution plan (matching Java FE):
  - Fragment 0 (fragment_id=0): Scan with TPaloScanRange
  - Fragment 1 (fragment_id=1): Exchange (placeholder)
  - Fragment 2 (fragment_id=2): Result sink (placeholder)
- Each fragment requires TPlanFragment with plan nodes (BE validation)

**Scan Ranges** (`src/be/thrift_pipeline.rs:1290+`)
- Loaded from `scan_ranges.json` config
- TPaloScanRange fields: tablet_id, schema_hash, version, hosts
- **Validated**: version='3' required, schema_hash correct
- Mapped to per_node_scan_ranges in TPipelineInstanceParams
- Location: `encode_pipeline_instance_params_typed()`

**Descriptor Table** (`src/be/thrift_pipeline.rs`)
- **FIXED (2025-11-17)**:
  - Java FE: 18 slot_descriptors + **5 tuple_descriptors**
  - Rust FE: 18 slot_descriptors + **5 tuple_descriptors** ✅ FIXED
  - **Added**: tuples 2, 3, 4 (projection/aggregation/result)
- Current impl: `encode_descriptor_table_for_scan_typed()` at line 1811
- **Fix committed**: All 5 tuples now created matching Java FE pattern
  - tuple 0: id=0, table_id=Some(...) - Base scan tuple
  - tuple 1: id=1, table_id=Some(...) - Base scan tuple (duplicate for BE processing)
  - tuple 2: id=2, table_id=None - Projection tuple
  - tuple 3: id=3, table_id=None - Aggregation tuple
  - tuple 4: id=4, table_id=None - Result tuple

**Type System** (`src/metadata/types.rs`)
- `DataType::from_arrow_type()` - Maps DataFusion → Doris types
- Populates metadata catalog before registering tables
- Reference: `register_in_metadata_catalog()` in `src/catalog/tpch_tables.rs`

## Java FE Reference Points

**Fragment Generation**
- Java: `ThriftPlansBuilder.plansToThrift()`
- Creates TPipelineFragmentParams + TPipelineInstanceParams per fragment
- Scan ranges: `computeDefaultScanSourceParam()` → `perNodeScanRanges`

**Descriptor Table (TARGET STRUCTURE)**
- Java: `DescriptorTable.toThrift()`
- Contains TSlotDescriptor + TTupleDescriptor + TTableDescriptor
- **5-tuple structure**:
  ```
  tuple 0: id=0, table_id=Some(-2004094275)  # Base scan
  tuple 1: id=1, table_id=Some(-2004094275)  # Base scan
  tuple 2: id=2, table_id=None               # Projection
  tuple 3: id=3, table_id=None               # Aggregation
  tuple 4: id=4, table_id=None               # Result
  ```
- Slots distributed across tuples (non-sequential IDs: 4,8,9,16,17,18,19,20...)
- Rust: `encode_descriptor_table_for_table_from_catalog_typed()` needs update

**Plan Nodes**
- OLAP_SCAN_NODE: Rust impl in `encode_olap_scan_node_tpch_lineitem_typed()`
- EXCHANGE_NODE: TODO - needed for proper multi-fragment execution
- RESULT_SINK_NODE: TODO - needed for result aggregation

## Key File Locations

```
src/be/client.rs:186           - exec_pipeline_fragments()
src/be/thrift_pipeline.rs:308  - from_query_plan_multi_fragment()
src/be/thrift_pipeline.rs:919  - TPaloScanRange experimental code (version fix)
src/be/thrift_pipeline.rs:1290 - encode_plan_fragment_for_scan_typed()
src/be/thrift_pipeline.rs:???  - encode_descriptor_table_* (NEEDS 5-TUPLE FIX)
src/catalog/tpch_tables.rs     - register_lineitem() - Metadata catalog
src/metadata/types.rs          - from_arrow_type() - Type conversion
scan_ranges.json               - Tablet metadata config
```

## Current Status & Findings (2025-11-17)

### ✅ Validated Components
1. **TPaloScanRange fields**: version='3', schema_hash correct (experiments done)
2. **Scan ranges loading**: 3 tablets, correct metadata
3. **Fragment structure**: 3 fragments created
4. **MySQL protocol**: Working end-to-end
5. **Type system**: Correct DataFusion → Doris mapping

### ❌ Root Cause: Descriptor Table
**Problem**: Missing 3 tuple descriptors (only 2/5 tuples present)

**Evidence**:
- Experiments proved TPaloScanRange is NOT the issue
- BE still returns 0 rows with correct scan range fields
- Java FE has 5 tuples, Rust FE has 2 tuples
- BE likely skips rows due to missing tuple descriptors

**Fix Required**:
- Add tuples 2, 3, 4 to descriptor table
- Redistribute 18 slots across 5 tuples matching Java pattern
- Location: Descriptor table builder in `thrift_pipeline.rs`

## Current Limitations

1. **Descriptor Table**: Only 2 tuples (needs 5) ← **ROOT CAUSE OF 0 ROWS**
2. **Plan Nodes**: Exchange and result fragments use scan plan as workaround
3. **Single Backend**: Only uses one BE instance (no distribution)
4. **Replica Support**: Only first tablet replica used

## Next Steps (Priority Order)

1. **P0**: Fix descriptor table (add 3 missing tuples) - HIGH CONFIDENCE FIX
2. **P1**: Fill in TOlapScanNode key metadata if needed
3. **P2**: Implement proper EXCHANGE_NODE and result nodes
4. **P3**: Add tablet replica support and multi-backend distribution

---

**Investigation Documents**
- `/tmp/DESCRIPTOR_TABLE_ANALYSIS.md` - Detailed 5-tuple structure analysis
- `/tmp/SESSION_PROGRESS_2025-11-17-CONTINUED.md` - Experiment results

**References**
- Java FE: `fe/fe-core/src/main/java/org/apache/doris/qe/Coordinator.java`
- Java FE: `fe/fe-core/src/main/java/org/apache/doris/planner/ThriftPlansBuilder.java`
- BE: `be/src/service/internal_service.cpp` (exec_plan_fragment_prepare/start)
- BE: `be/src/pipeline/pipeline_fragment_context.cpp`
