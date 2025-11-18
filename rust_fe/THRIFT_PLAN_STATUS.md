# Thrift Plan Generation - Current Status

## Completed ✓

### 1. Thrift Structure Definitions
- **TPlanNodeType**: 35 node types (OLAP scan, aggregation, sort, etc.)
- **TPrimitiveType**: 38 data types (Int, Varchar, Decimal, Date, etc.)
- **TPlanNode**: Core plan node with required fields:
  - node_id, node_type, num_children, limit
  - row_tuples, nullable_tuples
  - compact_data (required field)
  - olap_scan_node (optional)

- **TOlapScanNode**: OLAP table scan with:
  - tuple_id
  - key_column_name (required)
  - key_column_type (required - **newly added**)
  - is_preaggregation
  - table_name (optional)

- **TPlanFragment**: Top-level plan container
- **TPlan**: Collection of plan nodes

### 2. Query Planner
- **QueryPlanner**: Converts AST to execution plans
  - plan_lineitem_scan(): Creates OLAP scan for TPC-H lineitem
  - create_olap_scan_node(): Generates scan nodes from catalog
  - Reads table metadata (columns, types, keys)

- **Type Mapping**: DataType → TPrimitiveType conversion
  - Maps all 23 Doris types to Thrift primitive types
  - Handles parameterized types (Decimal, Varchar, etc.)

### 3. Serialization
- **JSON Output**: Human-readable plan inspection
- **Test Coverage**: 9 tests (5 thrift_plan + 4 planner)

### 4. Example Output
```json
{
  "plan": {
    "nodes": [{
      "node_id": 0,
      "node_type": "OlapScanNode",
      "num_children": 0,
      "limit": -1,
      "row_tuples": [0],
      "nullable_tuples": [false],
      "compact_data": true,
      "olap_scan_node": {
        "tuple_id": 0,
        "key_column_name": ["L_ORDERKEY"],
        "key_column_type": ["Int"],
        "is_preaggregation": true,
        "table_name": "lineitem"
      }
    }]
  }
}
```

## Next Steps

### Phase 1: Thrift Binary Protocol (Optional for MVP)
- Generate Rust bindings from .thrift files using Thrift compiler
- Implement binary serialization/deserialization
- Compare byte-for-byte with Java FE output

### Phase 2: RPC to BE (Required for execution)
- Implement Thrift RPC client
- Connect to BE (default: 127.0.0.1:9060)
- Send TPipelineFragmentParams (not just TPlanFragment)
- Handle BE response

### Phase 3: Full Query Support (Required for TPC-H)
- Add aggregation node planning (GROUP BY)
- Add sort node planning (ORDER BY)
- Add expression evaluation (WHERE, SELECT)
- Add TPC-H Q1 full query planning

## Immediate Priority: MySQL Protocol

To enable MySQL CLI connections (user's requirement), focus on:
1. MySQL handshake protocol
2. Query command handling (COM_QUERY)
3. Result set encoding
4. Authentication

This allows interactive testing without complex RPC setup.

## Test Strategy

```
Phase 1 (Current):
  MySQL CLI → Rust FE → DDL (CREATE TABLE) → Catalog ✓

Phase 2 (Next):
  MySQL CLI → Rust FE → DQL (SELECT) → Plan → JSON output ✓

Phase 3 (Future):
  MySQL CLI → Rust FE → DQL → Plan → Thrift RPC → BE → Results

Phase 4 (Final):
  MySQL CLI → Rust FE → TPC-H Q1 → BE → Verify vs Java FE
```

## Files

- `fe-planner/src/thrift_plan.rs`: Thrift structure definitions
- `fe-planner/src/planner.rs`: Query planner implementation
- `/tmp/rust_plan.json`: Generated plan output (test artifact)
