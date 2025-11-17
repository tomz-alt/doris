# Rust FE Development Guide

## Context
Migrating Apache Doris Frontend from Java (4,654 files, 48 modules) to Rust for better performance and lower memory usage.

## Current Status
- ✅ Phase 1: Infrastructure (fe-common) - DONE
- ✅ Phase 2: Catalog basics (fe-catalog) - DONE
- 🚧 Phase 2: Complete catalog functionality
- ⏳ Phase 3+: SQL parsing, optimizer, execution engine

## Key Principles
1. **Behavior Parity**: Rust FE must behave EXACTLY like Java FE
2. **No Java Modifications**: Only read Java code for reference
3. **Test-Driven**: Write tests comparing Rust vs Java behavior
4. **Incremental**: Migrate module by module

## Architecture
- **Workspace**: Multi-crate structure (21 crates)
- **Async**: Tokio runtime
- **Concurrency**: Arc, RwLock, DashMap
- **Error Handling**: Result<T, DorisError>

## Testing Strategy
1. Reference tests in Java FE (read-only)
2. Unit tests for each Rust module
3. Integration tests for cross-module behavior
4. Behavior parity tests (Rust vs Java)

## Next Steps (Priority Order)
1. Add comprehensive tests for fe-catalog
2. Implement missing catalog features (external tables, etc.)
3. Start fe-analysis (SQL parser)
4. Implement fe-nereids (query optimizer)

See: rust_fe_migration_todos.md, current_impl.md, todo.md
