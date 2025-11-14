mod executor;
mod queue;

pub use executor::QueryExecutor;
pub use queue::{QueryQueue, QueuedQuery};

use crate::mysql::packet::{ColumnDefinition, ResultRow};

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub is_dml: bool,
    pub affected_rows: u64,
    pub columns: Vec<ColumnDefinition>,
    pub rows: Vec<ResultRow>,
}

impl QueryResult {
    pub fn new_select(columns: Vec<ColumnDefinition>, rows: Vec<ResultRow>) -> Self {
        Self {
            is_dml: false,
            affected_rows: 0,
            columns,
            rows,
        }
    }

    pub fn new_dml(affected_rows: u64) -> Self {
        Self {
            is_dml: true,
            affected_rows,
            columns: vec![],
            rows: vec![],
        }
    }

    pub fn empty() -> Self {
        Self {
            is_dml: false,
            affected_rows: 0,
            columns: vec![],
            rows: vec![],
        }
    }
}
