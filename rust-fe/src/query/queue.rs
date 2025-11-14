use std::collections::VecDeque;
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::sync::Semaphore;
use uuid::Uuid;
use tracing::{debug, info};

use crate::error::{DorisError, Result};

#[derive(Debug, Clone)]
pub struct QueuedQuery {
    pub query_id: Uuid,
    pub query: String,
    pub database: Option<String>,
}

pub struct QueryQueue {
    queue: Arc<Mutex<VecDeque<QueuedQuery>>>,
    max_queue_size: usize,
    execution_slots: Arc<Semaphore>,
    max_concurrent: usize,
}

impl QueryQueue {
    pub fn new(max_queue_size: usize, max_concurrent: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            max_queue_size,
            execution_slots: Arc::new(Semaphore::new(max_concurrent)),
            max_concurrent,
        }
    }

    pub fn enqueue(&self, query: QueuedQuery) -> Result<()> {
        let mut queue = self.queue.lock();

        if queue.len() >= self.max_queue_size {
            return Err(DorisError::QueueFull);
        }

        queue.push_back(query.clone());
        info!("Query queued: {} (queue size: {})", query.query_id, queue.len());

        Ok(())
    }

    pub fn dequeue(&self) -> Option<QueuedQuery> {
        let mut queue = self.queue.lock();
        let query = queue.pop_front();

        if let Some(ref q) = query {
            debug!("Query dequeued: {} (remaining: {})", q.query_id, queue.len());
        }

        query
    }

    pub async fn acquire_slot(&self) -> tokio::sync::SemaphorePermit<'_> {
        self.execution_slots.acquire().await
            .expect("Semaphore should not be closed")
    }

    pub fn queue_size(&self) -> usize {
        self.queue.lock().len()
    }

    pub fn available_slots(&self) -> usize {
        self.execution_slots.available_permits()
    }

    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }
}
