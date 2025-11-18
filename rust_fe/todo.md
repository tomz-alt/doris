# TODO

## Completed ✓
- [x] SQL parser (sqlparser-rs 0.49)
- [x] AST definitions (DDL, DML, SELECT)
- [x] Query planner (TPlanFragment, OLAP scan)
- [x] MySQL protocol server (handshake, auth, commands, results)
- [x] Query executor (CREATE/DROP TABLE/DB, USE, SELECT schema)
- [x] TPC-H Q1 parsing and schema validation
- [x] 209 tests (171 base + 34 comparison + 5 TPC-H E2E - 1 ignored)
- [x] BE client crate with MockBackend (6 tests)
- [x] Comparison test suite (34 tests verify Rust FE matches Java FE)
- [x] BE integration test plan documented
- [x] Additional comparison tests (duplicate errors, TPC-H Q2/Q3/Q6, complex expressions)
- [x] TPC-H E2E test suite (Q1-Q22 all parse successfully via MySQL protocol)
- [x] Real BackendClient implementation using generated gRPC types

## Now - C++ BE Integration (**COMPLETE** - Ready for real BE!)
- [x] **C++ BE Integration** (see BE_INTEGRATION_TEST_PLAN.md & TESTING_WITH_REAL_BE.md)
  - [x] Document gRPC/Protobuf requirements
  - [x] Create fe-backend-client crate
  - [x] Create MockBackend for testing without protoc (6 tests ✓)
  - [x] Document integration test scenarios
  - [x] Document real BE testing workflow (TESTING_WITH_REAL_BE.md)
  - [x] ✅ **UNBLOCKED**: Install protoc v30.2 (binary provided by user!)
  - [x] Generate Rust bindings from protobuf (252KB doris.rs + 30KB segment_v2.rs)
  - [x] Implement real BackendClient using generated gRPC types ✓
  - [x] Implement exec_plan_fragment RPC (gRPC) ✓
  - [x] Implement fetch_data RPC (gRPC) ✓
  - [ ] Test with real C++ BE (needs running BE on port 8060)
  - [ ] Verify 100% identical results vs Java FE (needs full setup)
- [ ] Tablet metadata from BE
- [ ] Partition routing

## Alternative: Continue Without C++ BE
Since C++ BE integration is blocked by protoc installation, we can:
- [ ] Add more comparison tests (expand from 34 to 50+)
- [ ] Implement SHOW commands for MySQL compatibility
- [ ] Add more TPC-H query parsing tests (Q1-Q22)
- [ ] Improve error messages to match Java FE exactly
- [ ] Performance benchmarks on parsing/planning
- [ ] Documentation for deployment without C++ BE

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
