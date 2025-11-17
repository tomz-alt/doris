use tonic::transport::Channel;
use uuid::Uuid;
use tracing::{debug, info, error};

use crate::error::{DorisError, Result};
use crate::query::QueryResult;
use crate::mysql::packet::{ColumnDefinition, ResultRow};
use crate::mysql::ColumnType;

// Generated protobuf types. When `SKIP_PROTO=1` is set, we provide minimal
// stubs so the code compiles without gRPC support.
#[cfg(skip_proto)]
mod be_pb {
    use std::marker::PhantomData;

    // Dummy client type so the struct definition compiles; methods are never
    // called when SKIP_PROTO=1.
    pub struct PBackendServiceClient<T>(pub PhantomData<T>);

    pub use crate::be::doris::{
        PExecPlanFragmentRequest,
        PExecPlanFragmentResult,
        PFetchDataRequest,
        PFetchDataResult,
        PCancelPlanFragmentRequest,
        PCancelPlanFragmentResult,
    };
}

#[cfg(not(skip_proto))]
mod be_pb {
    pub use crate::be::doris::p_backend_service_client::PBackendServiceClient;
    pub use crate::be::doris::{
        PExecPlanFragmentRequest,
        PExecPlanFragmentResult,
        PFetchDataRequest,
        PFetchDataResult,
        PFetchArrowDataRequest,
        PFetchArrowDataResult,
        PTabletWriterOpenRequest,
        PTabletWriterOpenResult,
        PTabletWriterAddBlockRequest,
        PTabletWriterAddBlockResult,
        PStatus,
        PCancelPlanFragmentRequest,
        PCancelPlanFragmentResult,
    };
}

use be_pb::*;

pub struct BackendClient {
    host: String,
    port: u16,
    grpc_port: u16,
    client: Option<PBackendServiceClient<Channel>>,
}

impl std::fmt::Debug for BackendClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackendClient")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("grpc_port", &self.grpc_port)
            .field("client", &self.client.is_some())
            .finish()
    }
}

impl BackendClient {
    pub fn new(host: String, port: u16, grpc_port: u16) -> Self {
        Self {
            host,
            port,
            grpc_port,
            client: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_some()
    }

    #[cfg(skip_proto)]
    pub async fn connect(&mut self) -> Result<()> {
        error!("BE gRPC client is not available (SKIP_PROTO=1)");
        Err(DorisError::BackendCommunication(
            "BE gRPC client is disabled (SKIP_PROTO=1)".to_string(),
        ))
    }

    #[cfg(not(skip_proto))]
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to BE at {}:{}", self.host, self.grpc_port);

        let addr = format!("http://{}:{}", self.host, self.grpc_port);

        match Channel::from_shared(addr.clone()) {
            Ok(endpoint) => {
                match endpoint.connect().await {
                    Ok(channel) => {
                        self.client = Some(PBackendServiceClient::new(channel));
                        info!("Successfully connected to BE at {}", addr);
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to connect to BE: {}", e);
                        Err(DorisError::BackendCommunication(format!(
                            "Failed to connect to BE at {}: {}",
                            addr, e
                        )))
                    }
                }
            }
            Err(e) => {
                error!("Invalid BE address {}: {}", addr, e);
                Err(DorisError::BackendCommunication(format!(
                    "Invalid BE address {}: {}",
                    addr, e
                )))
            }
        }
    }

    #[cfg(skip_proto)]
    pub async fn execute_fragment(
        &mut self,
        _query_id: Uuid,
        _fragment_id: i64,
        _query_sql: &str,
    ) -> Result<PExecPlanFragmentResult> {
        error!("BE fragment execution is not available (SKIP_PROTO=1)");
        Err(DorisError::BackendCommunication(
            "BE fragment execution disabled (SKIP_PROTO=1)".to_string(),
        ))
    }

    #[cfg(all(not(skip_proto), not(feature = "real_be_proto")))]
    pub async fn execute_fragment(
        &mut self,
        query_id: Uuid,
        fragment_id: i64,
        query_sql: &str,
    ) -> Result<PExecPlanFragmentResult> {
        let client = self.client.as_mut().ok_or_else(|| {
            DorisError::BackendCommunication("Not connected to BE".to_string())
        })?;

        debug!(
            "Executing fragment {} for query {} on BE",
            fragment_id, query_id
        );

        // Convert UUID to bytes
        let query_id_bytes = query_id.as_bytes().to_vec();

        let request = tonic::Request::new(PExecPlanFragmentRequest {
            query_id: query_id_bytes,
            fragment_instance_id: fragment_id,
            query: query_sql.to_string(),
            query_options: std::collections::HashMap::new(),
        });

        match client.exec_plan_fragment(request).await {
            Ok(response) => {
                let result = response.into_inner();
                debug!("Fragment execution result: status_code={}", result.status_code);
                Ok(result)
            }
            Err(e) => {
                error!("Failed to execute fragment: {}", e);
                Err(DorisError::BackendCommunication(format!(
                    "Failed to execute fragment: {}",
                    e
                )))
            }
        }
    }

