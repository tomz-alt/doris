pub mod client;
pub mod pool;
pub mod thrift_pipeline;
pub mod load;

pub use client::BackendClient;
pub use pool::BackendClientPool;

// Include generated protobuf code for the BE APIs.
// The `doris` module contains PBackendService and related messages defined in
// internal_service.proto / backend_service.proto.
// The `segment_v2` module contains storage-format types referenced from data.proto.
#[allow(unused, clippy::all)]
pub mod doris {
    tonic::include_proto!("doris");
}

#[allow(unused, clippy::all)]
pub mod segment_v2 {
    tonic::include_proto!("segment_v2");
}

// Re-export commonly used types so callers can use crate::be::PExecPlanFragmentRequest, etc.
pub use doris::*;
pub use segment_v2::*;
