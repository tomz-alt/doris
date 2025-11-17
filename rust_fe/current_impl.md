# Current Implementation

## fe-common (Foundation) - 4 tests ✓
- DorisError (13 variants), Result<T>
- Config (TOML, 20+ fields)
- Types: IDs (9), enums (TableType, KeysType, DataType, etc.)
- Utils: timestamps, size parsing/formatting

## fe-catalog (Metadata) - 63 tests ✓
**Files**: catalog.rs, database.rs, table.rs, column.rs, partition.rs, index.rs, replica.rs

- **Catalog**: Root manager (create/drop DB/table)
- **Database**: Quotas, properties, table map (DashMap)
- **OlapTable**: columns, partitions, indexes, KeysType (AGG/DUP/UNIQUE)
- **Column**: types, nullable, agg_type, defaults, positions
- **Partition**: versions, tablets, temp flag, data size
- **Index**: base/rollup/bitmap/inverted types
- **Replica**: state machine, version tracking, health

**Tests**:
- catalog_tests (7): CRUD, concurrent
- column_tests (6): Types, aggregates
- partition_tests (5): Versions
- replica_tests (4): State, health
- database_extended (6): Register, ordering
- column_extended (11): All types/aggs
- table_tests (9): Keys, storage
- edge_case_tests (15): Quotas, boundaries, concurrency

## fe-main (Entry) - 0 tests
CLI, config, logging, signals

## Design
- DashMap #[serde(skip)]
- Arc<RwLock<T>> for shared state
- &Column refs vs Arc<Column>
- Thread-safe by default