    /// Execute a pipeline-fragment-based request using the real Doris
    /// internal_service.proto API. The caller is responsible for
    /// constructing the serialized Thrift
    /// `TPipelineFragmentParamsList` payload. This uses the two-phase
    /// exec_plan_fragment_prepare + exec_plan_fragment_start sequence
    /// to mirror Java FE behavior.
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    pub async fn exec_pipeline_fragments(
        &mut self,
        query_id: Uuid,
        request_bytes: Vec<u8>,
    ) -> Result<PExecPlanFragmentResult> {
        use crate::be::doris::{PExecPlanFragmentStartRequest, PFragmentRequestVersion, PUniqueId};

        let client = self.client.as_mut().ok_or_else(|| {
            DorisError::BackendCommunication("Not connected to BE".to_string())
        })?;

        info!(
            "Executing pipeline fragments request ({} bytes) on BE",
            request_bytes.len()
        );
        info!("Phase 1: Calling exec_plan_fragment_prepare()...");

        // Phase 1: prepare (send TPipelineFragmentParamsList)
        //
        // The Rust FE now encodes `TPipelineFragmentParamsList` using the
        // Thrift Compact Protocol (`CompactEncode`). To match Java FE
        // behavior and ensure BE deserializes the payload correctly, we
        // must set `compact = true` here.
        let prepare_request = tonic::Request::new(PExecPlanFragmentRequest {
            request: Some(request_bytes),
            compact: Some(true),
            // VERSION_3 = TPipelineFragmentParamsList
            version: Some(PFragmentRequestVersion::Version3 as i32),
        });

        let prepare_result = match client.exec_plan_fragment_prepare(prepare_request).await {
            Ok(response) => response.into_inner(),
            Err(e) => {
                error!("Failed to prepare pipeline fragments: {}", e);
                return Err(DorisError::BackendCommunication(format!(
                    "Failed to prepare pipeline fragments: {}",
                    e
                )));
            }
        };

        info!("Pipeline fragment prepare status received: {:?}", prepare_result.status);
        info!("Phase 2: Calling exec_plan_fragment_start()...");

        // Phase 2: start execution for this query_id
        let bytes = query_id.as_u128().to_be_bytes();
        let (hi_bytes, lo_bytes) = bytes.split_at(8);
        let hi = i64::from_be_bytes(hi_bytes.try_into().unwrap());
        let lo = i64::from_be_bytes(lo_bytes.try_into().unwrap());

        let start_request = tonic::Request::new(PExecPlanFragmentStartRequest {
            query_id: Some(PUniqueId { hi, lo }),
        });

        match client.exec_plan_fragment_start(start_request).await {
            Ok(response) => {
                let result = response.into_inner();
                info!("Pipeline fragment start finished");
                Ok(result)
            }
            Err(e) => {
                error!("Failed to start pipeline fragments: {}", e);
                Err(DorisError::BackendCommunication(format!(
                    "Failed to start pipeline fragments: {}",
                    e
                )))
            }
        }
    }

    /// Open a Doris tablet writer for a given index/tablet set using
    /// the real internal_service.proto API. This is a thin wrapper
    /// around `tablet_writer_open` and is only available when
    /// `real_be_proto` is enabled.
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    pub async fn tablet_writer_open(
        &mut self,
        request: PTabletWriterOpenRequest,
    ) -> Result<PTabletWriterOpenResult> {
        let client = self.client.as_mut().ok_or_else(|| {
            DorisError::BackendCommunication("Not connected to BE".to_string())
        })?;

        debug!(
            "Opening tablet writer on BE (index_id={}, tablets={})",
            request.index_id,
            request.tablets.len()
        );

        let req = tonic::Request::new(request);
        match client.tablet_writer_open(req).await {
            Ok(response) => {
                let result = response.into_inner();
                // PStatus is not optional in proto
                let status_code = result.status.status_code;
                if status_code != 0 {
                    let msg = if !result.status.error_msgs.is_empty() {
                        result.status.error_msgs.join("; ")
                    } else {
                        format!(
                            "tablet_writer_open failed with status_code={}",
                            status_code
                        )
                    };
                    return Err(DorisError::BackendCommunication(msg));
                }
                Ok(result)
            }
            Err(e) => {
                error!("tablet_writer_open RPC failed: {}", e);
                Err(DorisError::BackendCommunication(format!(
                    "tablet_writer_open RPC failed: {}",
                    e
                )))
            }
        }
    }

