# Rust FE → Doris BE: Full Java FE Migration TODOs

This file tracks high-level TODOs for making the Rust FE a drop-in replacement for the Java FE, across **any schema / database**, aligned with `AGENTS.md`.

---

## P1 – Metadata & Catalog (AGENTS #1, #2)

**Goal:** Make Rust FE’s catalog fully dynamic and Doris-authored, so all schemas and tablet layouts come from Doris metadata, not hardcoded TPCH definitions.

- **Single source of truth for schema**
  - Introduce a `CatalogBackend` trait with:
    - `InMemoryCatalog` (current TPCH in-memory catalog).
    - `DorisCatalog` (backed by Doris FE/BE metadata RPCs).
  - Implement `DorisCatalog` methods:
    - `list_databases()`, `list_tables(db)`.
    - `get_table(db, table)` → full schema (columns, types, keys, distribution, partitions, indexes).
    - Cache + invalidate on DDL.

- **Tablet / partition layout integration**
  - Extend catalog entries to carry:
    - `db_id`, `table_id`, `index_id`.
    - `partition_ids`, `tablet_ids` and their routing info.
  - Replace hardcoded IDs in:
    - `BeLoadHelper::build_tpch_lineitem_schema_param`.
    - `BackendClientPool::load_tpch_lineitem` and future generic load helpers.
    - `PipelineFragmentParamsList::from_query_plan` (scan node → tablets).

- **Validation against Java FE**
  - Add catalog tests that:
    - Fetch schema via Rust FE `DorisCatalog`.
    - Fetch schema via Java FE (SHOW CREATE TABLE / DESC).
    - Compare column names, types, nullability, keys for a sample of tables across multiple DBs.

---

## P2 – Generic BE Encoding for Load & Scan (AGENTS #1, #3)

**Goal:** Generalize descriptors, Thrift plans, and `PBlock` encoding/decoding so Rust FE can load and scan **any** Doris table using the same contracts as Java FE.

- **Descriptor + Thrift plan generalization (read path)**
  - Extend `PipelineFragmentParamsList::from_query_plan` to:
    - Build `TDescriptorTable` from catalog metadata for arbitrary tables (not just `tpch.lineitem`).
    - Handle multiple indexes and partitioned tables.
    - Encode scan tablets and partition pruning decisions from catalog layout.
  - Add golden tests:
    - Capture `TPipelineFragmentParamsList` from Java FE for a set of queries.
    - Generate the same from Rust FE `QueryPlan`.
    - Compare key fields (nodes, scan ranges, limits, predicates, sort keys, tablet lists).

- **PBlock encoder (write path)**
  - Implement a `PBlockBuilder` in Rust that:
    - Accepts a table schema (Doris types) and a row/columnar representation (e.g. from `ParsedInsert` or a typed batch).
    - Produces `PBlock` with:
      - `column_metas` matching Doris vectorized types.
      - `column_values` following `Block::serialize` conventions for:
        - Core numeric types (TINY/SMALL/INT/BIGINT, FLOAT/DOUBLE).
        - DECIMAL.
        - DATE / DATETIME / TIMESTAMP.
        - CHAR / VARCHAR / STRING.
  - Wire into load path:
    - Replace `build_empty_add_block_request` with `build_add_block_request_with_pblock`.
    - Make `BackendClientPool::load_*` use the real PBlock builder.
  - Validation:
    - Unit tests in Rust that:
      - Build a small block (1–2 rows, multiple types).
      - Compare `column_metas` to expectations from Doris schema.
    - Cross-language tests:
      - Feed a Rust-built `PBlock` into a tiny C++ helper that calls `Block::deserialize` and validates decoded values.
      - Golden PBlock bytes captured from Java FE load path for specific tables.

- **Result decoding generalization (read path)**
  - Ensure `BackendClientPool::execute_pipeline_query`:
    - Builds MySQL `ColumnDefinition`s for all Doris scalar types used in production.
    - Uses `decode_mysql_text_row_batch` to decode `TResultBatch` into `QueryResult` rows for any schema.
  - Add differential tests:
    - Run the same SELECT via Java FE and Rust FE against a shared BE.
    - Compare MySQL result sets (types, column order, row values, NULLs).

---

## P3 – Query Surface & Pipeline Fragments (AGENTS #1, #2)

**Goal:** Support the same SQL surface as Java FE by mapping logical plans to Doris pipeline fragments for all core query shapes.

- **PlanConverter and QueryPlan coverage**
  - Incrementally extend `PlanConverter` + `QueryPlan` + `PipelineFragmentParamsList::from_query_plan` for:
    - Projections, filters, and limits (SELECT … WHERE … LIMIT …).
    - Single-table aggregations and GROUP BY.
    - ORDER BY and TOP-N.
    - Joins:
      - Broadcast / shuffle join strategies.
      - Multi-fragment plans with exchange nodes.
  - For each new shape:
    - Capture Java FE fragment graph for a representative query.
    - Generate Rust FE graph for the same SQL.
    - Compare fragments, exchange patterns, and operator nodes (not necessarily byte-for-byte, but semantically equivalent).

- **Execution policies & resource control**
  - Mirror Java FE behavior for:
    - Fragment parallelism and pipeline degree.
    - Scan ranges per tablet, split thresholds, and locality hints.
    - Query options: memory limits, timeouts, workload groups.
  - Expose these via Rust FE config, while keeping:
    - `execute_sql` + planner + catalog unaware of underlying RPC/gRPC details.
    - All Thrift / proto specifics hidden behind `be::client`, `be::thrift_pipeline`, and `be::load` (AGENTS #3).

---

## P4 – Semantics, Transactions, Observability (AGENTS #2, #4)

**Goal:** Match Java FE user-visible semantics and operational behavior, including loads, transactions, and debugging experience.

- **Transaction + load job semantics**
  - Implement load job behavior equivalent to Java FE:
    - Label handling, idempotency, deduplication.
    - Transaction lifecycle for stream load / INSERT (begin, commit, abort).
    - Proper accounting of loaded, filtered, and unselected rows.
  - Tests:
    - Run the same labeled load via Java FE and Rust FE.
    - Compare final job state, FE-visible metrics, and row counts.

- **Operational observability**
  - Add structured logging and metrics for:
    - Thrift payload sizes (fragment params, descriptor tables, PBlocks).
    - BE RPC latencies and error codes (open, add_block, fetch_data, cancel).
    - Query planning and queueing (extending `QueryQueue` metrics).
  - Provide “wire debug” tooling (behind a feature flag):
    - Dump Thrift/Proto payloads to disk for selected queries / loads.
    - Compare against Java FE payloads for the same operations.

- **Protocol abstraction hardening**
  - Maintain a strict boundary:
    - Core: `execute_sql` + parser + catalog + planner + `QueryPlan`.
    - BE edge: `BackendClientPool`, `thrift_pipeline`, `load` modules encapsulating all gRPC/Thrift details.
  - Ensure new functionality (more load modes, more protocols like pgwire) always goes through this core API, not directly to BE.

---

## Suggested Execution Order

1. **P1 – Doris-backed catalog + tablet layout** for any DB/table.
2. **P2 – Generic descriptor + PBlock encoder**, both load and scan, covering core types.
3. **P3 – Expand `QueryPlan` → pipeline fragments** to full SQL surface (joins, aggregates, multi-fragment).
4. **P4 – Transaction semantics, metrics/logging, Java FE diff tests** for behavior and performance.

This ordering keeps the core boundary clean, uses Java FE as the behavioral spec, and isolates transport details according to `AGENTS.md`, while gradually expanding from TPCH-only to “any schema / any database” with production semantics.***
