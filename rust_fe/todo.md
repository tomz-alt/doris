# TODO

## Completed ✓
- [x] SQL parser (sqlparser-rs 0.49)
- [x] AST definitions (DDL, DML, SELECT)
- [x] Query planner (TPlanFragment, OLAP scan)
- [x] MySQL protocol server (handshake, auth, commands, results)
- [x] Query executor (CREATE/DROP TABLE/DB, USE, SELECT schema)
- [x] TPC-H Q1 parsing and schema validation
- [x] 200 tests (166 base + 34 comparison)
- [x] BE client crate with MockBackend (6 tests)
- [x] Comparison test suite (34 tests verify Rust FE matches Java FE)
- [x] BE integration test plan documented
- [x] Additional comparison tests (duplicate errors, TPC-H Q2/Q3/Q6, complex expressions)

## Now - C++ BE Integration (Blocked by protoc)
- [ ] **C++ BE Integration** (see BE_INTEGRATION_TEST_PLAN.md)
  - [x] Document gRPC/Protobuf requirements
  - [x] Create fe-backend-client crate
  - [x] Create MockBackend for testing without protoc (6 tests ✓)
  - [x] Document integration test scenarios
  - [ ] ⚠️ **BLOCKED**: Install protoc (apt-get fails in sandbox)
  - [ ] Generate Rust bindings from protobuf
  - [ ] Implement exec_plan_fragment RPC (gRPC)
  - [ ] Implement fetch_data RPC (gRPC)
  - [ ] Test with real C++ BE
  - [ ] Verify 100% identical results vs Java FE
- [ ] Tablet metadata from BE
- [ ] Partition routing

## Next - Full Query Execution
- [ ] Expression evaluation (arithmetic, comparison)
- [ ] Aggregation execution (SUM, AVG, COUNT)
- [ ] GROUP BY implementation
- [ ] ORDER BY implementation
- [ ] Filter pushdown

## Later - Advanced Features
- [ ] External tables (MySQL, ES, Hive, Iceberg)
- [ ] Materialized views
- [ ] Schema changes (ALTER TABLE)
- [ ] Metadata persist (journal/checkpoint)
- [ ] BE/FE node tracking
- [ ] Nereids optimizer integration
- [ ] Data loading (STREAM LOAD, BROKER LOAD)
- [ ] Transactions
