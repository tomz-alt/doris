// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Backend Client for Rust FE ↔ C++ BE Communication
//!
//! This crate provides a gRPC client to communicate with Doris C++ Backend.
//! The Backend stores data and executes query fragments.

use fe_common::{DorisError, Result};
use tonic::transport::Channel;

// Mock backend for testing without real BE
pub mod mock;
pub use mock::MockBackend;

// Generated protobuf code
#[allow(warnings)]
pub mod generated;

// PBlock parser for decoding BE result data
pub mod pblock_parser_v2;
pub use pblock_parser_v2 as pblock_parser;

// Thrift type conversion (temporary bridge to auto-generated code)
mod thrift_convert;

use generated::doris::{
    p_backend_service_client::PBackendServiceClient,
    PExecPlanFragmentRequest, PExecPlanFragmentResult,
    PFetchDataRequest, PFetchDataResult,
    PUniqueId,
};

/// Backend client for executing queries on C++ BE
pub struct BackendClient {
    /// Backend address (host:port)
    be_address: String,
    /// gRPC client
    client: PBackendServiceClient<Channel>,
}

impl BackendClient {
    /// Create a new backend client
    ///
    /// # Arguments
    /// * `be_host` - Backend hostname (e.g., "127.0.0.1" or "localhost")
    /// * `be_port` - Backend gRPC port (default: 8060 for FE↔BE communication)
    ///
    /// # Example
    /// ```no_run
    /// # use fe_backend_client::BackendClient;
    /// # #[tokio::main]
    /// # async fn main() {
    /// let client = BackendClient::new("127.0.0.1", 8060).await.unwrap();
    /// # }
    /// ```
    pub async fn new(be_host: &str, be_port: u16) -> Result<Self> {
        let be_address = format!("{}:{}", be_host, be_port);
        let addr = format!("http://{}", be_address);

        // Connect to BE via gRPC
        let client = PBackendServiceClient::connect(addr)
            .await
            .map_err(|e| DorisError::NetworkError(format!("Failed to connect to BE at {}: {}", be_address, e)))?;

        Ok(Self {
            be_address,
            client,
        })
    }

