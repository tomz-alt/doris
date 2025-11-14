use tonic::transport::Channel;
use uuid::Uuid;
use tracing::{debug, error};

use crate::error::{DorisError, Result};
use crate::query::QueryResult;
use crate::mysql::packet::{ColumnDefinition, ResultRow};
use crate::mysql::ColumnType;
// Protobuf imports disabled when SKIP_PROTO is set
// use super::pb::{p_backend_service_client::PBackendServiceClient, *};

pub struct BackendClient {
    host: String,
    port: u16,
    grpc_port: u16,
    // Proto client stubbed out when SKIP_PROTO is set
    client: Option<()>,  // Would be PBackendServiceClient<Channel> with proto
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
        // Stubbed out when SKIP_PROTO is set
        debug!("BE connection stubbed (SKIP_PROTO mode)");
        Err(DorisError::BackendCommunication("Proto support not available in SKIP_PROTO mode".to_string()))
    }

    pub async fn execute_query(&mut self, _query_id: Uuid, _query: &str) -> Result<QueryResult> {
        // Stubbed out when SKIP_PROTO is set
        Err(DorisError::BackendCommunication("Proto support not available in SKIP_PROTO mode".to_string()))
    }

    async fn fetch_data(&mut self, _query_id: Uuid) -> Result<QueryResult> {
        // Stubbed out when SKIP_PROTO is set
        Err(DorisError::BackendCommunication("Proto support not available in SKIP_PROTO mode".to_string()))
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

    pub async fn cancel_query(&mut self, _query_id: Uuid) -> Result<()> {
        // Stubbed out when SKIP_PROTO is set
        Ok(())
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