    /// Send a `PTabletWriterAddBlockRequest` to BE. The caller is
    /// responsible for constructing the `PBlock` payload and tablet
    /// routing. This wrapper only handles transport errors and maps
    /// `PStatus` into DorisError.
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    pub async fn tablet_writer_add_block(
        &mut self,
        request: PTabletWriterAddBlockRequest,
    ) -> Result<PTabletWriterAddBlockResult> {
        let client = self.client.as_mut().ok_or_else(|| {
            DorisError::BackendCommunication("Not connected to BE".to_string())
        })?;

        debug!(
            "Sending tablet_writer_add_block to BE (tablet_ids={}, eos={:?})",
            request.tablet_ids.len(),
            request.eos
        );

        let req = tonic::Request::new(request);
        match client.tablet_writer_add_block(req).await {
            Ok(response) => {
                let result = response.into_inner();
                // PStatus is not optional in proto
                let status_code = result.status.status_code;
                if status_code != 0 {
                    let msg = if !result.status.error_msgs.is_empty() {
                        result.status.error_msgs.join("; ")
                    } else {
                        format!(
                            "tablet_writer_add_block failed with status_code={}",
                            status_code
                        )
                    };
                    return Err(DorisError::BackendCommunication(msg));
                }
                Ok(result)
            }
            Err(e) => {
                error!("tablet_writer_add_block RPC failed: {}", e);
                Err(DorisError::BackendCommunication(format!(
                    "tablet_writer_add_block RPC failed: {}",
                    e
                )))
            }
        }
    }

    #[cfg(skip_proto)]
    pub async fn fetch_data(
        &mut self,
        _query_id: Uuid,
    ) -> Result<PFetchDataResult> {
        error!("BE fetch_data is not available (SKIP_PROTO=1)");
        Err(DorisError::BackendCommunication(
            "BE fetch_data disabled (SKIP_PROTO=1)".to_string(),
        ))
    }

    #[cfg(all(not(skip_proto), not(feature = "real_be_proto")))]
    pub async fn fetch_data(
        &mut self,
        query_id: Uuid,
    ) -> Result<PFetchDataResult> {
        let client = self.client.as_mut().ok_or_else(|| {
            DorisError::BackendCommunication("Not connected to BE".to_string())
        })?;

        debug!("Fetching data for query {}", query_id);

        let query_id_bytes = query_id.as_bytes().to_vec();

        let request = tonic::Request::new(PFetchDataRequest {
            query_id: query_id_bytes,
            fragment_instance_id: 0,
        });

        match client.fetch_data(request).await {
            Ok(response) => {
                let result = response.into_inner();
                debug!(
                    "Fetched {} bytes (mock), eos={:?}",
                    result.data.len(),
                    result.eos
                );
                Ok(result)
            }
            Err(e) => {
                error!("Failed to fetch data: {}", e);
                Err(DorisError::BackendCommunication(format!(
                    "Failed to fetch data: {}",
                    e
                )))
            }
        }
    }

    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    pub async fn fetch_data(
        &mut self,
        query_id: Uuid,
    ) -> Result<PFetchDataResult> {
        use crate::be::doris::PUniqueId;

        let client = self.client.as_mut().ok_or_else(|| {
            DorisError::BackendCommunication("Not connected to BE".to_string())
        })?;

        debug!("Fetching data for query {}", query_id);

        let bytes = query_id.as_u128().to_be_bytes();
        let (hi_bytes, lo_bytes) = bytes.split_at(8);
        let hi = i64::from_be_bytes(hi_bytes.try_into().unwrap());
        let lo = i64::from_be_bytes(lo_bytes.try_into().unwrap());

        let finst_id = PUniqueId { hi, lo };

        let request = tonic::Request::new(PFetchDataRequest {
            finst_id,  // PUniqueId is not optional
            resp_in_attachment: None,
        });

        match client.fetch_data(request).await {
            Ok(response) => {
                let result = response.into_inner();
                debug!(
                    "PFetchDataResult: status={:?}, packet_seq={:?}, eos={:?}, empty_batch={:?}, row_batch_len={:?}",
                    result.status,
                    result.packet_seq,
                    result.eos,
                    result.empty_batch,
                    result.row_batch.as_ref().map(|b| b.len())
                );
                // If we have row_batch data, log a hex preview
                if let Some(ref row_batch) = result.row_batch {
                    let preview_len = row_batch.len().min(64);
                    debug!(
                        "Row batch data (first {} bytes): {:02x?}",
                        preview_len,
                        &row_batch[..preview_len]
                    );
                }
                Ok(result)
            }
            Err(e) => {
                error!("Failed to fetch data: {}", e);
                Err(DorisError::BackendCommunication(format!(
                    "Failed to fetch data: {}",
                    e
                )))
            }
        }
    }

