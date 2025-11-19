//! Conversion between manual fe-planner types and auto-generated doris-thrift types
//!
//! This is a temporary bridge until fe-planner is fully migrated to use auto-generated types.

use doris_thrift;
use doris_thrift::palo_internal_service as pis;
use fe_common::Result;
use std::collections::BTreeMap;
use thrift::protocol::{TCompactOutputProtocol, TSerializable};
use thrift::transport::TBufferChannel;

/// Serialize pipeline params using auto-generated Thrift code
pub fn serialize_with_autogen(
    manual_params: &fe_planner::thrift_plan::TPipelineFragmentParamsList,
    query_id_bytes: [u8; 16],
) -> Result<Vec<u8>> {
    // Convert query_id bytes to TUniqueId
    let query_id = bytes_to_unique_id(query_id_bytes);

    // Convert manual params_list to auto-generated
    let autogen_params_list: Vec<pis::TPipelineFragmentParams> = manual_params
        .params_list
        .iter()
        .map(|p| convert_pipeline_fragment_params(p))
        .collect::<Result<Vec<_>>>()?;

    // Create TPipelineFragmentParamsList using auto-generated types
    let autogen_params = pis::TPipelineFragmentParamsList {
        params_list: Some(autogen_params_list),
        desc_tbl: None, // Moved to individual params
        file_scan_params: None,
        coord: None,
        query_globals: None, // Moved to individual params
        resource_info: None,
        fragment_num_on_host: None,
        query_options: None, // Moved to individual params
        is_nereids: Some(true),
        workload_groups: None,
        query_id: Some(query_id),
        topn_filter_source_node_ids: None,
        runtime_filter_merge_addr: None,
        runtime_filter_info: None,
    };

    // Serialize using auto-generated TSerializable implementation
    let mut transport = TBufferChannel::with_capacity(0, 8192);
    let mut protocol = TCompactOutputProtocol::new(&mut transport);

    autogen_params
        .write_to_out_protocol(&mut protocol)
        .map_err(|e| fe_common::DorisError::InternalError(format!("Thrift serialization failed: {}", e)))?;

    Ok(transport.write_bytes().to_vec())
}

/// Convert bytes to TUniqueId
fn bytes_to_unique_id(bytes: [u8; 16]) -> doris_thrift::types::TUniqueId {
    doris_thrift::types::TUniqueId {
        hi: i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]),
        lo: i64::from_be_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11],
            bytes[12], bytes[13], bytes[14], bytes[15],
        ]),
    }
}

/// Convert TPipelineFragmentParams from manual to auto-generated
fn convert_pipeline_fragment_params(
    manual: &fe_planner::thrift_plan::TPipelineFragmentParams,
) -> Result<pis::TPipelineFragmentParams> {
    // Convert query_id
    let query_id = doris_thrift::types::TUniqueId {
        hi: manual.query_id.hi,
        lo: manual.query_id.lo,
    };

    // Convert per_exch_num_senders (HashMap -> BTreeMap)
    let per_exch_num_senders: BTreeMap<i32, i32> = manual
        .per_exch_num_senders
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();

    // Convert descriptor table if present
    let desc_tbl = manual
        .desc_tbl
        .as_ref()
        .map(convert_descriptor_table)
        .transpose()?;

    // Convert fragment
    let fragment = convert_plan_fragment(&manual.fragment)?;

    // Convert local_params
    let local_params = manual
        .local_params
        .iter()
        .map(convert_pipeline_instance_params)
        .collect::<Result<Vec<_>>>()?;

    // Convert query_globals if present
    let query_globals = manual
        .query_globals
        .as_ref()
        .map(convert_query_globals)
        .transpose()?;

    // Convert query_options if present
    let query_options = manual
        .query_options
        .as_ref()
        .map(convert_query_options)
        .transpose()?;

    Ok(pis::TPipelineFragmentParams {
        protocol_version: pis::PaloInternalServiceVersion::V1,  // Required field
        query_id,  // Required field
        fragment_id: manual.fragment_id,  // Option<i32>
        per_exch_num_senders,  // Required BTreeMap
        desc_tbl,
        resource_info: None,
        destinations: None,
        num_senders: manual.num_senders,  // Already Option<i32>
        send_query_statistics_with_every_batch: Some(false),
        coord: None,
        query_globals,
        query_options,
        import_label: None,
        db_name: None,
        load_job_id: None,
        load_error_hub_info: None,
        fragment_num_on_host: None,
        backend_id: manual.backend_id,  // Already Option<i64>
        need_wait_execution_trigger: None,
        instances_sharing_hash_table: None,
        is_simplified_param: None,
        global_dict: None,
        fragment: Some(fragment),
        local_params: Some(local_params),
        workload_groups: None,
        txn_conf: None,
        table_name: None,
        file_scan_params: None,
        group_commit: None,
        load_stream_per_node: None,
        total_load_streams: None,
        num_local_sink: None,
        num_buckets: None,
        bucket_seq_to_instance_idx: None,
        per_node_shared_scans: None,
        parallel_instances: None,
        total_instances: manual.total_instances,  // Already Option<i32>
        shuffle_idx_to_instance_idx: None,
        is_nereids: manual.is_nereids,  // Already Option<bool>
        wal_id: None,
        content_length: None,
        current_connect_fe: None,
        topn_filter_source_node_ids: None,
        ai_resources: None,
        is_mow_table: None,
    })
}