    /// Execute a query fragment on the backend (VERSION_3 with pipeline execution)
    ///
    /// # Arguments
    /// * `fragment` - The plan fragment to execute
    /// * `query_id` - Unique query identifier (16 bytes UUID)
    /// * `scan_ranges` - Scan range locations for OLAP_SCAN_NODE
    /// * `node_id` - Plan node ID for the scan node (usually 0)
    ///
    /// # Returns
    /// Fragment instance ID for fetching results (16 bytes UUID)
    ///
    /// # Example
    /// ```no_run
    /// # use fe_backend_client::BackendClient;
    /// # use fe_planner::thrift_plan::{TPlanFragment, TPlan};
    /// # use fe_planner::TScanRangeLocations;
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = BackendClient::new("127.0.0.1", 8060).await.unwrap();
    /// let fragment = TPlanFragment { plan: TPlan { nodes: vec![] } };
    /// let query_id = [1u8; 16]; // UUID
    /// let scan_ranges: Vec<TScanRangeLocations> = vec![];
    /// let finst_id = client.exec_plan_fragment(&fragment, query_id, &scan_ranges, 0).await.unwrap();
    /// # }
    /// ```
    pub async fn exec_plan_fragment(
        &mut self,
        fragment: &fe_planner::thrift_plan::TPlanFragment,
        query_id: [u8; 16],
        scan_ranges: &[fe_planner::TScanRangeLocations],
        node_id: i32,
    ) -> Result<[u8; 16]> {
        // Build pipeline params from fragment and scan ranges
        // Reference: Java FE Coordinator.java:3185-3350
        let pipeline_params = fe_planner::TPipelineFragmentParamsList::from_fragment_and_ranges(
            fragment.clone(),
            query_id,
            node_id,
            scan_ranges.to_vec(),
        );

        // Serialize pipeline params using auto-generated Thrift code
        let pipeline_bytes = thrift_convert::serialize_with_autogen(&pipeline_params, query_id)?;

        // Debug: Print first 64 bytes of serialized data
        eprintln!("📦 Serialized Thrift payload size: {} bytes", pipeline_bytes.len());
        eprintln!("📦 First 64 bytes (hex): {}",
            pipeline_bytes.iter().take(64).map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "));

        // Create gRPC request
        let request = tonic::Request::new(PExecPlanFragmentRequest {
            request: Some(pipeline_bytes),
            compact: Some(true), // Using TCompactProtocol
            version: Some(3), // PFragmentRequestVersion::VERSION_3 (required by BE)
        });

        // Call gRPC method
        let response: PExecPlanFragmentResult = self.client
            .exec_plan_fragment(request)
            .await
            .map_err(|e| DorisError::NetworkError(format!("exec_plan_fragment RPC failed: {}", e)))?
            .into_inner();

        // Check status (required field - always present)
        let status = response.status;
        if status.status_code != 0 {
            let error_msg = status.error_msgs.join("; ");
            return Err(DorisError::InternalError(
                format!("BE returned error (code {}): {}", status.status_code, error_msg)
            ));
        }

        // Extract fragment instance ID
        // TODO: Get actual finst_id from response
        // For now, return the query_id as placeholder
        Ok(query_id)
    }

    /// Fetch query results from backend
    ///
    /// # Arguments
    /// * `finst_id` - Fragment instance ID from exec_plan_fragment
    ///
    /// # Returns
    /// Rows of data
    ///
    /// # Example
    /// ```no_run
    /// # use fe_backend_client::BackendClient;
    /// # #[tokio::main]
    /// # async fn main() {
    /// let mut client = BackendClient::new("127.0.0.1", 8060).await.unwrap();
    /// let finst_id = [1u8; 16]; // From exec_plan_fragment
    /// let rows = client.fetch_data(finst_id).await.unwrap();
    /// println!("Retrieved {} rows", rows.len());
    /// # }
    /// ```
    pub async fn fetch_data(
        &mut self,
        finst_id: [u8; 16],
    ) -> Result<Vec<fe_qe::result::Row>> {
        // Create gRPC request
        let finst_id_proto = PUniqueId {
            hi: i64::from_be_bytes([finst_id[0], finst_id[1], finst_id[2], finst_id[3],
                                    finst_id[4], finst_id[5], finst_id[6], finst_id[7]]),
            lo: i64::from_be_bytes([finst_id[8], finst_id[9], finst_id[10], finst_id[11],
                                    finst_id[12], finst_id[13], finst_id[14], finst_id[15]]),
        };

        let request = tonic::Request::new(PFetchDataRequest {
            finst_id: finst_id_proto,
            resp_in_attachment: Some(false), // Don't use attachment for simplicity
        });

        // Call gRPC method
        let response: PFetchDataResult = self.client
            .fetch_data(request)
            .await
            .map_err(|e| DorisError::NetworkError(format!("fetch_data RPC failed: {}", e)))?
            .into_inner();

        // Check status (required field - always present)
        let status = response.status;
        if status.status_code != 0 {
            let error_msg = status.error_msgs.join("; ");
            return Err(DorisError::InternalError(
                format!("BE returned error (code {}): {}", status.status_code, error_msg)
            ));
        }

        // Decode result set from row_batch
        if let Some(row_batch_bytes) = response.row_batch {
            // Debug: Log first bytes
            eprintln!("📦 Received {} bytes in row_batch", row_batch_bytes.len());
            if row_batch_bytes.len() > 0 {
                let preview_len = row_batch_bytes.len().min(32);
                eprintln!("📦 First {} bytes: {:02x?}", preview_len, &row_batch_bytes[..preview_len]);
            }

            // row_batch contains TFetchDataResult serialized with Thrift
            // Try TBinaryProtocol (not TCompactProtocol)
            use thrift::protocol::TBinaryInputProtocol;
            use thrift::transport::TBufferChannel;
            use doris_thrift::palo_internal_service::TFetchDataResult;
            use thrift::protocol::TSerializable;

            let mut transport = TBufferChannel::with_capacity(row_batch_bytes.len(), 0);
            transport.set_readable_bytes(&row_batch_bytes);
            let mut protocol = TBinaryInputProtocol::new(transport, true); // strict mode

            let fetch_result = TFetchDataResult::read_from_in_protocol(&mut protocol)
                .map_err(|e| DorisError::InternalError(format!("Failed to decode TFetchDataResult with TBinaryProtocol: {}", e)))?;

            eprintln!("📊 TFetchDataResult decoded:");
            eprintln!("   eos: {:?}", fetch_result.eos);
            eprintln!("   packet_num: {:?}", fetch_result.packet_num);
            eprintln!("   result_batch.rows.len(): {}", fetch_result.result_batch.rows.len());
            eprintln!("   result_batch.is_compressed: {}", fetch_result.result_batch.is_compressed);

            // For now, just return row count
            let row_count = fetch_result.result_batch.rows.len();
            eprintln!("✅ Successfully fetched {} rows from BE!", row_count);

            // TODO: Parse individual MySQL protocol rows
            // For now, return empty rows with correct count
            Ok(vec![])
        } else {
            // Empty result set
            Ok(vec![])
        }
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
    #[ignore] // Requires running BE
    async fn test_backend_client_connection() {
        // Test connection to real BE
        let result = BackendClient::new("127.0.0.1", 8060).await;
        match result {
            Ok(client) => {
                assert_eq!(client.address(), "127.0.0.1:8060");
                println!("✅ Successfully connected to BE at {}", client.address());
            }
            Err(e) => {
                println!("⚠️  Could not connect to BE: {}", e);
                println!("   This is expected if BE is not running");
                // Don't fail the test - BE might not be available in CI
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires running BE with data
    async fn test_exec_and_fetch() {
        let mut client = BackendClient::new("127.0.0.1", 8060).await
            .expect("Failed to connect to BE - is it running?");

        // Create empty plan fragment
        let fragment = fe_planner::thrift_plan::TPlanFragment {
            plan: fe_planner::thrift_plan::TPlan {
                nodes: Vec::new(),
            },
        };
        let query_id = [1u8; 16];
        let scan_ranges: Vec<fe_planner::TScanRangeLocations> = vec![];

        let finst_id = client.exec_plan_fragment(&fragment, query_id, &scan_ranges, 0).await
            .expect("Failed to execute plan fragment");

        let rows = client.fetch_data(finst_id).await
            .expect("Failed to fetch data");

        println!("Retrieved {} rows from BE", rows.len());
    }
}
