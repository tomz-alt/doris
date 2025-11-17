# Rust FE Development Guide

## Context
Migrating 4,654 Java files (48 modules) to Rust for better performance and lower memory usage.

## Status
- ✅ Phase 1: Infrastructure (fe-common)
- ✅ Phase 2: Catalog basics (Database, Table, Column, Partition, Replica)
- 🚧 Phase 2: Complete catalog (external tables, MVs, schema changes)
- ⏳ Phase 3+: SQL parser, optimizer, execution

## Key Principles
1. **Behavior Parity**: Match Java FE exactly
2. **No Java Modifications**: Read-only reference
3. **Test-Driven**: 52 tests verify parity (26 unit + 26 integration)
4. **Incremental**: Module by module

## Architecture
- **Workspace**: 21 crates
- **Async**: Tokio
- **Concurrency**: Arc, RwLock, DashMap
- **Errors**: Result<T, DorisError>

## Next Steps
1. Complete catalog features
2. SQL parser (fe-analysis)
3. Query optimizer (fe-nereids)
4. Services (HTTP/RPC/MySQL)

Ref: rust_fe_migration_todos.md
