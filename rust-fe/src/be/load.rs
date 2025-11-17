#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use uuid::Uuid;

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use crate::be::doris::{
    PBlock,
    PColumnMeta,
    PTabletWriterOpenRequest,
    PTabletWriterAddBlockRequest,
    PTabletWithPartition,
    POlapTableSchemaParam,
    POlapTableIndexSchema,
    PSlotDescriptor,
    PTupleDescriptor,
    PTypeDesc,
    PTypeNode,
    PScalarType,
    PUniqueId,
    p_generic_type::TypeId as PTypeId,
    p_column_meta,
};
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use crate::metadata::{catalog, types::DataType as MetaDataType};
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use crate::query::ParsedInsert;
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use crate::error::{Result, DorisError};

/// Helpers for building Doris tablet-writer load requests from FE
/// metadata. This keeps low-level proto details behind a narrow
/// interface so that `execute_sql` / `execute_dml` can remain focused
/// on SQL semantics (AGENTS.md #1, #3).
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
pub struct BeLoadHelper;

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
impl BeLoadHelper {
    /// Build a minimal `PTabletWriterOpenRequest` for a single index
    /// and a set of tablets. This does not yet populate the full
    /// `POlapTableSchemaParam`; the schema integration will be added
    /// once BE-backed metadata is wired into the Rust FE catalog.
    pub fn build_open_request(
        load_id: Uuid,
        index_id: i64,
        txn_id: i64,
        tablet_ids: &[i64],
    ) -> PTabletWriterOpenRequest {
        let id = Self::encode_punique_id(load_id);

        let tablets: Vec<PTabletWithPartition> = tablet_ids
            .iter()
            .map(|tid| PTabletWithPartition {
                partition_id: 0, // placeholder; will come from BE metadata
                tablet_id: *tid,
            })
            .collect();

        PTabletWriterOpenRequest {
            id,  // PUniqueId is not optional
            index_id,
            txn_id,
            schema: POlapTableSchemaParam::default(),  // Not optional
            tablets,
            num_senders: 1,
            need_gen_rollup: false,
            load_mem_limit: None,
            load_channel_timeout_s: None,
            is_high_priority: None,
            sender_ip: None,
            is_vectorized: Some(true),
            backend_id: None,
            enable_profile: Some(false),
            is_incremental: Some(false),
            txn_expiration: None,
            write_file_cache: None,
            storage_vault_id: None,
            sender_id: None,
            workload_group_id: None,
        }
    }

    /// Build an empty `PTabletWriterAddBlockRequest` envelope for a
    /// given load/index/tablet set. The caller is expected to attach a
    /// real `PBlock` payload in a future increment; for now this
    /// helper is used to exercise the control path and validate
    /// request shapes against BE.
    pub fn build_empty_add_block_request(
        load_id: Uuid,
        index_id: i64,
        tablet_ids: &[i64],
        eos: bool,
    ) -> PTabletWriterAddBlockRequest {
        let id = Self::encode_punique_id(load_id);

        PTabletWriterAddBlockRequest {
            id,  // PUniqueId is not optional
            index_id,
            sender_id: 0,
            eos: Some(eos),  // Actually IS optional in real proto
            packet_seq: Some(0),  // Actually IS optional in real proto
            tablet_ids: tablet_ids.to_vec(),
            block: None,
            partition_ids: Vec::new(),
            backend_id: None,
            transfer_by_attachment: Some(false),
            is_high_priority: Some(false),
            write_single_replica: Some(false),
            slave_tablet_nodes: std::collections::HashMap::new(),
            is_single_tablet_block: Some(tablet_ids.len() == 1),
            hang_wait: Some(false),
        }
    }

    fn encode_punique_id(id: Uuid) -> PUniqueId {
        let bytes = id.as_u128().to_be_bytes();
        let (hi_bytes, lo_bytes) = bytes.split_at(8);
        let hi = i64::from_be_bytes(hi_bytes.try_into().unwrap());
        let lo = i64::from_be_bytes(lo_bytes.try_into().unwrap());
        PUniqueId { hi, lo }
    }
}

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
impl BeLoadHelper {
    /// Build a minimal POlapTableSchemaParam for tpch.lineitem using
    /// the in-memory Rust FE catalog. This keeps schema derivation
    /// strictly in the FE layer while using the generated protobuf
    /// types as the wire contract toward BE.
    pub fn build_tpch_lineitem_schema_param() -> POlapTableSchemaParam {
        Self::build_schema_param_for_table("tpch", "lineitem")
    }

