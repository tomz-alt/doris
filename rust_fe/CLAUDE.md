# Rust FE Dev Guide

## Context
Migrating 4,654 Java files (48 modules) to Rust for performance/memory.

## Status
✅ Phase 1: Infrastructure (fe-common)
✅ Phase 2: Catalog (Database, Table, Column, Partition, Replica)
✅ Phase 3: SQL parser (TPC-H Q1 ✓)
✅ Phase 4: Query executor (DDL ✓, SELECT schema ✓)
✅ Phase 5: MySQL protocol server (handshake, auth, results)
🚧 Phase 6: gRPC client to C++ BE (exec queries with real data)

## Principles
1. **Exact Java parity** - 200 tests verify behaviors (34 comparison tests)
2. **No Java mods** - Read-only reference
3. **Test-driven** - Write tests first
4. **Incremental** - Module by module
5. **TPC-H ready** - Parse, plan & execute on C++ BE via gRPC

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