/// Convert TDescriptorTable
fn convert_descriptor_table(
    manual: &fe_planner::thrift_plan::TDescriptorTable,
) -> Result<doris_thrift::descriptors::TDescriptorTable> {
    use doris_thrift::descriptors::*;

    let slot_descriptors = manual.slot_descriptors.as_ref().map(|descriptors| {
        descriptors
            .iter()
            .map(|sd| TSlotDescriptor {
                id: sd.id,  // Required
                parent: sd.parent,  // Required
                slot_type: convert_type_desc(&sd.slot_type),  // Required
                column_pos: sd.column_pos,  // Required
                byte_offset: sd.byte_offset,  // Required
                null_indicator_byte: sd.null_indicator_byte,  // Required
                null_indicator_bit: sd.null_indicator_bit,  // Required
                col_name: sd.col_name.clone(),  // Required String
                slot_idx: sd.slot_idx,  // Required
                is_materialized: sd.is_materialized,  // Required bool
                col_unique_id: sd.col_unique_id,
                is_key: None,
                need_materialize: None,
                is_auto_increment: None,
                column_paths: None,
                col_default_value: None,
                primitive_type: None,
                virtual_column_expr: None,
            })
            .collect()
    });

    let tuple_descriptors: Vec<TTupleDescriptor> = manual
        .tuple_descriptors
        .iter()
        .map(|td| TTupleDescriptor {
            id: td.id,  // Required i32
            byte_size: td.byte_size,  // Required i32
            num_null_bytes: td.num_null_bytes,  // Required i32
            table_id: td.table_id,
            num_null_slots: None,
        })
        .collect();

    Ok(TDescriptorTable {
        slot_descriptors,
        tuple_descriptors,  // Required Vec, not Option
        table_descriptors: None,
    })
}

/// Convert TTypeDesc
fn convert_type_desc(manual: &fe_planner::thrift_plan::TTypeDesc) -> doris_thrift::types::TTypeDesc {
    use doris_thrift::types::*;

    let types: Vec<TTypeNode> = manual
        .types
        .iter()
        .map(|tn| {
            let node_type_val = match &tn.node_type {
                fe_planner::thrift_plan::TTypeNodeType::Scalar => 0,
                fe_planner::thrift_plan::TTypeNodeType::Array => 1,
                fe_planner::thrift_plan::TTypeNodeType::Map => 2,
                fe_planner::thrift_plan::TTypeNodeType::Struct => 3,
            };
            TTypeNode {
                type_: TTypeNodeType(node_type_val),  // Required
                scalar_type: None,
                struct_fields: None,
                contains_null: None,
                contains_nulls: None,
            }
        })
        .collect();

    TTypeDesc {
        types: Some(types),
        is_nullable: None,
        byte_size: None,
        sub_types: None,
        result_is_nullable: None,
        function_name: None,
        be_exec_version: None,
    }
}