    /// Build a minimal POlapTableSchemaParam for an arbitrary table in
    /// the in-memory Rust FE catalog. This mirrors the lineitem
    /// helper but is parameterized by database and table name so that
    /// the same code path can be used for any registered schema.
    pub fn build_schema_param_for_table(db_name: &str, table_name: &str) -> POlapTableSchemaParam {
        let cat = catalog::catalog();
        let table = cat
            .get_table(db_name, table_name)
            .unwrap_or_else(|| panic!("table {}.{} should exist in metadata catalog", db_name, table_name));

        // For early experiments we use placeholder IDs that can be
        // replaced by real BE metadata once wired:
        let db_id: i64 = 1;
        let table_id: i64 = 1;
        let version: i64 = 1;

        let tuple_id: i32 = 0;

        // Build slot descriptors from catalog columns
        let mut slot_descs = Vec::with_capacity(table.columns.len());
        for (i, col) in table.columns.iter().enumerate() {
            let slot_id = i as i32;
            let slot_type = Self::encode_ptype_desc_from_metadata(&col.data_type);

            let slot = PSlotDescriptor {
                id: slot_id,
                parent: tuple_id,
                slot_type,  // Not optional in real proto
                column_pos: slot_id,
                byte_offset: 0,
                null_indicator_byte: 0,
                null_indicator_bit: 0,
                col_name: col.name.clone(),
                slot_idx: slot_id,
                is_materialized: Some(true),
                col_unique_id: None,
                is_key: Some(col.primary_key),
                is_auto_increment: Some(col.auto_increment),
                col_type: None,
                column_paths: Vec::new(),
            };
            slot_descs.push(slot);
        }

        let tuple_desc = PTupleDescriptor {
            id: tuple_id,
            byte_size: 0,
            num_null_bytes: 0,
            table_id: Some(table_id),
            num_null_slots: None,
        };

        // Build a single index schema that references all logical columns.
        let index_id: i64 = 1;
        let schema_hash: i32 = 0;

        let columns: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();

        // For now we leave ColumnPB descriptors empty; they can be
        // derived from catalog types in a later increment if BE
        // requires them for correctness.
        let columns_desc = Vec::new();

        let index_schema = POlapTableIndexSchema {
            id: index_id,
            columns,
            schema_hash,
            columns_desc,
            indexes_desc: Vec::new(),
        };

        POlapTableSchemaParam {
            db_id,
            table_id,
            version,
            slot_descs,
            tuple_desc,  // Not optional in real proto
            indexes: vec![index_schema],
            partial_update: None,
            partial_update_input_columns: Vec::new(),
            is_strict_mode: Some(false),
            auto_increment_column: None,
            timestamp_ms: None,
            timezone: None,
            auto_increment_column_unique_id: None,
            nano_seconds: None,
            unique_key_update_mode: None,
            sequence_map_col_unique_id: None,
            partial_update_new_key_policy: None,
        }
    }

    fn encode_ptype_desc_from_metadata(dt: &MetaDataType) -> PTypeDesc {
        let (t, len, precision, scale) = match dt {
            MetaDataType::TinyInt => (3, None, None, None),
            MetaDataType::SmallInt => (4, None, None, None),
            MetaDataType::Int => (5, None, None, None),
            MetaDataType::BigInt => (6, None, None, None),
            MetaDataType::Float => (7, None, None, None),
            MetaDataType::Double => (8, None, None, None),
            MetaDataType::Decimal { precision, scale } => {
                (31, None, Some(*precision as i32), Some(*scale as i32))
            }
            MetaDataType::Char { length } => (13, Some(*length as i32), None, None),
            MetaDataType::Varchar { length } => (15, Some(*length as i32), None, None),
            MetaDataType::String | MetaDataType::Text => (23, None, None, None),
            MetaDataType::Date => (26, None, None, None),
            MetaDataType::DateTime | MetaDataType::Timestamp => (27, None, None, None),
            MetaDataType::Boolean => (2, None, None, None),
            MetaDataType::Binary => (11, None, None, None),
            MetaDataType::Varbinary { .. } => (43, None, None, None),
            MetaDataType::Json => (32, None, None, None),
            MetaDataType::Array(_) => (20, None, None, None),
        };

        let scalar = PScalarType {
            r#type: t,
            len,
            precision,
            scale,
        };

        let node = PTypeNode {
            r#type: 0, // SCALAR
            scalar_type: Some(scalar),
            struct_fields: Vec::new(),
            contains_null: None,
            contains_nulls: Vec::new(),
            variant_max_subcolumns_count: None,  // New field in real proto
        };

        PTypeDesc {
            types: vec![node],
        }
    }

