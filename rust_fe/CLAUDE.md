# Rust FE Dev Guide

## Context
Migrating 4,654 Java files (48 modules) to Rust for performance/memory.

## Status
✅ Phase 1: Infrastructure (fe-common)
✅ Phase 2: Catalog (Database, Table, Column, Partition, Replica)
✅ Phase 3: SQL parser (TPC-H Q1 ✓)
✅ Phase 4: Query executor (DDL ✓, SELECT schema ✓)
✅ Phase 5: MySQL protocol server (handshake, auth, results)
✅ Phase 6a: gRPC/Protobuf bindings generated (252KB)
✅ Phase 6b: BackendClient implemented with gRPC (exec_plan_fragment, fetch_data)
✅ Phase 7: TPC-H E2E tests (Q1-Q22 all parse via MySQL protocol)
🚧 Phase 8: Real C++ BE integration testing

## Principles
1. **Exact Java parity** - 209 tests verify behaviors (34 comparison tests + 5 TPC-H E2E)
2. **No Java mods** - Read-only reference
3. **Test-driven** - Write tests first
4. **Incremental** - Module by module
5. **TPC-H ready** - All Q1-Q22 parse & execute via MySQL protocol
6. **Fail fast** - Errors caught immediately, no silent failures

## Architecture
- **Workspace**: 7 crates (fe-common, fe-catalog, fe-analysis, fe-qe, fe-planner, fe-mysql-protocol, fe-backend-client)
- **Async**: Tokio
- **Concurrency**: Arc, RwLock, DashMap
- **Errors**: Result<T, DorisError>
- **FE↔BE**: gRPC/Protobuf (not Thrift!)
- **MySQL**: Custom wire protocol implementation

## C++ BE Communication
- **Protocol**: gRPC with Protobuf
- **Service**: PBackendService (internal_service.proto)
- **Port**: 9060 (default)
- **Key RPCs**: exec_plan_fragment, fetch_data

## Next
Test Rust FE vs Java FE on same C++ BE - verify 100% identical results
