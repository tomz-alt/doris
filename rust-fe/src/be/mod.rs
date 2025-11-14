mod client;
mod pool;

pub use client::BackendClient;
pub use pool::BackendClientPool;

// Include generated protobuf code
#[allow(unused, clippy::all)]
pub mod pb {
    tonic::include_proto!("doris");
}

// Re-export commonly used types
pub use pb::*;
