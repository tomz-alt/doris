# Query Planner Architecture

## Overview

Rust FE uses Thrift RPC to communicate with existing C++ Backend (BE).
We DON'T need to rewrite query execution - we reuse the existing BE!

## Flow: TPC-H Q1 Example

```sql
SELECT
    l_returnflag,
    l_linestatus,
    sum(l_quantity) as sum_qty,
    sum(l_extendedprice) as sum_base_price,
    avg(l_quantity) as avg_qty,
    count(*) as count_order
FROM lineitem
WHERE l_shipdate <= date '1998-12-01'
GROUP BY l_returnflag, l_linestatus
ORDER BY l_returnflag, l_linestatus;
```

### 1. Parse SQL (✓ Already Done)
- DorisParser → AST (SelectStatement)

### 2. Create Query Plan (NEW - Needed)
- **OlapScanNode**: Scan lineitem table
  - Predicate: l_shipdate <= '1998-12-01'
  - Columns: l_returnflag, l_linestatus, l_quantity, l_extendedprice
- **AggregationNode**: Compute aggregates
  - Group by: l_returnflag, l_linestatus
  - Aggregates: SUM(l_quantity), SUM(l_extendedprice), AVG(l_quantity), COUNT(*)
- **SortNode**: Order by l_returnflag, l_linestatus

### 3. Convert to Thrift (NEW - Needed)
```rust
TPlanFragment {
    plan: TPlan {
        nodes: vec![
            TPlanNode {
                node_type: TPlanNodeType::OLAP_SCAN_NODE,
                olap_scan_node: TOlapScanNode {
                    table_name: "lineitem",
                    key_column_name: ["l_orderkey", "l_partkey"],
                    ...
                },
                ...
            },
            TPlanNode {
                node_type: TPlanNodeType::AGGREGATION_NODE,
                agg_node: TAggregationNode {
                    grouping_exprs: [...],
                    aggregate_functions: [...],
                },
                ...
            },
            ...
        ]
    }
}
```

### 4. Send via Thrift RPC (NEW - Needed)
```rust
// Connect to BE
let be_addr = "127.0.0.1:9060";
let client = BackendServiceClient::new(be_addr)?;

// Execute plan
let request = TPipelineFragmentParams {
    fragment: plan_fragment,
    ...
};
let response = client.exec_plan_fragment(request)?;

// Receive results
let result_rows = response.rows;
```

### 5. BE Executes (EXISTING C++ - No changes!)
- Scans data from tablets
- Applies predicates
- Computes aggregations
- Sorts results
- Returns rows

### 6. Return to MySQL Client (NEW - Needed)
- Encode results in MySQL protocol
- Send result set to client

## Key Thrift Files

### Plan Structure
- `PlanNodes.thrift`: TPlanNode, TOlapScanNode, TAggregationNode, etc.
- `Planner.thrift`: TPlanFragment, TPlan
- `Exprs.thrift`: TExpr (expressions, predicates)
- `Descriptors.thrift`: Table/column descriptors

### RPC Interface
- `BackendService.thrift`: FE → BE RPC service
- `PaloInternalService.thrift`: Internal exec methods

## Implementation Plan

1. **Query Planner (fe-planner)** - Convert AST to logical plan
2. **Thrift Bindings** - Generate Rust bindings from .thrift files
3. **Plan to Thrift** - Convert logical plan → TPlanFragment
4. **RPC Client** - Send to BE via Thrift
5. **Result Decoder** - Decode BE response

## Benefits

✅ Reuse existing, battle-tested C++ BE
✅ Only need FE in Rust (parser, planner, coordinator)
✅ Much smaller scope than full rewrite
✅ Can incrementally improve performance
✅ Existing data files, storage formats work as-is

## What We DON'T Need

❌ Rewrite query execution engine
❌ Rewrite storage engine
❌ Rewrite aggregation/join algorithms
❌ Rewrite data file formats

## Current Status

✅ SQL Parser (TPC-H Q1 parses correctly)
✅ Catalog (tables, columns, metadata)
✅ Query Executor (DDL - CREATE/DROP TABLE)
⏳ Query Planner (AST → Execution Plan)
⏳ Thrift Bindings (Rust bindings for .thrift files)
⏳ Thrift RPC Client (FE → BE communication)
⏳ MySQL Protocol Server (MySQL CLI → Rust FE)

## Next Steps

1. Add `thrift` crate for Rust bindings
2. Generate bindings from PlanNodes.thrift, BackendService.thrift
3. Build simple plan for TPC-H lineitem scan
4. Send to BE via Thrift RPC
5. Verify BE can execute
