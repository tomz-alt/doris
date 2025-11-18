# TODO

## Completed ✓
- [x] SQL parser (sqlparser-rs 0.49)
- [x] AST definitions (DDL, DML, SELECT)
- [x] Query planner (TPlanFragment, OLAP scan)
- [x] MySQL protocol server (handshake, auth, commands, results)
- [x] Query executor (CREATE/DROP TABLE/DB, USE, SELECT schema)
- [x] TPC-H Q1 parsing and schema validation
- [x] 166 tests across all modules

## Now - Java BE Integration
- [ ] **Java BE Integration Test** (see JAVA_BE_INTEGRATION_TEST.md)
  - [ ] Thrift RPC client to Java BE
  - [ ] Send query plans to BE
  - [ ] Receive and parse results from BE
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