    /// Build a minimal `PBlock` for a ParsedInsert and table schema
    /// using the legacy (be_exec_version = 0) serialization layout for
    /// numeric and string columns. This intentionally supports only a
    /// subset of Doris scalar types (integers and strings) so that we
    /// can validate the control path and on-wire format without
    /// re-implementing the entire vectorized type system in one step.
    ///
    /// For unsupported column types this returns
    /// DorisError::QueryExecution so callers can fall back or surface
    /// a clear error.
    pub fn build_pblock_for_insert(
        db_name: &str,
        table_name: &str,
        insert: &ParsedInsert,
    ) -> Result<PBlock> {
        let cat = catalog::catalog();
        let table = cat
            .get_table(db_name, table_name)
            .ok_or_else(|| DorisError::QueryExecution(format!(
                "Table '{}.{}' not found in catalog for PBlock build",
                db_name, table_name
            )))?;

        if insert.columns.len() != table.columns.len() {
            return Err(DorisError::QueryExecution(format!(
                "PBlock builder expects all columns for table '{}.{}' (got {}, expected {})",
                db_name,
                table_name,
                insert.columns.len(),
                table.columns.len()
            )));
        }

        let num_rows = insert.rows.len();

        // Build column metas and gather per-column serialized bytes.
        let mut metas: Vec<PColumnMeta> = Vec::with_capacity(table.columns.len());
        let mut column_bytes: Vec<Vec<u8>> = Vec::with_capacity(table.columns.len());

        for (col_idx, col_def) in table.columns.iter().enumerate() {
            let col_name = &col_def.name;
            let parsed_name = &insert.columns[col_idx];
            if parsed_name != col_name {
                return Err(DorisError::QueryExecution(format!(
                    "PBlock builder requires INSERT column order to match catalog; \
                     got '{}', expected '{}'",
                    parsed_name, col_name
                )));
            }

            let type_id = map_metadata_type_to_pgeneric_type(&col_def.data_type);

            let decimal_param = match &col_def.data_type {
                MetaDataType::Decimal { precision, scale } => {
                    Some(p_column_meta::Decimal {
                        precision: Some(*precision as u32),
                        scale: Some(*scale as u32),
                    })
                }
                _ => None,
            };

            let meta = PColumnMeta {
                name: Some(col_name.clone()),
                r#type: Some(type_id as i32),
                is_nullable: Some(col_def.nullable),
                decimal_param,
                children: Vec::new(),
                result_is_nullable: Some(col_def.nullable),
                function_name: None,
                be_exec_version: Some(0),
                column_path: None,
                variant_max_subcolumns_count: Some(0),
            };
            metas.push(meta);

            let col_values: Vec<String> = insert
                .rows
                .iter()
                .map(|row| row[col_idx].clone())
                .collect();

            let bytes = serialize_column_legacy(&col_def.data_type, &col_values, num_rows)?;
            column_bytes.push(bytes);
        }

        // Concatenate per-column payloads to form column_values.
        let total_len: usize = column_bytes.iter().map(|b| b.len()).sum();
        let mut column_values = Vec::with_capacity(total_len);
        for b in column_bytes {
            column_values.extend_from_slice(&b);
        }

        Ok(PBlock {
            column_metas: metas,
            column_values: Some(column_values),
            compressed: Some(false),
            uncompressed_size: None,
            compression_type: None,
            be_exec_version: Some(0),
        })
    }
}

/// Map Rust FE metadata DataType to Doris PGenericType.TypeId for
/// PColumnMeta. This keeps the mapping centralized so we can reuse it
/// for both load and scan paths.
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn map_metadata_type_to_pgeneric_type(dt: &MetaDataType) -> PTypeId {
    match dt {
        MetaDataType::TinyInt => PTypeId::Int8,
        MetaDataType::SmallInt => PTypeId::Int16,
        MetaDataType::Int => PTypeId::Int32,
        MetaDataType::BigInt => PTypeId::Int64,
        MetaDataType::Float => PTypeId::Float,
        MetaDataType::Double => PTypeId::Double,
        MetaDataType::Decimal { .. } => PTypeId::Decimal128i,
        MetaDataType::Char { .. }
        | MetaDataType::Varchar { .. }
        | MetaDataType::String
        | MetaDataType::Text => PTypeId::String,
        MetaDataType::Date => PTypeId::Datev2,
        MetaDataType::DateTime | MetaDataType::Timestamp => PTypeId::Datetimev2,
        MetaDataType::Boolean => PTypeId::Boolean,
        MetaDataType::Binary | MetaDataType::Varbinary { .. } => PTypeId::Varbinary,
        MetaDataType::Json => PTypeId::Jsonb,
        MetaDataType::Array(_) => PTypeId::List,
    }
}

