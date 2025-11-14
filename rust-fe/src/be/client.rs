use tonic::transport::Channel;
use uuid::Uuid;
use tracing::{debug, info, error};

use crate::error::{DorisError, Result};
use crate::query::QueryResult;
use crate::mysql::packet::{ColumnDefinition, ResultRow};
use crate::mysql::ColumnType;

// Include generated protobuf code
mod pb {
    include!(concat!(env!("OUT_DIR"), "/doris.rs"));
}

use pb::p_backend_service_client::PBackendServiceClient;
use pb::{PExecPlanFragmentRequest, PExecPlanFragmentResult};
use pb::{PFetchDataRequest, PFetchDataResult};
use pb::{PCancelPlanFragmentRequest, PCancelPlanFragmentResult};

pub struct BackendClient {
    host: String,
    port: u16,
    grpc_port: u16,
    client: Option<PBackendServiceClient<Channel>>,
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
