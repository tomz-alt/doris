// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Thrift Binary Serialization
//!
//! Converts plan structures to Thrift binary format for BE communication.

use crate::thrift_plan::*;
use fe_common::{DorisError, Result};
use thrift::protocol::{
    TCompactOutputProtocol, TFieldIdentifier, TListIdentifier, TOutputProtocol, TStructIdentifier,
    TType,
};
use thrift::transport::TBufferChannel;

/// Serialize a TPlanFragment to Thrift binary format
pub fn serialize_plan_fragment(fragment: &TPlanFragment) -> Result<Vec<u8>> {
    let mut transport = TBufferChannel::with_capacity(0, 1024);
    let mut protocol = TCompactOutputProtocol::new(&mut transport);

    // Write struct: TPlanFragment
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPlanFragment"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: plan (TPlan)
    protocol
        .write_field_begin(&TFieldIdentifier::new("plan", TType::Struct, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    write_plan(&mut protocol, &fragment.plan)?;

    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    protocol
        .write_field_stop()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    protocol
        .write_struct_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Extract bytes from transport
    Ok(transport.write_bytes().to_vec())
}

/// Write TPlan structure
fn write_plan<P: TOutputProtocol>(protocol: &mut P, plan: &TPlan) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPlan"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: nodes (list<TPlanNode>)
    protocol
        .write_field_begin(&TFieldIdentifier::new("nodes", TType::List, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    protocol
        .write_list_begin(&TListIdentifier::new(TType::Struct, plan.nodes.len() as i32))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    for node in &plan.nodes {
        write_plan_node(protocol, node)?;
    }

    protocol
        .write_list_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    protocol
        .write_field_stop()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    protocol
        .write_struct_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    Ok(())
}

/// Write TPlanNode structure
fn write_plan_node<P: TOutputProtocol>(protocol: &mut P, node: &TPlanNode) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPlanNode"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: node_id (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("node_id", TType::I32, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(node.node_id)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: node_type (TPlanNodeType as i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("node_type", TType::I32, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(node.node_type as i32)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 3: num_children (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("num_children", TType::I32, 3))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(node.num_children)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 4: limit (i64)
    protocol
        .write_field_begin(&TFieldIdentifier::new("limit", TType::I64, 4))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i64(node.limit)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 5: row_tuples (list<i32>)
    protocol
        .write_field_begin(&TFieldIdentifier::new("row_tuples", TType::List, 5))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(TType::I32, node.row_tuples.len() as i32))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for tuple_id in &node.row_tuples {
        protocol
            .write_i32(*tuple_id)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }
    protocol
        .write_list_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 6: nullable_tuples (list<bool>)
    protocol
        .write_field_begin(&TFieldIdentifier::new("nullable_tuples", TType::List, 6))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(
            TType::Bool,
            node.nullable_tuples.len() as i32,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for nullable in &node.nullable_tuples {
        protocol
            .write_bool(*nullable)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }
    protocol
        .write_list_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 7: compact_data (bool)
    protocol
        .write_field_begin(&TFieldIdentifier::new("compact_data", TType::Bool, 7))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_bool(node.compact_data)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 8: olap_scan_node (optional TOlapScanNode)
    if let Some(olap_scan) = &node.olap_scan_node {
        protocol
            .write_field_begin(&TFieldIdentifier::new("olap_scan_node", TType::Struct, 8))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        write_olap_scan_node(protocol, olap_scan)?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    protocol
        .write_field_stop()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    protocol
        .write_struct_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    Ok(())
}

/// Write TOlapScanNode structure
fn write_olap_scan_node<P: TOutputProtocol>(
    protocol: &mut P,
    node: &TOlapScanNode,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TOlapScanNode"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: tuple_id (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("tuple_id", TType::I32, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(node.tuple_id)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: key_column_name (list<string>)
    protocol
        .write_field_begin(&TFieldIdentifier::new("key_column_name", TType::List, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(
            TType::String,
            node.key_column_name.len() as i32,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for name in &node.key_column_name {
        protocol
            .write_string(name)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }
    protocol
        .write_list_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 3: key_column_type (list<TPrimitiveType as i32>)
    protocol
        .write_field_begin(&TFieldIdentifier::new("key_column_type", TType::List, 3))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(
            TType::I32,
            node.key_column_type.len() as i32,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for type_id in &node.key_column_type {
        protocol
            .write_i32(*type_id as i32)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }
    protocol
        .write_list_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 4: is_preaggregation (bool)
    protocol
        .write_field_begin(&TFieldIdentifier::new("is_preaggregation", TType::Bool, 4))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_bool(node.is_preaggregation)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 5: table_name (optional string)
    if let Some(table_name) = &node.table_name {
        protocol
            .write_field_begin(&TFieldIdentifier::new("table_name", TType::String, 5))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_string(table_name)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    protocol
        .write_field_stop()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    protocol
        .write_struct_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_empty_plan() {
        let fragment = TPlanFragment {
            plan: TPlan { nodes: Vec::new() },
        };

        let bytes = serialize_plan_fragment(&fragment).unwrap();
        assert!(!bytes.is_empty(), "Serialized bytes should not be empty");
        println!("Serialized {} bytes", bytes.len());
    }

    #[test]
    fn test_serialize_scan_node() {
        let fragment = TPlanFragment {
            plan: TPlan {
                nodes: vec![TPlanNode {
                    node_id: 0,
                    node_type: TPlanNodeType::OlapScanNode,
                    num_children: 0,
                    limit: -1,
                    row_tuples: vec![0],
                    nullable_tuples: vec![false],
                    compact_data: true,
                    olap_scan_node: Some(TOlapScanNode {
                        tuple_id: 0,
                        key_column_name: vec!["l_orderkey".to_string()],
                        key_column_type: vec![TPrimitiveType::BigInt],
                        is_preaggregation: false,
                        table_name: Some("lineitem".to_string()),
                    }),
                }],
            },
        };

        let bytes = serialize_plan_fragment(&fragment).unwrap();
        assert!(!bytes.is_empty());
        println!("Serialized plan with scan node: {} bytes", bytes.len());
    }
}
