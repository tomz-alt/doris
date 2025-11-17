/// Mock Backend Server for Integration Testing
/// Implements a simple gRPC server that mimics Doris Backend behavior
///
/// This server:
/// 1. Accepts fragment execution requests
/// 2. Returns mock query results
/// 3. Handles data fetching and cancellation
///
/// Usage: cargo run --example mock_be_server
/// Then run integration tests against localhost:9070

use tonic::{transport::Server, Request, Response, Status};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Import the generated protobuf code
mod pb {
    // Core Doris BE service definitions.
    tonic::include_proto!("doris");
    // When using the real Doris internal_service.proto (feature=real_be_proto),
    // also include storage-format types referenced from the main protos.
    #[cfg(feature = "real_be_proto")]
    tonic::include_proto!("segment_v2");
}

use pb::p_backend_service_server::{PBackendService, PBackendServiceServer};
use pb::{
    PExecPlanFragmentRequest, PExecPlanFragmentResult,
    PFetchDataRequest, PFetchDataResult,
    PCancelPlanFragmentRequest, PCancelPlanFragmentResult,
};

#[derive(Debug)]
struct QueryState {
    query_id: Vec<u8>,
    fragment_id: i64,
    query_sql: String,
    completed: bool,
}

pub struct MockBackendService {
    queries: Arc<Mutex<HashMap<String, QueryState>>>,
}

impl MockBackendService {
    fn new() -> Self {
        Self {
            queries: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn query_key(query_id: &[u8], fragment_id: i64) -> String {
        format!("{:?}_{}", query_id, fragment_id)
    }
}

#[tonic::async_trait]
impl PBackendService for MockBackendService {
    async fn exec_plan_fragment(
        &self,
        request: Request<PExecPlanFragmentRequest>,
    ) -> Result<Response<PExecPlanFragmentResult>, Status> {
        let req = request.into_inner();

        println!("📥 Received ExecPlanFragment request:");
        println!("  Query ID: {:?}", req.query_id);
        println!("  Fragment ID: {}", req.fragment_instance_id);
        println!("  Query: {}", req.query);

        // Store query state
        let key = Self::query_key(&req.query_id, req.fragment_instance_id);
        let mut queries = self.queries.lock().unwrap();
        queries.insert(key.clone(), QueryState {
            query_id: req.query_id.clone(),
            fragment_id: req.fragment_instance_id,
            query_sql: req.query.clone(),
            completed: true,
        });

        println!("✓ Fragment execution started (mock)");

        let response = PExecPlanFragmentResult {
            status_code: 0,
            message: "Success".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn fetch_data(
        &self,
        request: Request<PFetchDataRequest>,
    ) -> Result<Response<PFetchDataResult>, Status> {
        let req = request.into_inner();

        println!("📥 Received FetchData request:");
        println!("  Query ID: {:?}", req.query_id);
        println!("  Fragment ID: {}", req.fragment_instance_id);

        let key = Self::query_key(&req.query_id, req.fragment_instance_id);
        let queries = self.queries.lock().unwrap();

        if let Some(query_state) = queries.get(&key) {
            println!("✓ Found query state, returning mock data");

            // Generate mock result data
            let mock_data = if query_state.query_sql.contains("COUNT") {
                // Return a count result
                b"42".to_vec()
            } else if query_state.query_sql.contains("GROUP BY") {
                // Return grouped data
                b"A,10\nB,20\nC,30".to_vec()
            } else {
                // Return generic rows
                b"row1,col1,col2\nrow2,col1,col2\nrow3,col1,col2".to_vec()
            };

            let response = PFetchDataResult {
                status_code: 0,
                data: mock_data,
                eos: true, // End of stream
            };

            Ok(Response::new(response))
        } else {
            println!("✗ Query not found");
            Err(Status::not_found("Query not found"))
        }
    }

    async fn cancel_plan_fragment(
        &self,
        request: Request<PCancelPlanFragmentRequest>,
    ) -> Result<Response<PCancelPlanFragmentResult>, Status> {
        let req = request.into_inner();

        println!("📥 Received CancelPlanFragment request:");
        println!("  Query ID: {:?}", req.query_id);
        println!("  Fragment ID: {}", req.fragment_instance_id);

        let key = Self::query_key(&req.query_id, req.fragment_instance_id);
        let mut queries = self.queries.lock().unwrap();

        if queries.remove(&key).is_some() {
            println!("✓ Fragment cancelled");
        } else {
            println!("⚠ Fragment not found (may already be completed)");
        }

        let response = PCancelPlanFragmentResult {
            status_code: 0,
        };

        Ok(Response::new(response))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:9070".parse()?;
    let backend_service = MockBackendService::new();

    println!("🚀 Mock Doris Backend Server starting...");
    println!("📡 Listening on: {}", addr);
    println!("✓ Ready to accept gRPC connections\n");
    println!("Press Ctrl+C to stop the server\n");

    Server::builder()
        .add_service(PBackendServiceServer::new(backend_service))
        .serve(addr)
        .await?;

    Ok(())
}
