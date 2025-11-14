use tonic::transport::Channel;
use uuid::Uuid;
use tracing::{debug, error};

use crate::error::{DorisError, Result};
use crate::query::QueryResult;
use crate::mysql::packet::{ColumnDefinition, ColumnType, ResultRow};
use super::pb::{p_backend_service_client::PBackendServiceClient, *};

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

    pub async fn connect(&mut self) -> Result<()> {
        let addr = format!("http://{}:{}", self.host, self.grpc_port);
        debug!("Connecting to BE at {}", addr);

        match PBackendServiceClient::connect(addr.clone()).await {
            Ok(client) => {
                self.client = Some(client);
                debug!("Connected to BE at {}", addr);
                Ok(())
            }
            Err(e) => {
                error!("Failed to connect to BE at {}: {}", addr, e);
                Err(DorisError::BackendCommunication(format!("Connection failed: {}", e)))
            }
        }
    }

    pub async fn execute_query(&mut self, query_id: Uuid, query: &str) -> Result<QueryResult> {
        // Ensure connected
        if self.client.is_none() {
            self.connect().await?;
        }

        let client = self.client.as_mut()
            .ok_or_else(|| DorisError::BackendCommunication("Not connected".to_string()))?;

        let request = tonic::Request::new(PExecPlanFragmentRequest {
            query_id: query_id.as_bytes().to_vec(),
            fragment_instance_id: 1,
            query: query.to_string(),
            query_options: Default::default(),
        });

        match client.exec_plan_fragment(request).await {
            Ok(response) => {
                let result = response.into_inner();

                if result.status_code != 0 {
                    return Err(DorisError::BackendCommunication(
                        format!("BE returned error: {}", result.message)
                    ));
                }

                // Fetch data
                self.fetch_data(query_id).await
            }
            Err(e) => {
                error!("Failed to execute query on BE: {}", e);
                Err(DorisError::BackendCommunication(format!("Execution failed: {}", e)))
            }
        }
    }

    async fn fetch_data(&mut self, query_id: Uuid) -> Result<QueryResult> {
        let client = self.client.as_mut()
            .ok_or_else(|| DorisError::BackendCommunication("Not connected".to_string()))?;

        let request = tonic::Request::new(PFetchDataRequest {
            query_id: query_id.as_bytes().to_vec(),
            fragment_instance_id: 1,
        });

        match client.fetch_data(request).await {
            Ok(response) => {
                let result = response.into_inner();

                if result.status_code != 0 {
                    return Err(DorisError::BackendCommunication("Fetch failed".to_string()));
                }

                // Parse result data (simplified for PoC)
                self.parse_result_data(&result.data)
            }
            Err(e) => {
                error!("Failed to fetch data from BE: {}", e);
                Err(DorisError::BackendCommunication(format!("Fetch failed: {}", e)))
            }
        }
    }

    fn parse_result_data(&self, data: &[u8]) -> Result<QueryResult> {
        // Simplified result parsing for PoC
        // In a real implementation, this would parse the serialized result format

        if data.is_empty() {
            return Ok(QueryResult::empty());
        }

        // Mock result for demonstration
        let columns = vec![
            ColumnDefinition::new("result".to_string(), ColumnType::VarString),
        ];

        let rows = vec![
            ResultRow::new(vec![Some("Query executed on BE".to_string())]),
        ];

        Ok(QueryResult::new_select(columns, rows))
    }

    pub async fn cancel_query(&mut self, query_id: Uuid) -> Result<()> {
        if self.client.is_none() {
            return Ok(());
        }

        let client = self.client.as_mut()
            .ok_or_else(|| DorisError::BackendCommunication("Not connected".to_string()))?;

        let request = tonic::Request::new(PCancelPlanFragmentRequest {
            query_id: query_id.as_bytes().to_vec(),
            fragment_instance_id: 1,
        });

        match client.cancel_plan_fragment(request).await {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to cancel query on BE: {}", e);
                Err(DorisError::BackendCommunication(format!("Cancel failed: {}", e)))
            }
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
