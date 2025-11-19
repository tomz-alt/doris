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

    // Field 3: fragment_id (optional i32)
    // Reference: Java FE ThriftPlansBuilder.java:340 params.setFragmentId(fragment.getFragmentId().asInt())
    if let Some(fragment_id) = params.fragment_id {
        protocol
            .write_field_begin(&TFieldIdentifier::new("fragment_id", TType::I32, 3))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(fragment_id)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

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

    // Field 5: desc_tbl (optional TDescriptorTable)
    // Reference: Java FE Coordinator.java:3214 params.setDescTbl(descTable)
    if let Some(ref desc_tbl) = params.desc_tbl {
        protocol
            .write_field_begin(&TFieldIdentifier::new("desc_tbl", TType::Struct, 5))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        write_descriptor_table(protocol, desc_tbl)?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 7: destinations (list<TPlanFragmentDestination>) - REQUIRED!
    // Reference: Java FE ThriftPlansBuilder.java:349-357 params.setDestinations(nonMultiCastDestinations)
    protocol
        .write_field_begin(&TFieldIdentifier::new("destinations", TType::List, 7))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(
            TType::Struct,
            params.destinations.len() as i32,
        ))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for dest in &params.destinations {
        write_plan_fragment_destination(protocol, dest)?;
    }
    protocol
        .write_list_end()
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

    // Field 11: query_globals (optional TQueryGlobals)
    // Reference: Java FE Coordinator.java:3221 params.setQueryGlobals(queryGlobals)
    if let Some(ref query_globals) = params.query_globals {
        protocol
            .write_field_begin(&TFieldIdentifier::new("query_globals", TType::Struct, 11))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        write_query_globals(protocol, query_globals)?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 12: query_options (optional TQueryOptions)
    // Reference: Java FE Coordinator.java:3222 params.setQueryOptions(queryOptions)
    if let Some(ref query_options) = params.query_options {
        protocol
            .write_field_begin(&TFieldIdentifier::new("query_options", TType::Struct, 12))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        write_query_options(protocol, query_options)?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 17: fragment_num_on_host (optional i32)
    // Reference: Java FE ThriftPlansBuilder.java:343 params.setFragmentNumOnHost(workerProcessInstanceNum.count(worker))
    if let Some(fragment_num_on_host) = params.fragment_num_on_host {
        protocol
            .write_field_begin(&TFieldIdentifier::new("fragment_num_on_host", TType::I32, 17))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(fragment_num_on_host)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 18: backend_id (optional i64)
    // Reference: Java FE ThriftPlansBuilder.java:336 params.setBackendId(worker.id())
    if let Some(backend_id) = params.backend_id {
        protocol
            .write_field_begin(&TFieldIdentifier::new("backend_id", TType::I64, 18))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i64(backend_id)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
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

    // Field 38: total_instances (optional i32)
    // Reference: Java FE ThriftPlansBuilder.java:360 params.setTotalInstances(instanceNumInThisFragment)
    if let Some(total_instances) = params.total_instances {
        protocol
            .write_field_begin(&TFieldIdentifier::new("total_instances", TType::I32, 38))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(total_instances)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 40: is_nereids (optional bool)
    // Reference: Java FE ThriftPlansBuilder.java:335 params.setIsNereids(true)
    if let Some(is_nereids) = params.is_nereids {
        protocol
            .write_field_begin(&TFieldIdentifier::new("is_nereids", TType::Bool, 40))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_bool(is_nereids)
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

/// Write TPlanFragment structure (reuse for pipeline params)
/// Reference: Planner.thrift TPlanFragment
fn write_plan_fragment<P: TOutputProtocol>(
    protocol: &mut P,
    fragment: &TPlanFragment,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPlanFragment"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: plan (TPlan) - optional
    protocol
        .write_field_begin(&TFieldIdentifier::new("plan", TType::Struct, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_plan(protocol, &fragment.plan)?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 6: partition (TDataPartition) - REQUIRED!
    protocol
        .write_field_begin(&TFieldIdentifier::new("partition", TType::Struct, 6))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_data_partition(protocol, &fragment.partition)?;
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

/// Write TDataPartition structure
/// Reference: Partitions.thrift TDataPartition
fn write_data_partition<P: TOutputProtocol>(
    protocol: &mut P,
    partition: &TDataPartition,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TDataPartition"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: type (TPartitionType) - REQUIRED
    protocol
        .write_field_begin(&TFieldIdentifier::new("type", TType::I32, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(partition.partition_type as i32)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: partition_exprs (optional list<TExpr>) - not implemented yet
    // Field 3: partition_infos (optional list<TRangePartition>) - not implemented yet

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

// ============================================================================
// Descriptor Table Serialization
// Reference: Descriptors.thrift
// ============================================================================

/// Write TDescriptorTable structure
/// Field IDs from Descriptors.thrift
pub fn write_descriptor_table<P: TOutputProtocol>(
    protocol: &mut P,
    desc_tbl: &crate::thrift_plan::TDescriptorTable,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TDescriptorTable"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: slotDescriptors (optional list<TSlotDescriptor>)
    if let Some(ref slot_descriptors) = desc_tbl.slot_descriptors {
        protocol
            .write_field_begin(&TFieldIdentifier::new("slotDescriptors", TType::List, 1))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_list_begin(&TListIdentifier::new(TType::Struct, slot_descriptors.len() as i32))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        for slot_desc in slot_descriptors {
            write_slot_descriptor(protocol, slot_desc)?;
        }
        protocol
            .write_list_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 2: tupleDescriptors (required list<TTupleDescriptor>)
    protocol
        .write_field_begin(&TFieldIdentifier::new("tupleDescriptors", TType::List, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(TType::Struct, desc_tbl.tuple_descriptors.len() as i32))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for tuple_desc in &desc_tbl.tuple_descriptors {
        write_tuple_descriptor(protocol, tuple_desc)?;
    }
    protocol
        .write_list_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 3: tableDescriptors (optional list<TTableDescriptor>) - skip for now

    protocol
        .write_field_stop()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_struct_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    Ok(())
}

/// Write TSlotDescriptor structure
/// Field IDs from Descriptors.thrift (fields 1-10 required, 11-13 optional)
fn write_slot_descriptor<P: TOutputProtocol>(
    protocol: &mut P,
    slot: &crate::thrift_plan::TSlotDescriptor,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TSlotDescriptor"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: id (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("id", TType::I32, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(slot.id)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: parent (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("parent", TType::I32, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(slot.parent)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 3: slotType (TTypeDesc)
    protocol
        .write_field_begin(&TFieldIdentifier::new("slotType", TType::Struct, 3))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_type_desc(protocol, &slot.slot_type)?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 4: columnPos (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("columnPos", TType::I32, 4))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(slot.column_pos)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 5: byteOffset (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("byteOffset", TType::I32, 5))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(slot.byte_offset)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 6: nullIndicatorByte (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("nullIndicatorByte", TType::I32, 6))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(slot.null_indicator_byte)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 7: nullIndicatorBit (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("nullIndicatorBit", TType::I32, 7))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(slot.null_indicator_bit)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 8: colName (string)
    protocol
        .write_field_begin(&TFieldIdentifier::new("colName", TType::String, 8))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_string(&slot.col_name)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 9: slotIdx (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("slotIdx", TType::I32, 9))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(slot.slot_idx)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 10: isMaterialized (bool)
    protocol
        .write_field_begin(&TFieldIdentifier::new("isMaterialized", TType::Bool, 10))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_bool(slot.is_materialized)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 11: col_unique_id (optional i32)
    if let Some(col_unique_id) = slot.col_unique_id {
        protocol
            .write_field_begin(&TFieldIdentifier::new("col_unique_id", TType::I32, 11))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(col_unique_id)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 12: is_key (optional bool)
    if let Some(is_key) = slot.is_key {
        protocol
            .write_field_begin(&TFieldIdentifier::new("is_key", TType::Bool, 12))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_bool(is_key)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 13: need_materialize (optional bool)
    if let Some(need_materialize) = slot.need_materialize {
        protocol
            .write_field_begin(&TFieldIdentifier::new("need_materialize", TType::Bool, 13))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_bool(need_materialize)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 14: is_auto_increment (optional bool)
    if let Some(is_auto_increment) = slot.is_auto_increment {
        protocol
            .write_field_begin(&TFieldIdentifier::new("is_auto_increment", TType::Bool, 14))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_bool(is_auto_increment)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 15: column_paths (optional list<string>)
    if let Some(ref column_paths) = slot.column_paths {
        protocol
            .write_field_begin(&TFieldIdentifier::new("column_paths", TType::List, 15))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_list_begin(&TListIdentifier::new(TType::String, column_paths.len() as i32))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        for path in column_paths {
            protocol
                .write_string(path)
                .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        }
        protocol
            .write_list_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 16: col_default_value (optional string)
    if let Some(ref col_default_value) = slot.col_default_value {
        protocol
            .write_field_begin(&TFieldIdentifier::new("col_default_value", TType::String, 16))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_string(col_default_value)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 17: primitive_type (optional TPrimitiveType)
    if let Some(primitive_type) = slot.primitive_type {
        protocol
            .write_field_begin(&TFieldIdentifier::new("primitive_type", TType::I32, 17))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(primitive_type as i32)
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

/// Write TTupleDescriptor structure
/// Field IDs from Descriptors.thrift
fn write_tuple_descriptor<P: TOutputProtocol>(
    protocol: &mut P,
    tuple: &crate::thrift_plan::TTupleDescriptor,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TTupleDescriptor"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: id (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("id", TType::I32, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(tuple.id)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: byteSize (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("byteSize", TType::I32, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(tuple.byte_size)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 3: numNullBytes (i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("numNullBytes", TType::I32, 3))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_i32(tuple.num_null_bytes)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 4: tableId (optional i64)
    if let Some(table_id) = tuple.table_id {
        protocol
            .write_field_begin(&TFieldIdentifier::new("tableId", TType::I64, 4))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i64(table_id)
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

/// Write TTypeDesc structure
/// Reference: Types.thrift TTypeDesc
fn write_type_desc<P: TOutputProtocol>(
    protocol: &mut P,
    type_desc: &crate::thrift_plan::TTypeDesc,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TTypeDesc"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: types (list<TTypeNode>)
    protocol
        .write_field_begin(&TFieldIdentifier::new("types", TType::List, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_list_begin(&TListIdentifier::new(TType::Struct, type_desc.types.len() as i32))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    for type_node in &type_desc.types {
        write_type_node(protocol, type_node)?;
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

/// Write TTypeNode structure
/// Reference: Types.thrift TTypeNode
fn write_type_node<P: TOutputProtocol>(
    protocol: &mut P,
    type_node: &crate::thrift_plan::TTypeNode,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TTypeNode"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: type (TTypeNodeType enum as i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("type", TType::I32, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    let type_value = match type_node.node_type {
        crate::thrift_plan::TTypeNodeType::Scalar => 0,
        crate::thrift_plan::TTypeNodeType::Array => 1,
        crate::thrift_plan::TTypeNodeType::Map => 2,
        crate::thrift_plan::TTypeNodeType::Struct => 3,
    };
    protocol
        .write_i32(type_value)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: scalar_type (optional TScalarType)
    if let Some(ref scalar_type) = type_node.scalar_type {
        protocol
            .write_field_begin(&TFieldIdentifier::new("scalar_type", TType::Struct, 2))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        write_scalar_type(protocol, scalar_type)?;
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

/// Write TScalarType structure
/// Reference: Types.thrift TScalarType (field 1 only, simplified)
fn write_scalar_type<P: TOutputProtocol>(
    protocol: &mut P,
    scalar_type: &crate::thrift_plan::TScalarType,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TScalarType"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: type (TPrimitiveType enum as i32)
    protocol
        .write_field_begin(&TFieldIdentifier::new("type", TType::I32, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    let prim_value = match scalar_type.scalar_type {
        crate::thrift_plan::TPrimitiveType::InvalidType => 0,
        crate::thrift_plan::TPrimitiveType::NullType => 1,
        crate::thrift_plan::TPrimitiveType::Boolean => 2,
        crate::thrift_plan::TPrimitiveType::TinyInt => 3,
        crate::thrift_plan::TPrimitiveType::SmallInt => 4,
        crate::thrift_plan::TPrimitiveType::Int => 5,
        crate::thrift_plan::TPrimitiveType::BigInt => 6,
        crate::thrift_plan::TPrimitiveType::Float => 7,
        crate::thrift_plan::TPrimitiveType::Double => 8,
        crate::thrift_plan::TPrimitiveType::Date => 9,
        crate::thrift_plan::TPrimitiveType::DateTime => 10,
        crate::thrift_plan::TPrimitiveType::Binary => 11,
        crate::thrift_plan::TPrimitiveType::DecimalDeprecated => 12,
        crate::thrift_plan::TPrimitiveType::Char => 13,
        crate::thrift_plan::TPrimitiveType::LargeInt => 14,
        crate::thrift_plan::TPrimitiveType::Varchar => 15,
        crate::thrift_plan::TPrimitiveType::Hll => 16,
        crate::thrift_plan::TPrimitiveType::DecimalV2 => 17,
        crate::thrift_plan::TPrimitiveType::Bitmap => 19,
        crate::thrift_plan::TPrimitiveType::Array => 20,
        crate::thrift_plan::TPrimitiveType::Map => 21,
        crate::thrift_plan::TPrimitiveType::Struct => 22,
        crate::thrift_plan::TPrimitiveType::String => 23,
        crate::thrift_plan::TPrimitiveType::All => 24,
        crate::thrift_plan::TPrimitiveType::QuantileState => 25,
        crate::thrift_plan::TPrimitiveType::DateV2 => 26,
        crate::thrift_plan::TPrimitiveType::DateTimeV2 => 27,
        crate::thrift_plan::TPrimitiveType::TimeV2 => 28,
        crate::thrift_plan::TPrimitiveType::Decimal32 => 29,
        crate::thrift_plan::TPrimitiveType::Decimal64 => 30,
        crate::thrift_plan::TPrimitiveType::Decimal128I => 31,
        crate::thrift_plan::TPrimitiveType::Decimal256 => 32,
        crate::thrift_plan::TPrimitiveType::JsonB => 33,
        crate::thrift_plan::TPrimitiveType::Variant => 34,
        crate::thrift_plan::TPrimitiveType::AggState => 35,
        crate::thrift_plan::TPrimitiveType::LambdaFunction => 36,
        crate::thrift_plan::TPrimitiveType::Ipv4 => 37,
        crate::thrift_plan::TPrimitiveType::Ipv6 => 38,
    };
    protocol
        .write_i32(prim_value)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: len (optional i32)
    if let Some(len) = scalar_type.len {
        protocol
            .write_field_begin(&TFieldIdentifier::new("len", TType::I32, 2))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(len)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 3: precision (optional i32)
    if let Some(precision) = scalar_type.precision {
        protocol
            .write_field_begin(&TFieldIdentifier::new("precision", TType::I32, 3))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(precision)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 4: scale (optional i32)
    if let Some(scale) = scalar_type.scale {
        protocol
            .write_field_begin(&TFieldIdentifier::new("scale", TType::I32, 4))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(scale)
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

/// Write TQueryGlobals structure
/// Field IDs from PaloInternalService.thrift
fn write_query_globals<P: TOutputProtocol>(
    protocol: &mut P,
    query_globals: &crate::thrift_plan::TQueryGlobals,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TQueryGlobals"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: now_string (required string)
    protocol
        .write_field_begin(&TFieldIdentifier::new("now_string", TType::String, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_string(&query_globals.now_string)
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: timestamp_ms (optional i64)
    if let Some(timestamp_ms) = query_globals.timestamp_ms {
        protocol
            .write_field_begin(&TFieldIdentifier::new("timestamp_ms", TType::I64, 2))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i64(timestamp_ms)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 3: time_zone (optional string)
    if let Some(ref time_zone) = query_globals.time_zone {
        protocol
            .write_field_begin(&TFieldIdentifier::new("time_zone", TType::String, 3))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_string(time_zone)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 4: load_zero_tolerance (optional bool)
    if let Some(load_zero_tolerance) = query_globals.load_zero_tolerance {
        protocol
            .write_field_begin(&TFieldIdentifier::new("load_zero_tolerance", TType::Bool, 4))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_bool(load_zero_tolerance)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 5: nano_seconds (optional i32)
    if let Some(nano_seconds) = query_globals.nano_seconds {
        protocol
            .write_field_begin(&TFieldIdentifier::new("nano_seconds", TType::I32, 5))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(nano_seconds)
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

/// Write TQueryOptions structure (minimal version)
/// Field IDs from PaloInternalService.thrift
fn write_query_options<P: TOutputProtocol>(
    protocol: &mut P,
    query_options: &crate::thrift_plan::TQueryOptions,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TQueryOptions"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 4: batch_size (optional i32)
    if let Some(batch_size) = query_options.batch_size {
        protocol
            .write_field_begin(&TFieldIdentifier::new("batch_size", TType::I32, 4))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(batch_size)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 12: mem_limit (optional i64)
    if let Some(mem_limit) = query_options.mem_limit {
        protocol
            .write_field_begin(&TFieldIdentifier::new("mem_limit", TType::I64, 12))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i64(mem_limit)
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_field_end()
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    }

    // Field 14: query_timeout (optional i32)
    if let Some(query_timeout) = query_options.query_timeout {
        protocol
            .write_field_begin(&TFieldIdentifier::new("query_timeout", TType::I32, 14))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        protocol
            .write_i32(query_timeout)
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

/// Write TPlanFragmentDestination structure
/// Reference: DataSinks.thrift TPlanFragmentDestination
fn write_plan_fragment_destination<P: TOutputProtocol>(
    protocol: &mut P,
    dest: &crate::thrift_plan::TPlanFragmentDestination,
) -> Result<()> {
    protocol
        .write_struct_begin(&TStructIdentifier::new("TPlanFragmentDestination"))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 1: fragment_instance_id (TUniqueId)
    protocol
        .write_field_begin(&TFieldIdentifier::new("fragment_instance_id", TType::Struct, 1))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_unique_id(protocol, &dest.fragment_instance_id)?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 2: server (TNetworkAddress)
    protocol
        .write_field_begin(&TFieldIdentifier::new("server", TType::Struct, 2))
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
    write_network_address(protocol, &dest.server)?;
    protocol
        .write_field_end()
        .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;

    // Field 3: brpc_server (optional TNetworkAddress)
    if let Some(ref brpc_server) = dest.brpc_server {
        protocol
            .write_field_begin(&TFieldIdentifier::new("brpc_server", TType::Struct, 3))
            .map_err(|e| DorisError::InternalError(format!("Thrift serialize error: {}", e)))?;
        write_network_address(protocol, brpc_server)?;
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
            partition: TDataPartition {
                partition_type: TPartitionType::Unpartitioned,
                partition_exprs: None,
                partition_infos: None,
            },
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
            partition: TDataPartition {
                partition_type: TPartitionType::Unpartitioned,
                partition_exprs: None,
                partition_infos: None,
            },
        };

        let bytes = serialize_plan_fragment(&fragment).unwrap();
        assert!(!bytes.is_empty());
        println!("Serialized plan with scan node: {} bytes", bytes.len());
    }
}
