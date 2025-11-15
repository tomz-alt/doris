use std::sync::Arc;
use dashmap::DashMap;
use uuid::Uuid;
use tracing::{debug, info};

use crate::config::BackendNode;
use crate::error::Result;
use crate::query::QueryResult;
use super::client::BackendClient;

#[derive(Debug)]
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

        debug!("Executing query {} via gRPC", query_id);

        let mut client = backend.lock().await;

        // Auto-connect if not already connected
        if !client.is_connected() {
            debug!("Auto-connecting to BE");
            client.connect().await?;
        }

        // Execute fragment (use fragment_id = 0 for simple queries)
        let result = client.execute_fragment(query_id, 0, query).await?;

        if result.status_code != 0 {
            return Err(crate::error::DorisError::BackendCommunication(
                format!("BE returned error: {}", result.message)
            ));
        }

        // Fetch data from BE
        let fetch_result = client.fetch_data(query_id, 0).await?;

        // Parse the result data
        if fetch_result.data.is_empty() {
            return Ok(QueryResult::empty());
        }

        // For now, return a simple result
        // TODO: Parse Arrow format from BE
        use crate::mysql::packet::{ColumnDefinition, ResultRow};
        use crate::mysql::ColumnType;

        let columns = vec![
            ColumnDefinition::new("result".to_string(), ColumnType::VarString),
        ];

        let rows = vec![
            ResultRow::new(vec![Some(format!("Received {} bytes from BE", fetch_result.data.len()))]),
        ];

        Ok(QueryResult::new_select(columns, rows))
    }

    pub async fn cancel_query(&self, query_id: Uuid) -> Result<()> {
        // Cancel on all backends (broadcast)
        let mut handles = vec![];

        for entry in self.clients.iter() {
            let client = entry.value().clone();
            let qid = query_id;

            let handle = tokio::spawn(async move {
                let mut client = client.lock().await;
                // Cancel fragment 0
                let _ = client.cancel_fragment(qid, 0).await;
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
