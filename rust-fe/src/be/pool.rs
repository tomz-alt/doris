use std::sync::Arc;
use dashmap::DashMap;
use uuid::Uuid;
use tracing::{debug, info, error};

use crate::config::BackendNode;
use crate::error::{DorisError, Result};
use crate::query::QueryResult;
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use crate::metadata::{catalog, types::DataType as MetaDataType};
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use crate::mysql::packet::ResultRow;
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use crate::mysql::packet::ColumnDefinition;
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use crate::mysql::ColumnType;
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use thrift::protocol::{TBinaryInputProtocol, TInputProtocol, TSerializable};
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use thrift::transport::TBufferChannel;
#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
use doris_thrift::data::{TResultBatch, TRow};
use super::client::BackendClient;

#[derive(Debug)]
pub struct BackendClientPool {
    clients: DashMap<String, Arc<tokio::sync::Mutex<BackendClient>>>,
    backend_nodes: Vec<BackendNode>,
    current_index: Arc<std::sync::atomic::AtomicUsize>,
}

impl BackendClientPool {
    pub fn new(backend_nodes: Vec<BackendNode>) -> Self {
        let pool = Self {
            clients: DashMap::new(),
            backend_nodes: backend_nodes.clone(),
            current_index: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };

        // Initialize clients
        for node in &backend_nodes {
            let key = format!("{}:{}", node.host, node.grpc_port);
            let client = BackendClient::new(
                node.host.clone(),
                node.port,
                node.grpc_port,
            );
            pool.clients.insert(key, Arc::new(tokio::sync::Mutex::new(client)));
        }

        info!("Backend client pool initialized with {} nodes", backend_nodes.len());

        pool
    }

    #[cfg(skip_proto)]
    pub async fn execute_query(&self, _query_id: Uuid, _query: &str) -> Result<QueryResult> {
        // In SKIP_PROTO mode we don't have a real BE; return an empty
        // result so callers can continue without gRPC.
        Ok(QueryResult::empty())
    }

    #[cfg(all(not(skip_proto), not(feature = "real_be_proto")))]
    pub async fn execute_query(&self, query_id: Uuid, query: &str) -> Result<QueryResult> {
        // Round-robin selection
        let backend = self.select_backend();

        debug!("Executing query {} via gRPC", query_id);

        let mut client = backend.lock().await;

        // Auto-connect if not already connected
        if !client.is_connected() {
            debug!("Auto-connecting to BE");
            client.connect().await?;
        }

        // Execute fragment (use fragment_id = 0 for simple queries)
        let result = client.execute_fragment(query_id, 0, query).await?;

        if result.status_code != 0 {
            return Err(crate::error::DorisError::BackendCommunication(
                format!("BE returned error: {}", result.message)
            ));
        }

        // Fetch data from BE
        let fetch_result = client.fetch_data(query_id).await?;

        // Parse the result data
        if fetch_result.data.is_empty() {
            return Ok(QueryResult::empty());
        }

        // For now, return a simple result
        // TODO: Parse Arrow format from BE
        use crate::mysql::packet::{ColumnDefinition, ResultRow};
        use crate::mysql::ColumnType;

        let columns = vec![
            ColumnDefinition::new("result".to_string(), ColumnType::VarString),
        ];

        let rows = vec![
            ResultRow::new(vec![Some(format!("Received {} bytes from BE", fetch_result.data.len()))]),
        ];

        Ok(QueryResult::new_select(columns, rows))
    }

