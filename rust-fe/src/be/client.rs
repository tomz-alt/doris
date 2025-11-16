use tonic::transport::Channel;
use uuid::Uuid;
use tracing::{debug, info, error};

use crate::error::{DorisError, Result};
use crate::query::QueryResult;
use crate::mysql::packet::{ColumnDefinition, ResultRow};
use crate::mysql::ColumnType;

// Generated protobuf types. When `skip_proto` is set (SKIP_PROTO=1), we
// provide minimal stubs so the code compiles without gRPC support.
#[cfg(skip_proto)]
mod be_pb {
    use std::marker::PhantomData;

    // Dummy client type so the struct definition compiles; methods are never
    // called when SKIP_PROTO=1.
    pub struct PBackendServiceClient<T>(pub PhantomData<T>);

    pub use crate::be::pb::doris::{
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
    pub use crate::be::pb::p_backend_service_client::PBackendServiceClient;
    pub use crate::be::pb::{
        PExecPlanFragmentRequest,
        PExecPlanFragmentResult,
        PFetchDataRequest,
        PFetchDataResult,
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

    #[cfg(not(skip_proto))]
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

    #[cfg(skip_proto)]
    pub async fn fetch_data(
        &mut self,
        _query_id: Uuid,
        _fragment_id: i64,
    ) -> Result<PFetchDataResult> {
        error!("BE fetch_data is not available (SKIP_PROTO=1)");
        Err(DorisError::BackendCommunication(
            "BE fetch_data disabled (SKIP_PROTO=1)".to_string(),
        ))
    }

    #[cfg(not(skip_proto))]
    pub async fn fetch_data(
        &mut self,
        query_id: Uuid,
        fragment_id: i64,
    ) -> Result<PFetchDataResult> {
        let client = self.client.as_mut().ok_or_else(|| {
            DorisError::BackendCommunication("Not connected to BE".to_string())
        })?;

        debug!("Fetching data for fragment {} of query {}", fragment_id, query_id);

        let query_id_bytes = query_id.as_bytes().to_vec();

        let request = tonic::Request::new(PFetchDataRequest {
            query_id: query_id_bytes,
            fragment_instance_id: fragment_id,
        });

        match client.fetch_data(request).await {
            Ok(response) => {
                let result = response.into_inner();
                debug!(
                    "Fetched {} bytes, eos={}",
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

    #[cfg(not(skip_proto))]
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

    fn parse_result_data(&self, data: &[u8]) -> Result<QueryResult> {
        // Simplified result parsing for PoC
        // In a real implementation, this would parse Arrow or other serialized format

        if data.is_empty() {
            return Ok(QueryResult::empty());
        }

        // Mock result for demonstration
        let columns = vec![
            ColumnDefinition::new("result".to_string(), ColumnType::VarString),
        ];

        let rows = vec![
            ResultRow::new(vec![Some(format!("Received {} bytes from BE", data.len()))]),
        ];

        Ok(QueryResult::new_select(columns, rows))
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn grpc_addr(&self) -> String {
        format!("{}:{}", self.host, self.grpc_port)
    }
}
