// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Minimal Query Planner Example
//!
//! CLAUDE.MD Principle #2: Use Java FE behavior as specification
//! This example builds TPlanFragment for simple SELECT queries,
//! reverse-engineered from:
//! - OlapScanNode.java (lines 1081-1190) - toThrift() method
//! - OlapScanNode.java (lines 441-674) - addScanRangeLocations()
//!
//! Goal: "run until TPC_H fully passed"
//! Progress: Building query execution capability

use fe_catalog::{Catalog, load_tpch_schema};
use fe_planner::thrift_plan::{
    TPlanNode, TPlanNodeType, TOlapScanNode, TPlanFragment, TPlan,
    TPrimitiveType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════");
    println!("  Minimal Query Planner Example");
    println!("  Building TPlanFragment for: SELECT * FROM lineitem LIMIT 10");
    println!("═══════════════════════════════════════════════════════\n");

    // Step 1: Load TPC-H catalog
    println!("Step 1: Loading TPC-H catalog");
    let catalog = Catalog::new();
    load_tpch_schema(&catalog)?;
    println!("✅ Catalog loaded\n");

    // Step 2: Get lineitem table metadata
    println!("Step 2: Retrieving lineitem table metadata");
    let table = catalog.get_table_by_name("tpch", "lineitem")?;
    let table_guard = table.read();

    println!("  Table: {} (ID: {})", table_guard.name, table_guard.id);
    println!("  Columns: {}", table_guard.columns.len());
    println!("  Keys type: {:?}", table_guard.keys_type);
    println!();

    // Step 3: Build key column metadata
    println!("Step 3: Extracting key column metadata");

    let mut key_column_names = Vec::new();
    let mut key_column_types = Vec::new();

    for col in &table_guard.columns {
        if col.is_key {
            key_column_names.push(col.name.clone());
            key_column_types.push(map_data_type_to_primitive(&col.data_type));
        }
    }

    println!("  ✓ Key columns: {:?}", key_column_names);
    println!("  ✓ Key types: {:?}", key_column_types);
    println!();

    // Step 4: Build TPlanNode (OLAP_SCAN_NODE)
    println!("Step 4: Building TPlanNode (OLAP_SCAN_NODE)");

    // Create TOlapScanNode (matching OlapScanNode.toThrift line 1131)
    let olap_scan_node = TOlapScanNode {
        tuple_id: 0, // TupleId for lineitem table
        key_column_name: key_column_names,
        key_column_type: key_column_types,
        is_preaggregation: true, // DUP_KEYS table can use pre-aggregation
        table_name: Some("lineitem".to_string()),
    };

    // Create TPlanNode
    let plan_node = TPlanNode {
        node_id: 0,
        node_type: TPlanNodeType::OlapScanNode,
        num_children: 0,
        limit: 10, // LIMIT 10
        row_tuples: vec![0],
        nullable_tuples: vec![false],
        compact_data: true,
        olap_scan_node: Some(olap_scan_node),
    };

    println!("  ✓ Created TPlanNode:");
    println!("    - Node ID: 0");
    println!("    - Type: OLAP_SCAN_NODE");
    println!("    - Limit: 10");
    println!("    - Tuple IDs: [0]");
    println!();

    // Step 5: Build TPlanFragment
    println!("Step 5: Building TPlanFragment");

    let plan_fragment = TPlanFragment {
        plan: TPlan {
            nodes: vec![plan_node],
        },
    };

    println!("  ✓ Created TPlanFragment with 1 node");
    println!();

    // Step 6: Serialize to JSON (for debugging/comparison with Java FE)
    println!("Step 6: Serializing plan to JSON");

    let json = plan_fragment.to_json()?;
    println!("{}", json);
    println!();

    // Step 7: Summary
    println!("═══════════════════════════════════════════════════════");
    println!("Query Plan Built Successfully!");
    println!("═══════════════════════════════════════════════════════");
    println!("✅ TPlanFragment: READY");
    println!("✅ OLAP_SCAN_NODE: READY");
    println!("✅ Key metadata: READY ({}  key columns)", table_guard.key_columns().len());
    println!("✅ Serialization: WORKING (JSON output above)");
    println!();
    println!("What We Built:");
    println!("- Plan fragment with single OLAP scan node");
    println!("- Lineitem table reference (ID: {})", table_guard.id);
    println!("- Key columns extracted from catalog");
    println!("- LIMIT 10 pushdown");
    println!();
    println!("Next Steps:");
    println!("1. Add TDescriptorTable (column descriptors)");
    println!("2. Add TScanRangeLocations (tablet → backend mapping)");
    println!("3. Send TPlanFragment to BE via gRPC");
    println!("4. Execute query on C++ BE");
    println!("5. Parse PBlock results (parser already complete!)");
    println!();
    println!("Current Blockers:");
    println!("- Need running C++ BE (currently not accessible on port 8060)");
    println!("- Need real tablet locations (query Java FE metadata or mock)");
    println!("- Need TDescriptorTable implementation");
    println!("═══════════════════════════════════════════════════════");

    Ok(())
}

/// Map DataType to TPrimitiveType for key columns
/// Reference: Java FE's Type.getPrimitiveType() method
fn map_data_type_to_primitive(data_type: &fe_common::types::DataType) -> TPrimitiveType {
    use fe_common::types::DataType;

    match data_type {
        DataType::Boolean => TPrimitiveType::Boolean,
        DataType::TinyInt => TPrimitiveType::TinyInt,
        DataType::SmallInt => TPrimitiveType::SmallInt,
        DataType::Int => TPrimitiveType::Int,
        DataType::BigInt => TPrimitiveType::BigInt,
        DataType::Float => TPrimitiveType::Float,
        DataType::Double => TPrimitiveType::Double,
        DataType::Decimal { .. } => TPrimitiveType::DecimalV2,
        DataType::Date => TPrimitiveType::DateV2,
        DataType::DateTime => TPrimitiveType::DateTimeV2,
        DataType::Char { .. } => TPrimitiveType::Char,
        DataType::Varchar { .. } => TPrimitiveType::Varchar,
        DataType::String => TPrimitiveType::String,
        _ => TPrimitiveType::String, // Default fallback
    }
}