    /// Experimental helper that builds a minimal pipeline fragment plan
    /// for `tpch.lineitem` and sends it to BE using the real fragment
    /// path. This is kept off the main execute_select path and is
    /// intended for targeted validation only.
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    pub async fn execute_tpch_lineitem_scan_fragments(
        &self,
    ) -> Result<crate::be::PExecPlanFragmentResult> {
        use crate::be::thrift_pipeline::PipelineFragmentParamsList;
        use crate::planner::plan_fragment::{PlanFragment, PlanNode, QueryPlan};

        // Build a simple QueryPlan with a single fragment rooted at an
        // OLAP scan of tpch.lineitem. The database is implied by the
        // catalog helper used in the Thrift encoder.
        let query_id = Uuid::new_v4();
        let scan = PlanNode::OlapScan {
            table_name: "lineitem".to_string(),
            columns: vec![],
            predicates: vec![],
            tablet_ids: vec![],
        };
        let fragment = PlanFragment::new(0, scan);
        let mut plan = QueryPlan::new(query_id);
        plan.add_fragment(fragment);

        // Phase 1: Use simple single-fragment approach to get data flowing
        // TODO Phase 2: Switch to from_query_plan_multi_fragment() for proper 3-fragment structure
        let params_list = PipelineFragmentParamsList::from_query_plan(&plan)
            .ok_or_else(|| DorisError::QueryExecution(
                "Unsupported fragment shape for pipeline execution".to_string(),
            ))?;

        let bytes = params_list.to_thrift_bytes();

        // Dump payload for verification against Java FE
        if let Err(e) = std::fs::write("/tmp/rust-fe-thrift.bin", &bytes) {
            debug!("Failed to write Thrift payload to /tmp/rust-fe-thrift.bin: {}", e);
        } else {
            debug!("Saved Thrift payload: {} bytes to /tmp/rust-fe-thrift.bin", bytes.len());
        }

        // Round-robin selection
        let backend = self.select_backend();
        let mut client = backend.lock().await;

        if !client.is_connected() {
            debug!("Auto-connecting to BE for pipeline fragments");
            client.connect().await?;
        }

        client.exec_pipeline_fragments(plan.query_id, bytes).await
    }

    /// Execute a simple SELECT query via the real pipeline fragments API,
    /// returning real rows via the Doris internal `fetch_data` row-batch
    /// API for MySQL-compatible text protocol.
    ///
    /// This currently only supports very simple single-fragment plans
    /// that `PipelineFragmentParamsList::from_query_plan` accepts. For
    /// other shapes callers should fall back to the DataFusion path.
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    pub async fn execute_pipeline_query(
        &self,
        plan: crate::planner::plan_fragment::QueryPlan,
        db_name: Option<&str>,
    ) -> Result<QueryResult> {
        use crate::be::thrift_pipeline::PipelineFragmentParamsList;

        // Phase 1: Use simple single-fragment approach to get data flowing
        // TODO Phase 2: Switch to from_query_plan_multi_fragment() for proper 3-fragment structure
        let params_list = PipelineFragmentParamsList::from_query_plan(&plan)
            .ok_or_else(|| DorisError::QueryExecution(
                "Unsupported fragment shape for pipeline execution".to_string(),
            ))?;

        // Infer table name for schema lookup from the single fragment.
        let table_name = params_list
            .fragments
            .first()  // Single fragment in Phase 1
            .and_then(|f| f.table_name.as_deref())
            .ok_or_else(|| DorisError::QueryExecution(
                "Missing table name in pipeline fragment params".to_string(),
            ))?;

        let db = db_name.unwrap_or("tpch_sf1");
        let columns = build_mysql_columns_for_table(db, table_name)?;

        let bytes = params_list.to_thrift_bytes();

        // Dump payload for verification and Java FE comparison.
        if let Err(e) = std::fs::write("/tmp/rust-fe-thrift.bin", &bytes) {
            debug!("Failed to write Thrift payload to /tmp/rust-fe-thrift.bin: {}", e);
        } else {
            debug!("Saved Thrift payload: {} bytes to /tmp/rust-fe-thrift.bin", bytes.len());
        }

        // Round-robin selection
        let backend = self.select_backend();
        let mut client = backend.lock().await;

        if !client.is_connected() {
            debug!("Auto-connecting to BE for pipeline fragments");
            client.connect().await?;
        }

        let exec_result = client.exec_pipeline_fragments(plan.query_id, bytes).await?;
        debug!("Pipeline fragments executed with status: {:?}", exec_result.status);

        // Fetch data (row-batch format) until EOS - this is the proven Java FE approach.
        // The BE encodes MySQL text rows into `data.TResultBatch.rows` as binary
        // payloads, so we must decode using the MySQL text helpers rather than
        // assuming Thrift `TRow` structs.
        let mut rows: Vec<ResultRow> = Vec::new();

        loop {
            let fetch = client.fetch_data(plan.query_id).await?;

            // Check status (required field in protobuf)
            if fetch.status.status_code != 0 {
                let msg = if !fetch.status.error_msgs.is_empty() {
                    fetch.status.error_msgs.join("; ")
                } else {
                    format!("BE fetch_data failed with status_code={}", fetch.status.status_code)
                };
                return Err(DorisError::BackendCommunication(msg));
            }

            if fetch.eos.unwrap_or(false) {
                info!("Reached EOS, total rows fetched: {}", rows.len());
                break;
            }

            // Check for empty batch
            if fetch.empty_batch.unwrap_or(false) {
                debug!("Received empty batch, continuing...");
                continue;
            }

            // Parse row_batch (Thrift TResultBatch format)
            if let Some(row_batch_bytes) = &fetch.row_batch {
                info!("Received row_batch with {} bytes", row_batch_bytes.len());

                // Decode MySQL text rows from TResultBatch
                match decode_mysql_text_row_batch(row_batch_bytes, columns.len()) {
                    Ok(batch_rows) => {
                        info!("Decoded {} rows from batch", batch_rows.len());
                        rows.extend(batch_rows);
                    }
                    Err(e) => {
                        error!("Failed to decode row_batch: {}", e);
                        return Err(e);
                    }
                }
            } else {
                debug!("No row_batch in response");
            }
        }

        Ok(QueryResult::new_select(columns, rows))
    }