/// Convert TPlanFragment
fn convert_plan_fragment(
    manual: &fe_planner::thrift_plan::TPlanFragment,
) -> Result<doris_thrift::planner::TPlanFragment> {
    use doris_thrift::planner::*;

    let plan = convert_plan(&manual.plan)?;
    let partition = convert_data_partition(&manual.partition)?;

    Ok(TPlanFragment {
        plan: Some(plan),
        output_exprs: None,
        output_sink: None,
        partition,  // Required field
        min_reservation_bytes: None,
        initial_reservation_total_claims: None,
        query_cache_param: None,
    })
}

/// Convert TPlan
fn convert_plan(
    manual: &fe_planner::thrift_plan::TPlan,
) -> Result<doris_thrift::plan_nodes::TPlan> {
    use doris_thrift::plan_nodes::*;

    let nodes: Vec<TPlanNode> = manual
        .nodes
        .iter()
        .map(convert_plan_node)
        .collect::<Result<Vec<_>>>()?;

    Ok(TPlan { nodes })  // Required Vec, not Option
}

/// Convert TPlanNode
fn convert_plan_node(
    manual: &fe_planner::thrift_plan::TPlanNode,
) -> Result<doris_thrift::plan_nodes::TPlanNode> {
    use doris_thrift::plan_nodes::*;

    let node_type_val = match manual.node_type {
        fe_planner::thrift_plan::TPlanNodeType::OlapScanNode => 0,
        fe_planner::thrift_plan::TPlanNodeType::HashJoinNode => 4,
        fe_planner::thrift_plan::TPlanNodeType::AggregationNode => 6,
        fe_planner::thrift_plan::TPlanNodeType::ExchangeNode => 10,
        _ => 0,  // Default to OlapScanNode for other types
    };

    let olap_scan_node = manual.olap_scan_node.as_ref()
        .map(convert_olap_scan_node)
        .transpose()?;

    Ok(TPlanNode {
        node_id: manual.node_id,  // Required
        node_type: TPlanNodeType(node_type_val),  // Required
        num_children: manual.num_children,  // Required
        limit: manual.limit,  // Required
        row_tuples: manual.row_tuples.clone(),  // Required Vec
        nullable_tuples: manual.nullable_tuples.clone(),  // Required Vec
        conjuncts: None,
        compact_data: manual.compact_data,  // Required bool
        hash_join_node: None,
        agg_node: None,
        sort_node: None,
        merge_node: None,
        exchange_node: None,
        mysql_scan_node: None,
        olap_scan_node,
        csv_scan_node: None,
        broker_scan_node: None,
        pre_agg_node: None,
        schema_scan_node: None,
        merge_join_node: None,
        meta_scan_node: None,
        analytic_node: None,
        olap_rewrite_node: None,
        union_node: None,
        resource_profile: None,
        es_scan_node: None,
        repeat_node: None,
        assert_num_rows_node: None,
        intersect_node: None,
        except_node: None,
        odbc_scan_node: None,
        runtime_filters: None,
        group_commit_scan_node: None,
        materialization_node: None,
        vconjunct: None,
        table_function_node: None,
        output_slot_ids: None,
        data_gen_scan_node: None,
        file_scan_node: None,
        jdbc_scan_node: None,
        nested_loop_join_node: None,
        test_external_scan_node: None,
        push_down_agg_type_opt: None,
        push_down_count: None,
        distribute_expr_lists: None,
        is_serial_operator: None,
        projections: None,
        output_tuple_id: None,
        partition_sort_node: None,
        intermediate_projections_list: None,
        intermediate_output_tuple_id_list: None,
        topn_filter_source_node_ids: None,
        nereids_id: None,
    })
}

