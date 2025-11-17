mod executor;
mod queue;

pub use executor::{QueryExecutor, ParsedInsert};
pub use queue::{QueryQueue, QueuedQuery};

use crate::mysql::packet::{ColumnDefinition, ResultRow};

/// Which wire protocol a session is using.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolType {
    Mysql,
    Postgres,
    Http,
}

/// SQL mode / compatibility flags for the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlMode {
    Default,
}

/// Protocol-agnostic session context used by all frontends.
///
/// This allows MySQL, pgwire and other entrypoints to share the same
/// core execution path while keeping protocol-specific state at the
/// edges.
#[derive(Debug, Clone)]
pub struct SessionCtx {
    pub database: Option<String>,
    pub user: String,
    pub sql_mode: SqlMode,
    pub protocol: ProtocolType,
}

impl SessionCtx {
    pub fn new() -> Self {
        Self {
            database: None,
            // Default to root/MySQL until the frontend sets real values.
            user: "root".to_string(),
            sql_mode: SqlMode::Default,
            protocol: ProtocolType::Mysql,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ctx_defaults_are_sane() {
        let session = SessionCtx::new();
        assert_eq!(session.database, None);
        assert_eq!(session.user, "root");
        assert_eq!(session.sql_mode, SqlMode::Default);
        assert_eq!(session.protocol, ProtocolType::Mysql);
    }
}

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