    /// Decode Thrift TResultBatch where each row is encoded in MySQL text format.
    ///
    /// NOTE: Doris BE's pipeline `ResultSink` for MYSQL_PROTOCOL uses
    /// `data.TResultBatch.rows: list<binary>` where each entry is a
    /// MySQL-compatible text row (length-encoded fields), not a serialized
    /// `TRow`/`TCell` struct. For that reason, `execute_pipeline_query`
    /// uses `decode_mysql_text_row_batch` instead of this helper.
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    fn decode_row_batch(
        batch_bytes: &[u8],
        columns: &[ColumnDefinition],
    ) -> Result<Vec<ResultRow>> {
        use thrift::protocol::{TBinaryInputProtocol, TInputProtocol};
        use thrift::transport::TBufferChannel;

        let mut transport = TBufferChannel::with_capacity(batch_bytes.len(), 0);
        transport.set_readable_bytes(batch_bytes);

        let mut protocol = TBinaryInputProtocol::new(transport, true);

        // Decode TResultBatch
        let result_batch = TResultBatch::read_from_in_protocol(&mut protocol)
            .map_err(|e| DorisError::BackendCommunication(format!("Failed to decode TResultBatch: {}", e)))?;

        debug!("TResultBatch: is_compressed={}, packet_seq={}, rows_len={}",
               result_batch.is_compressed,
               result_batch.packet_seq,
               result_batch.rows.len());

        // Check if compressed
        if result_batch.is_compressed {
            return Err(DorisError::BackendCommunication(
                "Compressed row batches not yet supported".to_string()
            ));
        }

        let mut rows = Vec::new();

        // Each element in result_batch.rows is a serialized TRow
        for row_bytes in result_batch.rows {
            let mut row_transport = TBufferChannel::with_capacity(row_bytes.len(), 0);
            row_transport.set_readable_bytes(&row_bytes);
            let mut row_protocol = TBinaryInputProtocol::new(row_transport, true);

            // Deserialize TRow
            let trow = TRow::read_from_in_protocol(&mut row_protocol)
                .map_err(|e| DorisError::BackendCommunication(format!("Failed to decode TRow: {}", e)))?;

            // Convert TRow to ResultRow
            let mut row_values = Vec::with_capacity(columns.len());

            if let Some(col_vals) = trow.column_value {
                for tcell in col_vals {
                    // Convert TCell to Option<String>
                    let value = if tcell.is_null.unwrap_or(false) {
                        None
                    } else if let Some(s) = tcell.string_val {
                        Some(s)
                    } else if let Some(i) = tcell.int_val {
                        Some(i.to_string())
                    } else if let Some(l) = tcell.long_val {
                        Some(l.to_string())
                    } else if let Some(d) = tcell.double_val {
                        Some(d.to_string())
                    } else if let Some(b) = tcell.bool_val {
                        Some(b.to_string())
                    } else {
                        None
                    };
                    row_values.push(value);
                }
            }

            rows.push(ResultRow::new(row_values));
        }

        Ok(rows)
    }