/// Convert TOlapScanNode
fn convert_olap_scan_node(
    manual: &fe_planner::thrift_plan::TOlapScanNode,
) -> Result<doris_thrift::plan_nodes::TOlapScanNode> {
    use doris_thrift::plan_nodes::*;

    Ok(TOlapScanNode {
        tuple_id: manual.tuple_id,  // Required
        key_column_name: manual.key_column_name.clone(),  // Required Vec<String>
        key_column_type: vec![],  // Required Vec - manual type is Vec<TPrimitiveType>, convert later if needed
        is_preaggregation: manual.is_preaggregation,  // Required bool
        sort_column: None,
        key_type: None,
        table_name: manual.table_name.clone(),
        columns_desc: None,
        sort_info: None,
        sort_limit: None,
        enable_unique_key_merge_on_write: None,
        push_down_agg_type_opt: None,
        use_topn_opt: None,
        indexes_desc: None,
        output_column_unique_ids: None,
        distribute_column_ids: None,
        schema_version: None,
        topn_filter_source_node_ids: None,
        score_sort_info: None,
        ann_sort_info: None,
        ann_sort_limit: None,
        score_sort_limit: None,
    })
}

/// Convert TDataPartition
fn convert_data_partition(
    manual: &fe_planner::thrift_plan::TDataPartition,
) -> Result<doris_thrift::partitions::TDataPartition> {
    use doris_thrift::partitions::*;

    Ok(TDataPartition {
        type_: TPartitionType(manual.partition_type as i32),
        partition_exprs: None,
        partition_infos: None,
    })
}