    /// Fetch data from BE using modern Arrow/PBlock API (for future use)
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    pub async fn fetch_arrow_data(
        &mut self,
        query_id: Uuid,
    ) -> Result<crate::be::doris::PFetchArrowDataResult> {
        use crate::be::doris::{PUniqueId, PFetchArrowDataRequest};

        let client = self.client.as_mut().ok_or_else(|| {
            DorisError::BackendCommunication("Not connected to BE".to_string())
        })?;

        debug!("Fetching arrow data for query {}", query_id);

        let bytes = query_id.as_u128().to_be_bytes();
        let (hi_bytes, lo_bytes) = bytes.split_at(8);
        let hi = i64::from_be_bytes(hi_bytes.try_into().unwrap());
        let lo = i64::from_be_bytes(lo_bytes.try_into().unwrap());

        let finst_id = PUniqueId { hi, lo };

        let request = tonic::Request::new(PFetchArrowDataRequest {
            finst_id: Some(finst_id),
        });

        match client.fetch_arrow_data(request).await {
            Ok(response) => {
                let result = response.into_inner();
                debug!(
                    "PFetchArrowDataResult: status={:?}, packet_seq={:?}, eos={:?}, block_present={}",
                    result.status,
                    result.packet_seq,
                    result.eos,
                    result.block.is_some()
                );
                // If we have block data, log some info
                if let Some(ref block) = result.block {
                    debug!(
                        "PBlock received: column_metas={}",
                        block.column_metas.len()
                    );
                }
                Ok(result)
            }
            Err(e) => {
                error!("Failed to fetch arrow data: {}", e);
                Err(DorisError::BackendCommunication(format!(
                    "Failed to fetch arrow data: {}",
                    e
                )))
            }
        }
    }

    #[cfg(skip_proto)]
    pub async fn cancel_fragment(
        &mut self,
        _query_id: Uuid,
        _fragment_id: i64,
    ) -> Result<PCancelPlanFragmentResult> {
        error!("BE cancel_fragment is not available (SKIP_PROTO=1)");
        Err(DorisError::BackendCommunication(
            "BE cancel_fragment disabled (SKIP_PROTO=1)".to_string(),
        ))
    }

    #[cfg(all(not(skip_proto), not(feature = "real_be_proto")))]
    pub async fn cancel_fragment(
        &mut self,
        query_id: Uuid,
        fragment_id: i64,
    ) -> Result<PCancelPlanFragmentResult> {
        let client = self.client.as_mut().ok_or_else(|| {
            DorisError::BackendCommunication("Not connected to BE".to_string())
        })?;

        debug!("Canceling fragment {} of query {}", fragment_id, query_id);

        let query_id_bytes = query_id.as_bytes().to_vec();

        let request = tonic::Request::new(PCancelPlanFragmentRequest {
            query_id: query_id_bytes,
            fragment_instance_id: fragment_id,
        });

        match client.cancel_plan_fragment(request).await {
            Ok(response) => {
                let result = response.into_inner();
                debug!("Cancel result: status_code={}", result.status_code);
                Ok(result)
            }
            Err(e) => {
                error!("Failed to cancel fragment: {}", e);
                Err(DorisError::BackendCommunication(format!(
                    "Failed to cancel fragment: {}",
                    e
                )))
            }
        }
    }

    /// `cancel_fragment` is not yet implemented for the real Doris
    /// internal_service.proto path. For now this always returns an
    /// error so that experimental builds can compile.
    #[cfg(all(not(skip_proto), feature = "real_be_proto"))]
    pub async fn cancel_fragment(
        &mut self,
        _query_id: Uuid,
        _fragment_id: i64,
    ) -> Result<PCancelPlanFragmentResult> {
        Err(DorisError::BackendCommunication(
            "cancel_fragment is not implemented for real_be_proto".to_string(),
        ))
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.host, self.grpc_port)
    }
}
