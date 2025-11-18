# Rust FE Dev Guide

## Context
Migrating 4,654 Java files (48 modules) to Rust for performance/memory.

## Status
✅ Phase 1: Infrastructure (fe-common)
✅ Phase 2: Catalog (Database, Table, Column, Partition, Replica)
✅ Phase 3: SQL parser (TPC-H Q1 ✓)
✅ Phase 4: Query executor (DDL ✓)
🚧 Phase 5: Query planner + Thrift RPC to BE

## Principles
1. **Exact Java parity** - 130 tests verify behaviors
2. **No Java mods** - Read-only reference
3. **Test-driven** - Write tests first
4. **Incremental** - Module by module
5. **TPC-H ready** - Parse, plan & send to BE via Thrift

## Architecture
- **Workspace**: 21 crates
- **Async**: Tokio
- **Concurrency**: Arc, RwLock, DashMap
- **Errors**: Result<T, DorisError>

## Next
External tables, MVs, serialization, SQL parser