/// Convert TPipelineInstanceParams
fn convert_pipeline_instance_params(
    manual: &fe_planner::thrift_plan::TPipelineInstanceParams,
) -> Result<pis::TPipelineInstanceParams> {
    // Convert fragment_instance_id
    let fragment_instance_id = doris_thrift::types::TUniqueId {
        hi: manual.fragment_instance_id.hi,
        lo: manual.fragment_instance_id.lo,
    };

    // Convert per_node_scan_ranges (HashMap -> BTreeMap)
    let per_node_scan_ranges: BTreeMap<i32, Vec<pis::TScanRangeParams>> = manual
        .per_node_scan_ranges
        .iter()
        .map(|(node_id, ranges)| {
            let converted_ranges = ranges
                .iter()
                .map(convert_scan_range_params)
                .collect::<Result<Vec<_>>>()?;
            Ok((*node_id, converted_ranges))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;

    Ok(pis::TPipelineInstanceParams {
        fragment_instance_id,
        per_node_scan_ranges,
        sender_id: manual.sender_id,  // Already Option<i32>
        backend_num: manual.backend_num,  // Already Option<i32>
        build_hash_table_for_broadcast_join: None,
        runtime_filter_params: None,
        per_node_shared_scans: None,
        topn_filter_source_node_ids: None,
        topn_filter_descs: None,
    })
}

/// Convert TScanRangeParams
fn convert_scan_range_params(
    manual: &fe_planner::thrift_plan::TScanRangeParams,
) -> Result<pis::TScanRangeParams> {
    let scan_range = convert_scan_range(&manual.scan_range)?;

    Ok(pis::TScanRangeParams {
        scan_range,  // Already Result<TScanRange>
        volume_id: manual.volume_id,
    })
}

/// Convert TScanRange
fn convert_scan_range(
    manual: &fe_planner::scan_range_builder::TScanRange,
) -> Result<doris_thrift::plan_nodes::TScanRange> {
    let palo_scan_range = manual
        .palo_scan_range
        .as_ref()
        .map(convert_palo_scan_range)
        .transpose()?;

    Ok(doris_thrift::plan_nodes::TScanRange {
        palo_scan_range,
        kudu_scan_token: None,
        broker_scan_range: None,
        es_scan_range: None,
        ext_scan_range: None,
        data_gen_scan_range: None,
        meta_scan_range: None,
    })
}

/// Convert TPaloScanRange
fn convert_palo_scan_range(
    manual: &fe_planner::scan_range_builder::TPaloScanRange,
) -> Result<doris_thrift::plan_nodes::TPaloScanRange> {
    let hosts: Vec<doris_thrift::types::TNetworkAddress> = manual
        .hosts
        .iter()
        .map(|h| doris_thrift::types::TNetworkAddress {
            hostname: h.hostname.clone(),
            port: h.port,
        })
        .collect();

    Ok(doris_thrift::plan_nodes::TPaloScanRange {
        hosts,  // Required Vec
        schema_hash: manual.schema_hash.clone(),  // Required String
        version: manual.version.clone(),  // Required String
        version_hash: manual.version_hash.clone(),  // Required String
        tablet_id: manual.tablet_id,  // Required
        db_name: manual.db_name.clone(),  // Required String
        partition_column_ranges: None,
        index_name: None,
        table_name: None,
    })
}

/// Convert TQueryGlobals
fn convert_query_globals(
    manual: &fe_planner::thrift_plan::TQueryGlobals,
) -> Result<pis::TQueryGlobals> {
    Ok(pis::TQueryGlobals {
        now_string: manual.now_string.clone(),
        timestamp_ms: manual.timestamp_ms,
        time_zone: manual.time_zone.clone(),
        load_zero_tolerance: None,
        nano_seconds: None,
        lc_time_names: None,
    })
}

/// Convert TQueryOptions - minimal version with only essential fields
fn convert_query_options(
    manual: &fe_planner::thrift_plan::TQueryOptions,
) -> Result<pis::TQueryOptions> {
    Ok(pis::TQueryOptions {
        // Essential fields
        batch_size: manual.batch_size,
        mem_limit: manual.mem_limit,
        query_timeout: manual.query_timeout,
        is_report_success: manual.is_report_success,
        num_scanner_threads: manual.num_scanner_threads,
        max_scan_key_num: manual.max_scan_key_num,
        max_pushdown_conditions_per_column: manual.max_pushdown_conditions_per_column,
        parallel_instance: manual.parallel_instance,
        be_exec_version: manual.be_exec_version,
        enable_profile: manual.enable_profile,

        // All other fields set to None for simplicity
        abort_on_error: None,
        max_errors: None,
        disable_codegen: None,
        num_nodes: None,
        max_scan_range_length: None,
        max_io_buffers: None,
        allow_unsupported_formats: None,
        default_order_by_limit: None,
        abort_on_default_limit_exceeded: None,
        codegen_level: None,
        kudu_latest_observed_ts: None,
        query_type: None,
        min_reservation: None,
        max_reservation: None,
        initial_reservation_total_claims: None,
        buffer_pool_limit: None,
        default_spillable_buffer_size: None,
        min_spillable_buffer_size: None,
        max_row_size: None,
        disable_stream_preaggregations: None,
        mt_dop: None,
        load_mem_limit: None,
        enable_spilling: None,
        enable_enable_exchange_node_parallel_merge: None,
        runtime_filter_wait_time_ms: None,
        runtime_filter_max_in_num: None,
        resource_limit: None,
        return_object_data_as_binary: None,
        trim_tailing_spaces_for_external_table_query: None,
        enable_function_pushdown: None,
        fragment_transmission_compression_codec: None,
        enable_local_exchange: None,
        skip_storage_engine_merge: None,
        skip_delete_predicate: None,
        enable_new_shuffle_hash_method: None,
        partitioned_hash_join_rows_threshold: None,
        enable_share_hash_table_for_broadcast_join: None,
        check_overflow_for_decimal: None,
        skip_delete_bitmap: None,
        enable_pipeline_engine: None,
        repeat_max_num: None,
        external_sort_bytes_threshold: None,
        partitioned_hash_agg_rows_threshold: None,
        enable_file_cache: None,
        insert_timeout: None,
        execution_timeout: None,
        dry_run_query: None,
        enable_common_expr_pushdown: None,
        mysql_row_binary_format: None,
        external_agg_bytes_threshold: None,
        external_agg_partition_bits: None,
        file_cache_base_path: None,
        enable_parquet_lazy_mat: None,
        enable_orc_lazy_mat: None,
        scan_queue_mem_limit: None,
        enable_scan_node_run_serial: None,
        enable_insert_strict: None,
        enable_inverted_index_query: None,
        truncate_char_or_varchar_columns: None,
        enable_hash_join_early_start_probe: None,
        enable_pipeline_x_engine: None,
        enable_memtable_on_sink_node: None,
        enable_delete_sub_predicate_v2: None,
        fe_process_uuid: None,
        inverted_index_conjunction_opt_threshold: None,
        enable_page_cache: None,
        analyze_timeout: None,
        faster_float_convert: None,
        enable_decimal256: None,
        enable_local_shuffle: None,
        skip_missing_version: None,
        runtime_filter_wait_infinitely: None,
        condition_cache_digest: None,
        inverted_index_max_expansions: None,
        inverted_index_skip_threshold: None,
        enable_parallel_scan: None,
        parallel_scan_max_scanners_count: None,
        parallel_scan_min_rows_per_scanner: None,
        skip_bad_tablet: None,
        scanner_scale_up_ratio: None,
        enable_distinct_streaming_aggregation: None,
        enable_join_spill: None,
        enable_sort_spill: None,
        enable_agg_spill: None,
        min_revocable_mem: None,
        spill_streaming_agg_mem_limit: None,
        data_queue_max_blocks: None,
        enable_common_expr_pushdown_for_inverted_index: None,
        local_exchange_free_blocks_limit: None,
        enable_force_spill: None,
        enable_parquet_filter_by_min_max: None,
        enable_orc_filter_by_min_max: None,
        max_column_reader_num: None,
        enable_local_merge_sort: None,
        enable_parallel_result_sink: None,
        enable_short_circuit_query_access_column_store: None,
        enable_no_need_read_data_opt: None,
        read_csv_empty_line_as_null: None,
        serde_dialect: None,
        enable_match_without_inverted_index: None,
        enable_fallback_on_missing_inverted_index: None,
        keep_carriage_return: None,
        runtime_bloom_filter_min_size: None,
        hive_parquet_use_column_names: None,
        hive_orc_use_column_names: None,
        enable_segment_cache: None,
        runtime_bloom_filter_max_size: None,
        in_list_value_count_threshold: None,
        enable_verbose_profile: None,
        rpc_verbose_profile_max_instance_count: None,
        enable_adaptive_pipeline_task_serial_read_on_limit: None,
        adaptive_pipeline_task_serial_read_on_limit: None,
        parallel_prepare_threshold: None,
        partition_topn_max_partitions: None,
        partition_topn_pre_partition_rows: None,
        enable_parallel_outfile: None,
        enable_phrase_query_sequential_opt: None,
        enable_auto_create_when_overwrite: None,
        orc_tiny_stripe_threshold_bytes: None,
        orc_once_max_read_bytes: None,
        orc_max_merge_distance_bytes: None,
        ignore_runtime_filter_error: None,
        enable_fixed_len_to_uint32_v2: None,
        enable_shared_exchange_sink_buffer: None,
        enable_inverted_index_searcher_cache: None,
        enable_inverted_index_query_cache: None,
        enable_condition_cache: None,
        profile_level: None,
        min_scanner_concurrency: None,
        min_scan_scheduler_concurrency: None,
        enable_runtime_filter_partition_prune: None,
        minimum_operator_memory_required_kb: None,
        enable_mem_overcommit: None,
        query_slot_count: None,
        enable_spill: None,
        enable_reserve_memory: None,
        revocable_memory_high_watermark_percent: None,
        spill_sort_mem_limit: None,
        spill_sort_batch_bytes: None,
        spill_aggregation_partition_count: None,
        spill_hash_join_partition_count: None,
        low_memory_mode_buffer_limit: None,
        dump_heap_profile_when_mem_limit_exceeded: None,
        inverted_index_compatible_read: None,
        check_orc_init_sargs_success: None,
        exchange_multi_blocks_byte_size: None,
        enable_strict_cast: None,
        new_version_unix_timestamp: None,
        hnsw_ef_search: None,
        hnsw_check_relative_distance: None,
        hnsw_bounded_queue: None,
        optimize_index_scan_parallelism: None,
        enable_prefer_cached_rowset: None,
        query_freshness_tolerance_ms: None,
        merge_read_slice_size: None,
        enable_fuzzy_blockable_task: None,
        shuffled_agg_ids: None,
        enable_extended_regex: None,
        iceberg_write_target_file_size_bytes: None,
        disable_file_cache: None,
    })
}
