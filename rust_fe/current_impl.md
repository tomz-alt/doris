# Current Implementation

## fe-common (Foundation)
**Files**: error.rs, config.rs, types.rs, utils.rs, constants.rs, version.rs

- DorisError (13 variants), Result<T>
- Config (TOML-based, 20+ fields)
- Types: TableId, DbId, etc. (9 ID types)
- Enums: TableType, KeysType, DataType, etc.
- Utils: timestamps, size parsing
- **Tests**: 4 unit tests

## fe-catalog (Metadata)
**Files**: catalog.rs, database.rs, table.rs, column.rs, partition.rs, index.rs, replica.rs

- **Catalog**: Root manager (DashMap<DbName, Database>)
- **Database**: Metadata + table map
- **OlapTable**: columns, partitions, indexes, KeysType
- **Column**: type, nullable, agg_type, default
- **Partition**: version, tablets
- **Index**: base/rollup/bitmap/inverted
- **Replica**: state, health tracking
- **Tests**: 48 integration tests

## fe-main (Entry Point)
**File**: main.rs

- CLI (--config, --log-level, --daemon)
- Config loading, logging, signals
- Catalog init

## Design Patterns
- DashMap: #[serde(skip)] for concurrent maps
- Arc<RwLock<T>>: shared mutable state
- Vec<Column> not Arc<Column>: minimize overhead
- &Column refs: avoid clones

## Stub Crates (18)
analysis, nereids, planner, qe, service, rpc, persist, system, statistics, load, transaction, mysql-protocol, http, alter, datasource, ha, metric, job
