// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Backend Client for Rust FE ↔ C++ BE Communication
//!
//! This crate provides a gRPC client to communicate with Doris C++ Backend.
//! The Backend stores data and executes query fragments.

use fe_common::Result;

// Mock backend for testing without protoc
pub mod mock;
pub use mock::MockBackend;

// Generated protobuf code will go here
// pub mod generated;
// use generated::internal_service::*;

/// Backend client for executing queries on C++ BE
pub struct BackendClient {
    /// Backend address (host:port)
    be_address: String,
    // TODO: Add gRPC client after protobuf generation
    // client: PBackendServiceClient<Channel>,
}

impl BackendClient {
    /// Create a new backend client
    ///
    /// # Arguments
    /// * `be_host` - Backend hostname (e.g., "127.0.0.1")
    /// * `be_port` - Backend gRPC port (default: 9060)
    pub async fn new(be_host: &str, be_port: u16) -> Result<Self> {
        let be_address = format!("{}:{}", be_host, be_port);

        // TODO: Connect to BE via gRPC
        // let addr = format!("http://{}", be_address);
        // let client = PBackendServiceClient::connect(addr).await?;

        Ok(Self {
            be_address,
            // client,
        })
    }

    /// Execute a query fragment on the backend
    ///
    /// # Arguments
    /// * `fragment` - The plan fragment to execute
    /// * `query_id` - Unique query identifier
    ///
    /// # Returns
    /// Fragment instance ID for fetching results
    pub async fn exec_plan_fragment(
        &mut self,
        _fragment: &fe_planner::thrift_plan::TPlanFragment,
        _query_id: [u8; 16],
    ) -> Result<[u8; 16]> {
        // TODO: Implement gRPC call to BE
        // 1. Serialize fragment to Thrift bytes
        // 2. Create PExecPlanFragmentRequest
        // 3. Call client.exec_plan_fragment()
        // 4. Return fragment instance ID

        Err(fe_common::DorisError::InternalError(
            "BE client not yet implemented - need protobuf generation".to_string()
        ))
    }

    /// Fetch query results from backend
    ///
    /// # Arguments
    /// * `finst_id` - Fragment instance ID from exec_plan_fragment
    ///
    /// # Returns
    /// Rows of data
    pub async fn fetch_data(
        &mut self,
        _finst_id: [u8; 16],
    ) -> Result<Vec<fe_qe::result::Row>> {
        // TODO: Implement fetch_data RPC
        // 1. Create PFetchDataRequest with finst_id
        // 2. Loop until eos (end of stream)
        // 3. Deserialize row batches
        // 4. Return all rows

        Err(fe_common::DorisError::InternalError(
            "BE client not yet implemented".to_string()
        ))
    }

    /// Get backend address
    pub fn address(&self) -> &str {
        &self.be_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backend_client_creation() {
        let client = BackendClient::new("127.0.0.1", 9060).await;
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.address(), "127.0.0.1:9060");
    }
}
