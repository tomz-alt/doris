# Rust FE TODO

## Immediate (Sprint 1)
- [ ] Add unit tests for fe-common (error, config, utils)
- [ ] Add unit tests for fe-catalog (all modules)
- [ ] Add integration tests (catalog operations)
- [ ] Implement missing catalog features:
  - [ ] External tables (MySQL, Elasticsearch, etc.)
  - [ ] Materialized views
  - [ ] Schema change tracking
  - [ ] Tablet/replica management

## Short-term (Sprint 2-3)
- [ ] fe-analysis: SQL parser (sqlparser-rs or custom)
- [ ] fe-analysis: AST nodes and expression analysis
- [ ] fe-analysis: Type coercion and validation
- [ ] fe-persist: Journal/checkpoint for metadata
- [ ] fe-system: Backend/Frontend node tracking

## Medium-term (Sprint 4-8)
- [ ] fe-nereids: Pattern matching framework
- [ ] fe-nereids: Rule-based optimizer
- [ ] fe-nereids: Cost-based optimizer
- [ ] fe-planner: Plan nodes and distributed planning
- [ ] fe-qe: Query execution coordinator

## Long-term (Sprint 9+)
- [ ] fe-service: HTTP/RPC/MySQL services
- [ ] fe-load: Data loading (stream/broker/routine)
- [ ] fe-transaction: 2PC transaction management
- [ ] Performance optimization and profiling
- [ ] Production deployment and migration tools

## Testing
- [ ] Behavior parity tests (Rust vs Java)
- [ ] Performance benchmarks
- [ ] Memory usage profiling
- [ ] Stress testing

See: rust_fe_migration_todos.md for full roadmap