/// Serialize a single logical column worth of values using the
/// "legacy" (be_exec_version = 0) layout used by
/// DataTypeNumberBase/DataTypeDecimal/DataTypeString when
/// be_exec_version < USE_CONST_SERDE. This keeps the encoding logic
/// narrow and avoids compression/streamvbyte branches for the small
/// test payloads we care about.
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn serialize_column_legacy(
    dt: &MetaDataType,
    values: &[String],
    num_rows: usize,
) -> Result<Vec<u8>> {
    use std::io::Write;

    match dt {
        MetaDataType::TinyInt => serialize_numbers::<i8>(values, num_rows),
        MetaDataType::SmallInt => serialize_numbers::<i16>(values, num_rows),
        MetaDataType::Int => serialize_numbers::<i32>(values, num_rows),
        MetaDataType::BigInt => serialize_numbers::<i64>(values, num_rows),
        MetaDataType::Boolean => {
            // Doris stores BOOLEAN as UInt8 under the hood.
            serialize_numbers::<u8>(values, num_rows)
        }
        MetaDataType::Char { .. }
        | MetaDataType::Varchar { .. }
        | MetaDataType::String
        | MetaDataType::Text => serialize_strings(values, num_rows),
        MetaDataType::Decimal { scale, .. } => {
            serialize_decimal_i128(values, num_rows, *scale)
        }
        MetaDataType::Date => serialize_date_v2(values, num_rows),
        MetaDataType::DateTime | MetaDataType::Timestamp => {
            // For now we treat DATETIME without fractional seconds
            // (scale=0) and encode as DATETIMEV2 using the packed
            // DateV2Value/DateTimeV2Value bit layout from
            // vdatetime_value.h.
            serialize_datetime_v2(values, num_rows)
        }
        // Complex types (arrays, JSON, etc.) still require
        // type-specific layouts and are not yet supported by the
        // PBlock builder.
        other => Err(DorisError::QueryExecution(format!(
            "PBlock serialization does not yet support column type {:?}",
            other
        ))),
    }
}

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn serialize_numbers<T>(values: &[String], num_rows: usize) -> Result<Vec<u8>>
where
    T: Copy + Default + TryFrom<i64> + bytemuck::Pod,
{
    if values.len() != num_rows {
        return Err(DorisError::QueryExecution(
            "serialize_numbers: row count mismatch".to_string(),
        ));
    }

    let mut raw: Vec<T> = Vec::with_capacity(num_rows);
    for v in values {
        let iv: i64 = v
            .parse()
            .map_err(|e| DorisError::QueryExecution(format!("Failed to parse numeric value '{}': {}", v, e)))?;
        let tv: T = T::try_from(iv).map_err(|_| {
            DorisError::QueryExecution(format!(
                "Value {} out of range for numeric type",
                iv
            ))
        })?;
        raw.push(tv);
    }

    let mem_size = (raw.len() * std::mem::size_of::<T>()) as u32;
    let mut buf = Vec::with_capacity(std::mem::size_of::<u32>() + mem_size as usize);

    buf.extend_from_slice(&mem_size.to_le_bytes());
    // Column data: contiguous little-endian representation.
    for v in raw {
        let bytes = bytemuck::bytes_of(&v);
        buf.extend_from_slice(bytes);
    }

    Ok(buf)
}

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn serialize_strings(values: &[String], num_rows: usize) -> Result<Vec<u8>> {
    if values.len() != num_rows {
        return Err(DorisError::QueryExecution(
            "serialize_strings: row count mismatch".to_string(),
        ));
    }

    // Emulate ColumnString layout: chars = value + '\0' per row,
    // offsets[i] = total bytes up to end of i-th string.
    let mut chars: Vec<u8> = Vec::new();
    let mut offsets: Vec<u32> = Vec::with_capacity(num_rows);

    for v in values {
        chars.extend_from_slice(v.as_bytes());
        chars.push(0); // trailing null
        offsets.push(chars.len() as u32);
    }

    let offsets_bytes = offsets.len() * std::mem::size_of::<u32>();

    // Old layout (be_exec_version < USE_CONST_SERDE):
    //   uint32 mem_size (bytes of offsets)
    //   [offsets...]
    //   uint64 value_len (bytes of chars)
    //   [chars...]
    let mut buf = Vec::with_capacity(
        std::mem::size_of::<u32>() + offsets_bytes + std::mem::size_of::<u64>() + chars.len(),
    );

    buf.extend_from_slice(&(offsets_bytes as u32).to_le_bytes());
    for off in offsets {
        buf.extend_from_slice(&off.to_le_bytes());
    }

    let value_len = chars.len() as u64;
    buf.extend_from_slice(&value_len.to_le_bytes());
    buf.extend_from_slice(&chars);

    Ok(buf)
}

