# Active TODO List

**Last Updated**: 2025-11-17

## 🎯 Strategic Direction: Systematic Rewrite

**Decision**: Moving from tcpdump/patch approach to systematic Java FE mirroring with unit tests.

**Rationale**:
- ❌ Tcpdump/patch is tedious, error-prone, unsustainable
- ✅ Systematic approach provides better understanding and maintainability
- ✅ Unit tests prevent regressions and validate correctness
- ✅ Can test Thrift generation without BE dependency

## ✅ Recent Completions

- ✅ MySQL protocol timeout (fixed: don't use `--no-default-features`)
- ✅ Type casting error (fixed: populate metadata catalog from DataFusion schema)
- ✅ 3-fragment structure (implemented in thrift_pipeline.rs:308)
- ✅ Plan node framework (fragment-specific encoders created)
- ✅ Enhanced logging (debug → info in exec_pipeline_fragments)
- ✅ TPaloScanRange experiments (version='3' fix, schema_hash tests - NOT root cause)
- ✅ Descriptor table analysis (ROOT CAUSE FOUND: 2 tuples vs 5 tuples required)
- ✅ Strategic decision: Systematic rewrite over tcpdump/patch

## 🔄 Current Status

**Known Issue**: BE returns 0 rows - Descriptor Table has 2/5 tuples

**Analysis Results**:
- Java FE scan fragment: 18 slots + **5 tuple descriptors**
- Rust FE scan fragment: 18 slots + **2 tuple descriptors** ❌
- **Missing**: tuples 2, 3, 4 (likely projection/aggregation/result tuples)

**Evidence**:
- TPaloScanRange fields are correct (version, schema_hash)
- Descriptor table is major structural difference
- Need systematic understanding of Java FE plan generation

## 📋 Systematic Rewrite Plan (Phases 1-4)

### Phase 1: Study Java FE Core Classes (1-2 days)
**Goal**: Deep understanding of Java FE plan generation logic

**Key Files to Study**:
1. ✅ `fe/fe-core/src/main/java/org/apache/doris/planner/DescriptorTable.java`
   - How tuples are created for scan/project/aggregate/result nodes
   - Slot assignment logic and distribution across tuples
   - Table descriptor creation and linkage

2. ✅ `fe/fe-core/src/main/java/org/apache/doris/planner/PlanFragment.java`
   - Fragment creation and dependency management
   - How fragments reference each other (exchange nodes)
   - Fragment validation requirements

3. ✅ `fe/fe-core/src/main/java/org/apache/doris/planner/OlapScanNode.java`
   - Scan node field generation
   - How scan ranges map to plan nodes
   - Key metadata: key_column_name, key_column_type, is_key
   - Relationship between scan node and descriptor table

4. ✅ `fe/fe-core/src/main/java/org/apache/doris/qe/Coordinator.java`
   - `computeFragmentExecParams()` - Backend selection logic
   - `computeScanRangeAssignment()` - Scan range distribution
   - Fragment assembly into TPipelineFragmentParamsList
   - Multi-backend distribution strategy

5. ✅ `fe/fe-core/src/main/java/org/apache/doris/planner/ThriftPlansBuilder.java`
   - `plansToThrift()` - Main conversion logic
   - Plan node conversion (toThrift methods)
   - Fragment parameter construction

**Deliverables**:
- [ ] Document Java FE descriptor table creation logic (markdown notes)
- [ ] Document fragment creation and linking patterns
- [ ] Document scan node generation requirements
- [ ] Identify minimum required fields vs optional fields
- [ ] Create reference implementation pseudocode

### Phase 2: Build Testable Components (2-3 days)
**Goal**: Create unit-testable Rust builders mirroring Java logic

**Components to Build**:

1. **DescriptorTableBuilder** (`src/planner/descriptor_builder.rs`)
   ```rust
   pub struct DescriptorTableBuilder {
       tuples: Vec<TTupleDescriptor>,
       slots: Vec<TSlotDescriptor>,
       tables: Vec<TTableDescriptor>,
   }

   impl DescriptorTableBuilder {
       pub fn add_scan_tuple(&mut self, table_id: i64, columns: &[Column]) -> TupleId;
       pub fn add_projection_tuple(&mut self, source_tuples: &[TupleId]) -> TupleId;
       pub fn add_aggregate_tuple(&mut self) -> TupleId;
       pub fn add_result_tuple(&mut self) -> TupleId;
       pub fn build(&self) -> TDescriptorTable;
   }
   ```

2. **PlanNodeBuilder** (`src/planner/plan_node_builder.rs`)
   ```rust
   pub struct OlapScanNodeBuilder {
       descriptor_table: &DescriptorTableBuilder,
       scan_ranges: Vec<TPaloScanRange>,
   }

   impl OlapScanNodeBuilder {
       pub fn with_table(&mut self, table: &BeTable) -> &mut Self;
       pub fn with_scan_ranges(&mut self, ranges: Vec<TPaloScanRange>) -> &mut Self;
       pub fn build(&self) -> TPlanNode;
   }
   ```

3. **FragmentBuilder** (`src/planner/fragment_builder.rs`)
   ```rust
   pub struct FragmentBuilder {
       fragment_id: i32,
       plan_nodes: Vec<TPlanNode>,
       output_sink: Option<TDataSink>,
   }

   impl FragmentBuilder {
       pub fn with_scan_node(&mut self, node: TPlanNode) -> &mut Self;
       pub fn with_exchange_node(&mut self, source_fragment: i32) -> &mut Self;
       pub fn with_result_sink(&mut self) -> &mut Self;
       pub fn build(&self) -> TPlanFragment;
   }
   ```

4. **ThriftPayloadBuilder** (`src/planner/thrift_builder.rs`)
   ```rust
   pub struct ThriftPayloadBuilder {
       descriptor_table: TDescriptorTable,
       fragments: Vec<TPlanFragment>,
       query_options: TQueryOptions,
   }

   impl ThriftPayloadBuilder {
       pub fn build_pipeline_params(&self) -> TPipelineFragmentParamsList;
   }
   ```

**Deliverables**:
- [ ] Implement DescriptorTableBuilder with full tuple/slot logic
- [ ] Implement OlapScanNodeBuilder matching Java behavior
- [ ] Implement FragmentBuilder for multi-fragment plans
- [ ] Implement ThriftPayloadBuilder for final assembly
- [ ] Add comprehensive inline documentation

### Phase 3: Unit Test Against Java Output (1 day)
**Goal**: Validate Rust implementation matches Java FE exactly

**Test Strategy**:

1. **Capture Java FE Reference Outputs**
   ```bash
   # Add instrumentation to Java FE to dump Thrift structures
   # Location: fe/fe-core/src/main/java/org/apache/doris/qe/Coordinator.java

   # Add before exec_plan_fragment call:
   String debugPayload = TSerializer.serialize(params);
   Files.write(Paths.get("/tmp/java_thrift_payload.json"), debugPayload.getBytes());
   ```

2. **Unit Tests** (`tests/thrift_generation_tests.rs`)
   ```rust
   #[test]
   fn test_descriptor_table_matches_java() {
       let java_output = load_java_baseline("testdata/java_descriptor_table.json");

       let rust_output = DescriptorTableBuilder::new()
           .add_scan_tuple(TABLE_ID, &LINEITEM_COLUMNS)
           .add_projection_tuple(&[0, 1])
           .add_aggregate_tuple()
           .add_result_tuple()
           .build();

       assert_descriptor_tables_equal(&rust_output, &java_output);
   }

   #[test]
   fn test_scan_node_matches_java() { ... }

   #[test]
   fn test_full_pipeline_params_matches_java() { ... }
   ```

3. **Property-Based Testing**
   ```rust
   #[quickcheck]
   fn prop_descriptor_table_valid(columns: Vec<Column>) -> bool {
       let table = DescriptorTableBuilder::new()
           .add_scan_tuple(123, &columns)
           .build();

       validate_descriptor_table(&table).is_ok()
   }
   ```

**Deliverables**:
- [ ] Java FE instrumentation for Thrift payload dumping
- [ ] Baseline test data from Java FE (descriptor table, scan node, full payload)
- [ ] Unit tests for each builder component
- [ ] Integration test for full TPipelineFragmentParamsList
- [ ] Test helper for comparing Thrift structures (ignoring order where appropriate)

### Phase 4: Integration Testing & Refinement (ongoing)
**Goal**: Validate against live BE and iterate

**Integration Tests**:

1. **BE Integration Tests**
   ```bash
   # Test script: tests/integration/be_query_test.sh

   # Start Rust FE
   RUST_LOG=info cargo run --features real_be_proto -- --config fe_config.json &

   # Run test queries
   mysql -h 127.0.0.1 -P 9031 -u root <<EOF
   SELECT * FROM tpch_sf1.lineitem LIMIT 3;
   SELECT COUNT(*) FROM tpch_sf1.lineitem;
   SELECT l_orderkey, SUM(l_quantity) FROM tpch_sf1.lineitem GROUP BY l_orderkey LIMIT 10;
   EOF
   ```

2. **Payload Comparison Tests**
   ```bash
   # Capture live Java FE payload (via tcpdump)
   ./scripts/capture_java_payload.sh > java_live.bin

   # Capture live Rust FE payload
   ./scripts/capture_rust_payload.sh > rust_live.bin

   # Compare using decoder
   cargo test --test payload_comparison -- --nocapture
   ```

3. **Regression Tests**
   - Add tests for each bug fix (descriptor table, type casting, etc.)
   - Prevent future regressions

**Deliverables**:
- [ ] BE integration test suite
- [ ] Live payload comparison framework
- [ ] Regression test suite
- [ ] Performance benchmarks (query latency, BE response time)
- [ ] Documentation for adding new tests

## 🎯 Success Criteria

**Phase 1 Complete When**:
- ✅ Understand tuple creation logic for scan/project/aggregate/result
- ✅ Understand fragment linking and exchange nodes
- ✅ Documented Java FE behavior in markdown

**Phase 2 Complete When**:
- ✅ All builders implemented and documented
- ✅ Code compiles and follows Rust best practices
- ✅ Can generate descriptor table with 5 tuples programmatically

**Phase 3 Complete When**:
- ✅ Unit tests pass comparing against Java baseline
- ✅ Thrift structures match byte-for-byte (or logically equivalent)
- ✅ Property tests validate structural invariants

**Phase 4 Complete When**:
- ✅ `SELECT * FROM lineitem LIMIT 3` returns 3 rows of data
- ✅ Live payload matches Java FE payload
- ✅ No BE errors in logs
- ✅ Integration tests pass consistently

## 📊 Timeline & Estimates

**Total Estimated Time**: 4-6 days of focused work

- Phase 1 (Study): 1-2 days
- Phase 2 (Build): 2-3 days
- Phase 3 (Test): 1 day
- Phase 4 (Integration): Ongoing validation

**Recommended Approach**:
- Work through phases sequentially
- Complete all deliverables before moving to next phase
- Use tcpdump/live testing in Phase 4 for validation only
- Keep detailed notes in markdown files for each phase

## 📝 Key Files

**Current Implementation**:
- `src/be/thrift_pipeline.rs` - Current Thrift generation (to be refactored)
- `src/be/client.rs` - BE gRPC communication
- `src/planner/fragment_executor.rs` - Query execution
- `src/catalog/be_table.rs` - BE-backed tables
- `src/metadata/types.rs` - Type system

**New Components (Phase 2)**:
- `src/planner/descriptor_builder.rs` - DescriptorTableBuilder (to be created)
- `src/planner/plan_node_builder.rs` - Plan node builders (to be created)
- `src/planner/fragment_builder.rs` - FragmentBuilder (to be created)
- `src/planner/thrift_builder.rs` - ThriftPayloadBuilder (to be created)

**Test Infrastructure (Phase 3)**:
- `tests/thrift_generation_tests.rs` - Unit tests (to be created)
- `tests/integration/be_query_test.sh` - Integration tests (to be created)
- `testdata/` - Java FE baseline data (to be captured)

**Configuration & Data**:
- `scan_ranges.json` - Tablet metadata config
- `fe_config.json` - FE server configuration

**Documentation**:
- `JAVA_FE_RESEARCH.md` - Phase 1 research notes (to be created)
- `SYSTEMATIC_REWRITE_PROGRESS.md` - Phase-by-phase progress (to be created)

## 📚 Reference Documents

**Investigation History** (tcpdump/patch era):
- `DESCRIPTOR_TABLE_ANALYSIS.md` - Root cause analysis (5 tuples vs 2)
- `SESSION_PROGRESS_2025-11-17-CONTINUED.md` - Experiment results
- `TSHARK_FINDINGS_AND_ACTION_PLAN.md` - Payload comparison findings
- `SUCCESS_REPORT.md` - Previous achievements

**Java FE Reference Locations**:
- `fe/fe-core/src/main/java/org/apache/doris/planner/DescriptorTable.java`
- `fe/fe-core/src/main/java/org/apache/doris/planner/OlapScanNode.java`
- `fe/fe-core/src/main/java/org/apache/doris/planner/PlanFragment.java`
- `fe/fe-core/src/main/java/org/apache/doris/qe/Coordinator.java`
- `fe/fe-core/src/main/java/org/apache/doris/planner/ThriftPlansBuilder.java`

---

**Next Session**: Start Phase 1 - Study Java FE DescriptorTable.java
**Current Focus**: Systematic rewrite with unit tests (no more tcpdump/patch)
