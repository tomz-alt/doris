use std::sync::Arc;
use uuid::Uuid;
use tracing::{debug, info, error};

use crate::error::{DorisError, Result};
use crate::be::BackendClientPool;
use crate::mysql::packet::{ColumnDefinition, ColumnType, ResultRow};
use super::{QueryQueue, QueryResult, QueuedQuery};

pub struct QueryExecutor {
    queue: Arc<QueryQueue>,
}

impl QueryExecutor {
    pub fn new(max_queue_size: usize, max_concurrent: usize) -> Self {
        Self {
            queue: Arc::new(QueryQueue::new(max_queue_size, max_concurrent)),
        }
    }

    pub async fn queue_query(
        &self,
        query_id: Uuid,
        query: String,
        database: Option<String>,
    ) -> Result<()> {
        let queued_query = QueuedQuery {
            query_id,
            query,
            database,
        };

        self.queue.enqueue(queued_query)
    }

    pub async fn execute_query(
        &self,
        query_id: Uuid,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        // Acquire execution slot (implements queuing)
        let _permit = self.queue.acquire_slot().await;

        info!("Executing query: {} (available slots: {})",
              query_id, self.queue.available_slots());

        // Dequeue the query
        let queued_query = self.queue.dequeue()
            .ok_or_else(|| DorisError::QueryExecution("Query not found in queue".to_string()))?;

        // Execute the query
        self.execute_internal(queued_query, be_client_pool).await
    }

    async fn execute_internal(
        &self,
        query: QueuedQuery,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        debug!("Executing query: {}", query.query);

        let query_lower = query.query.trim().to_lowercase();

        // Parse query type
        if query_lower.starts_with("select") {
            self.execute_select(query, be_client_pool).await
        } else if query_lower.starts_with("insert")
            || query_lower.starts_with("update")
            || query_lower.starts_with("delete")
        {
            self.execute_dml(query, be_client_pool).await
        } else if query_lower.starts_with("create")
            || query_lower.starts_with("drop")
            || query_lower.starts_with("alter")
        {
            self.execute_ddl(query, be_client_pool).await
        } else {
            // Unknown query type, return empty result
            Ok(QueryResult::empty())
        }
    }

    async fn execute_select(
        &self,
        query: QueuedQuery,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        info!("Executing SELECT query: {}", query.query_id);

        // For PoC: Send query to BE via gRPC
        match be_client_pool.execute_query(query.query_id, &query.query).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("BE execution failed: {}, returning mock data", e);
                // Return mock data for PoC
                self.mock_select_result(&query.query)
            }
        }
    }

    async fn execute_dml(
        &self,
        query: QueuedQuery,
        be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        info!("Executing DML query: {}", query.query_id);

        // For PoC: Send query to BE via gRPC
        match be_client_pool.execute_query(query.query_id, &query.query).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!("BE execution failed: {}, returning mock affected rows", e);
                // Return mock result for PoC
                Ok(QueryResult::new_dml(1))
            }
        }
    }

    async fn execute_ddl(
        &self,
        query: QueuedQuery,
        _be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        info!("Executing DDL query: {}", query.query_id);

        // For PoC: Just return success
        Ok(QueryResult::new_dml(0))
    }

    fn mock_select_result(&self, query: &str) -> Result<QueryResult> {
        // Simple mock result for demonstration
        let columns = vec![
            ColumnDefinition::new("id".to_string(), ColumnType::Long),
            ColumnDefinition::new("name".to_string(), ColumnType::VarString),
            ColumnDefinition::new("value".to_string(), ColumnType::Double),
        ];

        let rows = vec![
            ResultRow::new(vec![
                Some("1".to_string()),
                Some("test_row_1".to_string()),
                Some("123.45".to_string()),
            ]),
            ResultRow::new(vec![
                Some("2".to_string()),
                Some("test_row_2".to_string()),
                Some("678.90".to_string()),
            ]),
        ];

        Ok(QueryResult::new_select(columns, rows))
    }

    pub fn queue_stats(&self) -> (usize, usize, usize) {
        (
            self.queue.queue_size(),
            self.queue.available_slots(),
            self.queue.max_concurrent(),
        )
    }
}