/// Serialize DATEV2 values as packed u32 using the layout
///   day (5 bits) | month (4 bits) | year (23 bits)
/// matching the DateV2ValueType bitfields and MAX_DATE_V2 macro in
/// vdatetime_value.h. We then reuse the numeric serializer for
/// u32 values so the on-wire representation is compatible with
/// DataTypeNumberBase<PrimitiveType::TYPE_DATEV2>.
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn serialize_date_v2(values: &[String], num_rows: usize) -> Result<Vec<u8>> {
    if values.len() != num_rows {
        return Err(DorisError::QueryExecution(
            "serialize_date_v2: row count mismatch".to_string(),
        ));
    }

    let mut encoded: Vec<u32> = Vec::with_capacity(num_rows);
    for v in values {
        let (year, month, day) = parse_ymd(v)?;
        let y = year as u32;
        let m = month as u32;
        let d = day as u32;
        if y == 0 || y > 9999 || m == 0 || m > 12 || d == 0 || d > 31 {
            return Err(DorisError::QueryExecution(format!(
                "serialize_date_v2: out-of-range date '{}'",
                v
            )));
        }
        let packed: u32 = d | (m << 5) | (y << 9);
        encoded.push(packed);
    }

    // Use the same legacy layout as numeric types: u32 mem_size +
    // raw little-endian u32 values.
    let mem_size = (encoded.len() * std::mem::size_of::<u32>()) as u32;
    let mut buf = Vec::with_capacity(std::mem::size_of::<u32>() + mem_size as usize);
    buf.extend_from_slice(&mem_size.to_le_bytes());
    for v in encoded {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    Ok(buf)
}

/// Serialize DATETIMEV2 values as packed u64 using the layout
///   (date_v2 << 37) | (hour << 32) | (minute << 26) | (second << 20) | microsecond
/// with microsecond=0, matching the MAX_DATETIME_V2 macro in
/// vdatetime_value.h. We then reuse the numeric serializer for u64.
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn serialize_datetime_v2(values: &[String], num_rows: usize) -> Result<Vec<u8>> {
    if values.len() != num_rows {
        return Err(DorisError::QueryExecution(
            "serialize_datetime_v2: row count mismatch".to_string(),
        ));
    }

    let mut encoded: Vec<u64> = Vec::with_capacity(num_rows);
    for v in values {
        let (year, month, day, hour, minute, second) = parse_ymdhms(v)?;
        if year == 0
            || year > 9999
            || month == 0
            || month > 12
            || day == 0
            || day > 31
            || hour > 23
            || minute > 59
            || second > 59
        {
            return Err(DorisError::QueryExecution(format!(
                "serialize_datetime_v2: out-of-range datetime '{}'",
                v
            )));
        }

        let y = year as u32;
        let m = month as u32;
        let d = day as u32;
        let date_v2: u32 = d | (m << 5) | (y << 9);

        let micro: u64 = 0;
        let h = hour as u64;
        let mi = minute as u64;
        let s = second as u64;

        // From MAX_DATETIME_V2:
        //   (date_v2 << 37) | (hour << 32) | (minute << 26) | (second << 20) | microsecond
        let packed: u64 = ((date_v2 as u64) << 37)
            | (h << 32)
            | (mi << 26)
            | (s << 20)
            | micro;
        encoded.push(packed);
    }

    let mem_size = (encoded.len() * std::mem::size_of::<u64>()) as u32;
    let mut buf = Vec::with_capacity(std::mem::size_of::<u32>() + mem_size as usize);
    buf.extend_from_slice(&mem_size.to_le_bytes());
    for v in encoded {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    Ok(buf)
}

/// Serialize DECIMAL values as scaled i128 integers using the legacy
/// layout: u32 mem_size + raw little-endian i128 array. This matches
/// the "old" branch of DataTypeDecimal<T>::serialize when
/// be_exec_version < USE_CONST_SERDE, assuming DECIMAL128I storage.
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn serialize_decimal_i128(
    values: &[String],
    num_rows: usize,
    scale: u8,
) -> Result<Vec<u8>> {
    if values.len() != num_rows {
        return Err(DorisError::QueryExecution(
            "serialize_decimal_i128: row count mismatch".to_string(),
        ));
    }

    let mut raw: Vec<i128> = Vec::with_capacity(num_rows);
    for v in values {
        let iv = parse_decimal_to_scaled_i128(v, scale)?;
        raw.push(iv);
    }

    let mem_size = (raw.len() * std::mem::size_of::<i128>()) as u32;
    let mut buf = Vec::with_capacity(std::mem::size_of::<u32>() + mem_size as usize);

    buf.extend_from_slice(&mem_size.to_le_bytes());
    for v in raw {
        let bytes = bytemuck::bytes_of(&v);
        buf.extend_from_slice(bytes);
    }

    Ok(buf)
}

/// Parse a decimal string into a scaled i128 integer using the given
/// scale, e.g. "123.45" with scale=2 -> 12345.
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn parse_decimal_to_scaled_i128(s: &str, scale: u8) -> Result<i128> {
    let s = s.trim();
    if s.is_empty() {
        return Err(DorisError::QueryExecution(
            "Empty decimal value".to_string(),
        ));
    }

    let mut chars = s.chars().peekable();
    let mut sign: i128 = 1;
    if let Some(&c) = chars.peek() {
        if c == '-' {
            sign = -1;
            chars.next();
        } else if c == '+' {
            chars.next();
        }
    }

    let mut int_part = String::new();
    let mut frac_part = String::new();
    let mut seen_dot = false;

    for c in chars {
        if c == '.' {
            if seen_dot {
                return Err(DorisError::QueryExecution(format!(
                    "Invalid decimal '{}': multiple dots",
                    s
                )));
            }
            seen_dot = true;
            continue;
        }
        if !c.is_ascii_digit() {
            return Err(DorisError::QueryExecution(format!(
                "Invalid decimal '{}': non-digit character '{}'",
                s, c
            )));
        }
        if seen_dot {
            frac_part.push(c);
        } else {
            int_part.push(c);
        }
    }

    if int_part.is_empty() {
        int_part.push('0');
    }

    let int_val: i128 = int_part
        .parse()
        .map_err(|e| DorisError::QueryExecution(format!("Invalid decimal '{}': {}", s, e)))?;

    if frac_part.len() > scale as usize {
        return Err(DorisError::QueryExecution(format!(
            "Decimal '{}' has more fractional digits than scale {}",
            s, scale
        )));
    }

    // Right-pad fractional part with zeros to match scale.
    while frac_part.len() < scale as usize {
        frac_part.push('0');
    }

    let frac_val: i128 = if frac_part.is_empty() {
        0
    } else {
        frac_part
            .parse()
            .map_err(|e| DorisError::QueryExecution(format!("Invalid decimal '{}': {}", s, e)))?
    };

    let factor = 10_i128
        .checked_pow(scale as u32)
        .ok_or_else(|| DorisError::QueryExecution("Decimal scale too large".to_string()))?;

    let scaled = int_val
        .checked_mul(factor)
        .and_then(|v| v.checked_add(frac_val))
        .ok_or_else(|| DorisError::QueryExecution(format!("Decimal '{}' overflows i128", s)))?;

    Ok(sign * scaled)
}

/// Parse "YYYY-MM-DD" into (year, month, day). This matches the
/// canonical date format used by TPCH/most SQL clients; more exotic
/// formats are intentionally rejected to keep behavior predictable.
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn parse_ymd(s: &str) -> Result<(u16, u8, u8)> {
    let s = s.trim();
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(DorisError::QueryExecution(format!(
            "parse_ymd: invalid date '{}'",
            s
        )));
    }
    let year: u16 = parts[0].parse().map_err(|e| {
        DorisError::QueryExecution(format!("parse_ymd: invalid year in '{}': {}", s, e))
    })?;
    let month: u8 = parts[1].parse().map_err(|e| {
        DorisError::QueryExecution(format!("parse_ymd: invalid month in '{}': {}", s, e))
    })?;
    let day: u8 = parts[2].parse().map_err(|e| {
        DorisError::QueryExecution(format!("parse_ymd: invalid day in '{}': {}", s, e))
    })?;
    Ok((year, month, day))
}

/// Parse "YYYY-MM-DD HH:MM:SS" or "YYYY-MM-DDTHH:MM:SS" into
/// (year, month, day, hour, minute, second). We do not handle
/// fractional seconds here (scale=0).
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn parse_ymdhms(s: &str) -> Result<(u16, u8, u8, u8, u8, u8)> {
    let s = s.trim();
    let (date_str, time_str) = if let Some(space) = s.find(' ') {
        (&s[..space], &s[space + 1..])
    } else if let Some(tpos) = s.find('T') {
        (&s[..tpos], &s[tpos + 1..])
    } else {
        return Err(DorisError::QueryExecution(format!(
            "parse_ymdhms: missing time part in '{}'",
            s
        )));
    };

    let (year, month, day) = parse_ymd(date_str)?;

    let tparts: Vec<&str> = time_str.split(':').collect();
    if tparts.len() != 3 {
        return Err(DorisError::QueryExecution(format!(
            "parse_ymdhms: invalid time '{}'",
            s
        )));
    }
    let hour: u8 = tparts[0].parse().map_err(|e| {
        DorisError::QueryExecution(format!("parse_ymdhms: invalid hour in '{}': {}", s, e))
    })?;
    let minute: u8 = tparts[1].parse().map_err(|e| {
        DorisError::QueryExecution(format!("parse_ymdhms: invalid minute in '{}': {}", s, e))
    })?;
    let second: u8 = tparts[2].parse().map_err(|e| {
        DorisError::QueryExecution(format!("parse_ymdhms: invalid second in '{}': {}", s, e))
    })?;

    Ok((year, month, day, hour, minute, second))
}

#[cfg(test)]
mod tests {
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    use super::*;

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn build_open_request_shape() {
        let load_id = Uuid::nil();
        let index_id = 42_i64;
        let txn_id = 7_i64;
        let tablet_ids = vec![1001_i64, 1002_i64];

        let req = BeLoadHelper::build_open_request(load_id, index_id, txn_id, &tablet_ids);

        assert_eq!(req.index_id, index_id);
        assert_eq!(req.txn_id, txn_id);
        assert_eq!(req.num_senders, 1);
        assert_eq!(req.need_gen_rollup, false);
        assert!(req.schema.is_some());
        assert_eq!(req.tablets.len(), 2);
        assert_eq!(req.tablets[0].tablet_id, 1001);
        assert_eq!(req.tablets[1].tablet_id, 1002);
        // Partition IDs are placeholder 0 for now.
        assert_eq!(req.tablets[0].partition_id, 0);
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn build_tpch_lineitem_schema_param_shape() {
        let param = BeLoadHelper::build_tpch_lineitem_schema_param();

        assert_eq!(param.db_id, 1);
        assert_eq!(param.table_id, 1);
        assert_eq!(param.version, 1);

        // tpch.lineitem has 16 columns in the in-memory catalog.
        assert_eq!(param.slot_descs.len(), 16);
        assert!(param.tuple_desc.is_some());
        assert_eq!(param.indexes.len(), 1);

        let index = &param.indexes[0];
        assert_eq!(index.columns.len(), 16);
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn build_schema_param_for_orders_shape() {
        // Sanity check: we can build a schema param for another TPCH table.
        let param = BeLoadHelper::build_schema_param_for_table("tpch", "orders");

        assert_eq!(param.db_id, 1);
        assert_eq!(param.table_id, 1);
        assert_eq!(param.version, 1);
        assert!(param.tuple_desc.is_some());
        assert_eq!(param.indexes.len(), 1);

        let index = &param.indexes[0];
        // orders has 9 columns in the in-memory catalog.
        assert_eq!(index.columns.len(), 9);
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn build_empty_add_block_request_shape() {
        let load_id = Uuid::nil();
        let index_id = 99_i64;
        let tablet_ids = vec![2001_i64];

        let req = BeLoadHelper::build_empty_add_block_request(load_id, index_id, &tablet_ids, true);

        assert_eq!(req.index_id, index_id);
        assert_eq!(req.sender_id, 0);
        assert_eq!(req.eos, Some(true));
        assert_eq!(req.packet_seq, Some(0));
        assert_eq!(req.tablet_ids, tablet_ids);
        assert!(req.block.is_none());
        assert_eq!(req.partition_ids.len(), 0);
        assert_eq!(req.is_single_tablet_block, Some(true));
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn serialize_numbers_i32_layout() {
        // 3 INT32 values should serialize as:
        //   u32 mem_size = 3 * 4
        //   [v1, v2, v3] as little-endian i32 bytes.
        let values = vec!["1".to_string(), "2".to_string(), "3".to_string()];
        let buf = serialize_numbers::<i32>(&values, values.len()).unwrap();

        assert_eq!(buf.len(), 4 + 3 * 4);

        let mem_size = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        assert_eq!(mem_size as usize, 3 * 4);

        let mut decoded = Vec::new();
        for i in 0..3 {
            let start = 4 + i * 4;
            let v = i32::from_le_bytes(buf[start..start + 4].try_into().unwrap());
            decoded.push(v);
        }
        assert_eq!(decoded, vec![1, 2, 3]);
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn serialize_strings_layout_basic() {
        // Two strings "a", "bb" should serialize as:
        //   u32 mem_size = 2 * 4 (two offsets)
        //   offsets: [2, 5]  (a\0, a\0bb\0)
        //   u64 value_len = 5
        //   chars: "a\0bb\0"
        let values = vec!["a".to_string(), "bb".to_string()];
        let buf = serialize_strings(&values, values.len()).unwrap();

        // Offsets size
        let mem_size = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        assert_eq!(mem_size as usize, 2 * 4);

        // Offsets
        let off1 = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let off2 = u32::from_le_bytes(buf[8..12].try_into().unwrap());
        assert_eq!(off1, 2);
        assert_eq!(off2, 5);

        // Total chars length
        let value_len = u64::from_le_bytes(buf[12..20].try_into().unwrap());
        assert_eq!(value_len as usize, 5);

        let chars = &buf[20..20 + value_len as usize];
        assert_eq!(chars, b"a\0bb\0");
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn build_pblock_for_simple_int_string_table() {
        use crate::metadata::schema::{Database, Table, ColumnDef};
        use crate::metadata::types::DataType;

        // Dynamically register a small test database/table in the
        // global catalog so we can exercise the PBlock builder without
        // depending on TPCH decimals/dates yet.
        let cat = catalog::catalog();
        let mut db = Database::new("pblock_test".to_string());
        db.add_table(
            Table::new("simple".to_string())
                .with_columns(vec![
                    ColumnDef::new("id".to_string(), DataType::Int).not_null(),
                    ColumnDef::new("name".to_string(), DataType::Varchar { length: 10 }).not_null(),
                ])
        );
        cat.add_database(db);

        let insert = ParsedInsert {
            database: "pblock_test".to_string(),
            table: "simple".to_string(),
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![
                vec!["1".to_string(), "alice".to_string()],
                vec!["2".to_string(), "bob".to_string()],
            ],
        };

        let block = BeLoadHelper::build_pblock_for_insert("pblock_test", "simple", &insert)
            .expect("PBlock build should succeed for simple int/string table");

        assert_eq!(block.column_metas.len(), 2);
        assert!(block.column_values.as_ref().unwrap().len() > 0);
        assert_eq!(block.compressed, Some(false));
        assert_eq!(block.be_exec_version, Some(0));
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn serialize_decimal_i128_layout() {
        // DECIMAL(15,2) values "12.34" and "-0.56" at scale=2 should
        // serialize as scaled i128: [1234, -56] using the legacy
        // layout: u32 mem_size + raw i128 values.
        let values = vec!["12.34".to_string(), "-0.56".to_string()];
        let buf = serialize_decimal_i128(&values, values.len(), 2).unwrap();

        // mem_size = 2 * 16 bytes
        assert_eq!(buf.len(), 4 + 2 * 16);
        let mem_size = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        assert_eq!(mem_size as usize, 2 * 16);

        let v1 = i128::from_le_bytes(buf[4..20].try_into().unwrap());
        let v2 = i128::from_le_bytes(buf[20..36].try_into().unwrap());
        assert_eq!(v1, 1234);
        assert_eq!(v2, -56);
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn serialize_date_v2_layout() {
        // DATE '2024-11-05' should pack as:
        //   d | (m << 5) | (y << 9)
        // With y=2024, m=11, d=5.
        let values = vec!["2024-11-05".to_string()];
        let buf = serialize_date_v2(&values, values.len()).unwrap();

        // mem_size = 1 * 4 bytes
        assert_eq!(buf.len(), 4 + 4);
        let mem_size = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        assert_eq!(mem_size as usize, 4);

        let v = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let expected: u32 = 5 | (11 << 5) | (2024 << 9);
        assert_eq!(v, expected);
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    #[test]
    fn serialize_datetime_v2_layout() {
        // DATETIME '2024-11-05 01:02:03' with microsecond=0 should pack as:
        //   ((date_v2 << 37) | (hour << 32) | (minute << 26) | (second << 20))
        let values = vec!["2024-11-05 01:02:03".to_string()];
        let buf = serialize_datetime_v2(&values, values.len()).unwrap();

        // mem_size = 1 * 8 bytes
        assert_eq!(buf.len(), 4 + 8);
        let mem_size = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        assert_eq!(mem_size as usize, 8);

        let v = u64::from_le_bytes(buf[4..12].try_into().unwrap());
        let date_v2: u32 = 5 | (11 << 5) | (2024 << 9);
        let expected: u64 =
            ((date_v2 as u64) << 37) | (1_u64 << 32) | (2_u64 << 26) | (3_u64 << 20);
        assert_eq!(v, expected);
    }
}
