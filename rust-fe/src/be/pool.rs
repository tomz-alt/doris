use std::sync::Arc;
use dashmap::DashMap;
use uuid::Uuid;
use tracing::{debug, info};

use crate::config::BackendNode;
use crate::error::Result;
use crate::query::QueryResult;
use super::client::BackendClient;

pub struct BackendClientPool {
    clients: DashMap<String, Arc<tokio::sync::Mutex<BackendClient>>>,
    backend_nodes: Vec<BackendNode>,
    current_index: Arc<std::sync::atomic::AtomicUsize>,
}

impl BackendClientPool {
    pub fn new(backend_nodes: Vec<BackendNode>) -> Self {
        let pool = Self {
            clients: DashMap::new(),
            backend_nodes: backend_nodes.clone(),
            current_index: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };

        // Initialize clients
        for node in &backend_nodes {
            let key = format!("{}:{}", node.host, node.grpc_port);
            let client = BackendClient::new(
                node.host.clone(),
                node.port,
                node.grpc_port,
            );
            pool.clients.insert(key, Arc::new(tokio::sync::Mutex::new(client)));
        }

        info!("Backend client pool initialized with {} nodes", backend_nodes.len());

        pool
    }

    pub async fn execute_query(&self, query_id: Uuid, query: &str) -> Result<QueryResult> {
        // Round-robin selection
        let backend = self.select_backend();

        debug!("Executing query {} on backend: {}", query_id, backend.addr());

        let mut client = backend.lock().await;
        client.execute_query(query_id, query).await
    }

    pub async fn cancel_query(&self, query_id: Uuid) -> Result<()> {
        // Cancel on all backends (broadcast)
        let mut handles = vec![];

        for entry in self.clients.iter() {
            let client = entry.value().clone();
            let qid = query_id;

            let handle = tokio::spawn(async move {
                let mut client = client.lock().await;
                let _ = client.cancel_query(qid).await;
            });

            handles.push(handle);
        }

        // Wait for all cancellations
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }

    fn select_backend(&self) -> Arc<tokio::sync::Mutex<BackendClient>> {
        if self.backend_nodes.is_empty() {
            panic!("No backend nodes configured");
        }

        let index = self.current_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let node = &self.backend_nodes[index % self.backend_nodes.len()];
        let key = format!("{}:{}", node.host, node.grpc_port);

        self.clients.get(&key)
            .expect("Backend client should exist")
            .value()
            .clone()
    }

    pub fn backend_count(&self) -> usize {
        self.backend_nodes.len()
    }
}