    /// Experimental load helper for inserting rows into the BE for
    /// `tpch.lineitem` using the real Doris tablet-writer API. This
    /// currently only exercises the control path (open + add_block
    /// with an empty payload); value-to-PBlock encoding is a separate
    /// milestone.
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    pub async fn load_tpch_lineitem(
        &self,
        insert: &crate::query::ParsedInsert,
    ) -> Result<u64> {
        use crate::be::load::BeLoadHelper;

        if self.backend_nodes.is_empty() {
            return Err(DorisError::BackendCommunication(
                "No backend nodes configured for BE load".to_string(),
            ));
        }

        if insert.database != "tpch" || insert.table != "lineitem" {
            return Err(DorisError::QueryExecution(format!(
                "BE load currently only supports tpch.lineitem, got {}.{}",
                insert.database, insert.table
            )));
        }

        // TODO: derive these IDs and tablet list from real BE
        // metadata (e.g. GetTabletMeta RPC). For now we use
        // placeholders so the request shape can be validated against
        // Java FE / BE logs.
        let load_id = Uuid::new_v4();
        let index_id: i64 = 1;
        let txn_id: i64 = 1;
        let tablet_ids: Vec<i64> = vec![1];

        let mut open_req =
            BeLoadHelper::build_open_request(load_id, index_id, txn_id, &tablet_ids);
        open_req.schema = BeLoadHelper::build_tpch_lineitem_schema_param();  // Not optional

        let backend = self.select_backend();
        let mut client = backend.lock().await;

        if !client.is_connected() {
            debug!("Auto-connecting to BE for tablet writer");
            client.connect().await?;
        }

        let open_res = client.tablet_writer_open(open_req).await?;
        debug!("tablet_writer_open status: {:?}", open_res.status);

        // Build a minimal PBlock from the ParsedInsert. For now this
        // only supports a subset of scalar types (integers/strings),
        // so unsupported schemas will surface a clear error.
        let block = BeLoadHelper::build_pblock_for_insert("tpch", "lineitem", insert)?;

        let mut add_req =
            BeLoadHelper::build_empty_add_block_request(load_id, index_id, &tablet_ids, true);
        add_req.block = Some(block);

        let add_res = client.tablet_writer_add_block(add_req).await?;
        debug!("tablet_writer_add_block status: {:?}", add_res.status);

        Ok(insert.rows.len() as u64)
    }

    pub async fn cancel_query(&self, query_id: Uuid) -> Result<()> {
        // Cancel on all backends (broadcast)
        let mut handles = vec![];

        for entry in self.clients.iter() {
            let client = entry.value().clone();
            let qid = query_id;

            let handle = tokio::spawn(async move {
                let mut client = client.lock().await;
                // Cancel fragment 0
                let _ = client.cancel_fragment(qid, 0).await;
            });

            handles.push(handle);
        }

        // Wait for all cancellations
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    fn select_backend(&self) -> Arc<tokio::sync::Mutex<BackendClient>> {
        if self.backend_nodes.is_empty() {
            panic!("No backend nodes configured");
        }

        let index = self.current_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let node = &self.backend_nodes[index % self.backend_nodes.len()];
        let key = format!("{}:{}", node.host, node.grpc_port);

        self.clients.get(&key)
            .expect("Backend client should exist")
            .value()
            .clone()
    }

    pub fn backend_count(&self) -> usize {
        self.backend_nodes.len()
    }
}

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn build_mysql_columns_for_table(db_name: &str, table_name: &str) -> Result<Vec<ColumnDefinition>> {
    let cat = catalog::catalog();
    let table = cat
        .get_table(db_name, table_name)
        .ok_or_else(|| DorisError::QueryExecution(
            format!("Table {}.{} not found in catalog", db_name, table_name),
        ))?;

    let mut cols = Vec::with_capacity(table.columns.len());
    for col in &table.columns {
        let ty = match &col.data_type {
            MetaDataType::Boolean => ColumnType::Tiny,
            MetaDataType::TinyInt => ColumnType::Tiny,
            MetaDataType::SmallInt => ColumnType::Short,
            MetaDataType::Int => ColumnType::Long,
            MetaDataType::BigInt => ColumnType::LongLong,
            MetaDataType::Float => ColumnType::Float,
            MetaDataType::Double => ColumnType::Double,
            MetaDataType::Decimal { .. } => ColumnType::Decimal,
            MetaDataType::Char { .. } |
            MetaDataType::Varchar { .. } |
            MetaDataType::String |
            MetaDataType::Text => ColumnType::VarString,
            MetaDataType::Date => ColumnType::Date,
            MetaDataType::DateTime |
            MetaDataType::Timestamp => ColumnType::DateTime,
            MetaDataType::Binary |
            MetaDataType::Varbinary { .. } |
            MetaDataType::Json |
            MetaDataType::Array(_) => ColumnType::VarString,
        };
        cols.push(ColumnDefinition::new(col.name.clone(), ty));
    }
    Ok(cols)
}

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn decode_mysql_text_row_batch(data: &[u8], num_columns: usize) -> Result<Vec<ResultRow>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let mut channel = TBufferChannel::with_capacity(data.len(), 0);
    channel.set_readable_bytes(data);
    let mut i_prot = TBinaryInputProtocol::new(&mut channel, true);

