// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Thrift Binary Serialization
//!
//! Converts plan structures to Thrift binary format for BE communication.

use crate::thrift_plan::*;
use fe_common::{DorisError, Result};
use thrift::protocol::{
    TCompactOutputProtocol, TFieldIdentifier, TListIdentifier, TMapIdentifier, TOutputProtocol,
    TStructIdentifier, TType,
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

// ============================================================================
// Pipeline Execution Serialization (VERSION_3)
// Reference: PaloInternalService.thrift, Coordinator.java:3185-3350
// ============================================================================

/// Serialize TPipelineFragmentParamsList to Thrift binary format
/// Reference: Java FE Coordinator.java:3185+ (toThrift method)
pub fn serialize_pipeline_params(params: &TPipelineFragmentParamsList) -> Result<Vec<u8>> {
    let mut transport = TBufferChannel::with_capacity(0, 4096);
    let mut protocol = TCompactOutputProtocol::new(&mut transport);

    // Write struct: TPipelineFragmentParamsList
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPipelineFragmentParamsList"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: params_list (list<TPipelineFragmentParams>)
    protocol
        .write_field_begin(&TFieldIdentifier::new("params_list", TType::List, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(
            TType::Struct,
            params.params_list.len() as i32,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    for param in &params.params_list {
        write_pipeline_fragment_params(&mut protocol, param)?;
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

    Ok(transport.write_bytes().to_vec())
}

/// Write TPipelineFragmentParams structure
/// Field IDs from PaloInternalService.thrift
fn write_pipeline_fragment_params<P: TOutputProtocol>(
    protocol: &mut P,
    params: &TPipelineFragmentParams,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPipelineFragmentParams"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: protocol_version (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("protocol_version", TType::I32, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(params.protocol_version)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: query_id (TUniqueId)
    protocol
        .write_field_begin(&TFieldIdentifier::new("query_id", TType::Struct, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_unique_id(protocol, &params.query_id)?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 4: per_exch_num_senders (map<i32, i32>)
    protocol
        .write_field_begin(&TFieldIdentifier::new(
            "per_exch_num_senders",
            TType::Map,
            4,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_map_begin(&TMapIdentifier::new(
            TType::I32,
            TType::I32,
            params.per_exch_num_senders.len() as i32,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for (k, v) in &params.per_exch_num_senders {
        protocol
            .write_i32(*k)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(*v)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }
    protocol
        .write_map_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 8: num_senders (optional i32) - MUST come before field 10+
    if let Some(num_senders) = params.num_senders {
        protocol
            .write_field_begin(&TFieldIdentifier::new("num_senders", TType::I32, 8))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(num_senders)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 10: coord (optional TNetworkAddress)
    if let Some(coord) = &params.coord {
        protocol
            .write_field_begin(&TFieldIdentifier::new("coord", TType::Struct, 10))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        write_network_address(protocol, coord)?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 23: fragment (optional TPlanFragment)
    protocol
        .write_field_begin(&TFieldIdentifier::new("fragment", TType::Struct, 23))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_plan_fragment(protocol, &params.fragment)?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 24: local_params (list<TPipelineInstanceParams>) - REQUIRED!
    protocol
        .write_field_begin(&TFieldIdentifier::new("local_params", TType::List, 24))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(
            TType::Struct,
            params.local_params.len() as i32,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for local_param in &params.local_params {
        write_pipeline_instance_params(protocol, local_param)?;
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

/// Write TPlanFragment structure (reuse for pipeline params)
fn write_plan_fragment<P: TOutputProtocol>(
    protocol: &mut P,
    fragment: &TPlanFragment,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPlanFragment"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: plan (TPlan)
    protocol
        .write_field_begin(&TFieldIdentifier::new("plan", TType::Struct, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_plan(protocol, &fragment.plan)?;
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

/// Write TPipelineInstanceParams structure
fn write_pipeline_instance_params<P: TOutputProtocol>(
    protocol: &mut P,
    params: &TPipelineInstanceParams,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPipelineInstanceParams"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: fragment_instance_id (TUniqueId)
    protocol
        .write_field_begin(&TFieldIdentifier::new(
            "fragment_instance_id",
            TType::Struct,
            1,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_unique_id(protocol, &params.fragment_instance_id)?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 3: per_node_scan_ranges (map<i32, list<TScanRangeParams>>)
    protocol
        .write_field_begin(&TFieldIdentifier::new(
            "per_node_scan_ranges",
            TType::Map,
            3,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_map_begin(&TMapIdentifier::new(
            TType::I32,
            TType::List,
            params.per_node_scan_ranges.len() as i32,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for (node_id, scan_ranges) in &params.per_node_scan_ranges {
        protocol
            .write_i32(*node_id)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_list_begin(&TListIdentifier::new(TType::Struct, scan_ranges.len() as i32))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        for scan_range_param in scan_ranges {
            write_scan_range_params(protocol, scan_range_param)?;
        }
        protocol
            .write_list_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }
    protocol
        .write_map_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Optional fields
    if let Some(sender_id) = params.sender_id {
        // Field 4: sender_id (i32)
        protocol
            .write_field_begin(&TFieldIdentifier::new("sender_id", TType::I32, 4))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(sender_id)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    if let Some(backend_num) = params.backend_num {
        // Field 6: backend_num (i32)
        protocol
            .write_field_begin(&TFieldIdentifier::new("backend_num", TType::I32, 6))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(backend_num)
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

/// Write TScanRangeParams structure
fn write_scan_range_params<P: TOutputProtocol>(
    protocol: &mut P,
    params: &TScanRangeParams,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TScanRangeParams"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: scan_range (TScanRange)
    protocol
        .write_field_begin(&TFieldIdentifier::new("scan_range", TType::Struct, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_scan_range(protocol, &params.scan_range)?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: volume_id (optional i32)
    if let Some(volume_id) = params.volume_id {
        protocol
            .write_field_begin(&TFieldIdentifier::new("volume_id", TType::I32, 2))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(volume_id)
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

/// Write TScanRange structure
/// Reference: PlanNodes.thrift TScanRange
fn write_scan_range<P: TOutputProtocol>(
    protocol: &mut P,
    scan_range: &crate::scan_range_builder::TScanRange,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TScanRange"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 4: palo_scan_range (optional TPaloScanRange)
    if let Some(palo_range) = &scan_range.palo_scan_range {
        protocol
            .write_field_begin(&TFieldIdentifier::new("palo_scan_range", TType::Struct, 4))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        write_palo_scan_range(protocol, palo_range)?;
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

/// Write TPaloScanRange structure
/// Reference: PlanNodes.thrift TPaloScanRange
fn write_palo_scan_range<P: TOutputProtocol>(
    protocol: &mut P,
    palo_range: &crate::scan_range_builder::TPaloScanRange,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPaloScanRange"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: hosts (required list<TNetworkAddress>)
    protocol
        .write_field_begin(&TFieldIdentifier::new("hosts", TType::List, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(
            TType::Struct,
            palo_range.hosts.len() as i32,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for host in &palo_range.hosts {
        write_network_address(protocol, host)?;
    }
    protocol
        .write_list_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: schema_hash (required string)
    protocol
        .write_field_begin(&TFieldIdentifier::new("schema_hash", TType::String, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_string(&palo_range.schema_hash)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 3: version (required string)
    protocol
        .write_field_begin(&TFieldIdentifier::new("version", TType::String, 3))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_string(&palo_range.version)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 4: version_hash (required string - deprecated but required)
    protocol
        .write_field_begin(&TFieldIdentifier::new("version_hash", TType::String, 4))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_string(&palo_range.version_hash)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 5: tablet_id (required i64)
    protocol
        .write_field_begin(&TFieldIdentifier::new("tablet_id", TType::I64, 5))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i64(palo_range.tablet_id)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 6: db_name (required string)
    protocol
        .write_field_begin(&TFieldIdentifier::new("db_name", TType::String, 6))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_string(&palo_range.db_name)
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

/// Write TUniqueId structure
fn write_unique_id<P: TOutputProtocol>(protocol: &mut P, id: &TUniqueId) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TUniqueId"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: hi (i64)
    protocol
        .write_field_begin(&TFieldIdentifier::new("hi", TType::I64, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i64(id.hi)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: lo (i64)
    protocol
        .write_field_begin(&TFieldIdentifier::new("lo", TType::I64, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i64(id.lo)
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

/// Write TNetworkAddress structure
fn write_network_address<P: TOutputProtocol>(
    protocol: &mut P,
    addr: &crate::scan_range_builder::TNetworkAddress,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TNetworkAddress"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: hostname (string)
    protocol
        .write_field_begin(&TFieldIdentifier::new("hostname", TType::String, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_string(&addr.hostname)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: port (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("port", TType::I32, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(addr.port)
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