    let batch = TResultBatch::read_from_in_protocol(&mut i_prot).map_err(|e| {
        DorisError::QueryExecution(format!("Failed to decode TResultBatch: {}", e))
    })?;

    info!("TResultBatch decoded: {} rows in batch, is_compressed={:?}, packet_seq={}, expected_columns={}",
          batch.rows.len(),
          batch.is_compressed,
          batch.packet_seq,
          num_columns);

    let mut rows = Vec::with_capacity(batch.rows.len());
    for row_buf in &batch.rows {  // Borrow to avoid move
        let decoded = decode_mysql_text_row(row_buf, num_columns)?;
        rows.push(ResultRow::new(decoded));
    }
    Ok(rows)
}

#[cfg(all(not(skip_proto), feature = "real_be_proto"))]
fn decode_mysql_text_row(buf: &[u8], num_columns: usize) -> Result<Vec<Option<String>>> {
    let mut values = Vec::with_capacity(num_columns);
    let mut pos = 0usize;

    for _ in 0..num_columns {
        if pos >= buf.len() {
            values.push(None);
            continue;
        }

        let flag = buf[pos];
        pos += 1;

        if flag == 251 {
            values.push(None);
            continue;
        }

        let len: usize = match flag {
            0..=250 => flag as usize,
            252 => {
                if pos + 2 > buf.len() {
                    return Err(DorisError::QueryExecution(
                        "Invalid length-encoded field (252)".to_string(),
                    ));
                }
                let v = u16::from_le_bytes([buf[pos], buf[pos + 1]]) as usize;
                pos += 2;
                v
            }
            253 => {
                if pos + 3 > buf.len() {
                    return Err(DorisError::QueryExecution(
                        "Invalid length-encoded field (253)".to_string(),
                    ));
                }
                let v = (buf[pos] as usize)
                    | ((buf[pos + 1] as usize) << 8)
                    | ((buf[pos + 2] as usize) << 16);
                pos += 3;
                v
            }
            254 => {
                if pos + 8 > buf.len() {
                    return Err(DorisError::QueryExecution(
                        "Invalid length-encoded field (254)".to_string(),
                    ));
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&buf[pos..pos + 8]);
                pos += 8;
                u64::from_le_bytes(bytes) as usize
            }
            _ => {
                return Err(DorisError::QueryExecution(
                    format!("Unsupported length-encoded flag {}", flag),
                ))
            }
        };

        if pos + len > buf.len() {
            return Err(DorisError::QueryExecution(
                "Length-encoded field exceeds buffer".to_string(),
            ));
        }

        let val_bytes = &buf[pos..pos + len];
        pos += len;

        let s = String::from_utf8_lossy(val_bytes).to_string();
        values.push(Some(s));
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_mysql_text_row_basic() {
        let mut buf = Vec::new();
        // "5" (len=1)
        buf.push(1u8);
        buf.extend_from_slice(b"5");
        // "120" (len=3)
        buf.push(3u8);
        buf.extend_from_slice(b"120");

        let row = decode_mysql_text_row(&buf, 2).unwrap();
        assert_eq!(row.len(), 2);
        assert_eq!(row[0].as_deref(), Some("5"));
        assert_eq!(row[1].as_deref(), Some("120"));
    }
}
