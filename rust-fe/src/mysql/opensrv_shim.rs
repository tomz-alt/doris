use std::io;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::AsyncWrite;
use tracing::{error, info};

use opensrv_mysql::{
    AsyncMysqlShim,
    Column,
    ColumnFlags,
    ErrorKind,
    OkResponse,
    ParamParser,
    QueryResultWriter,
    StatementMetaWriter,
};

use crate::be::BackendClientPool;
use crate::error::DorisError;
use crate::metadata::catalog::catalog;
use crate::query::{QueryExecutor, SessionCtx, ProtocolType};

/// opensrv shim that adapts the MySQL protocol to the core `execute_sql` API.
///
/// One instance is created per connection and holds its own `SessionCtx`.
pub struct DorisShim {
    pub query_executor: Arc<QueryExecutor>,
    pub be_client_pool: Arc<BackendClientPool>,
    pub session: SessionCtx,
}

impl DorisShim {
    pub fn new(
        query_executor: Arc<QueryExecutor>,
        be_client_pool: Arc<BackendClientPool>,
    ) -> Self {
        let mut session = SessionCtx::new();
        session.protocol = ProtocolType::Mysql;

        Self {
            query_executor,
            be_client_pool,
            session,
        }
    }
}

#[async_trait]
impl<W> AsyncMysqlShim<W> for DorisShim
where
    W: AsyncWrite + Send + Unpin,
{
    type Error = io::Error;

    async fn on_prepare<'a>(
        &'a mut self,
        _query: &'a str,
        info: StatementMetaWriter<'a, W>,
    ) -> Result<(), Self::Error> {
        // Prepared statements are not supported yet.
        info.error(
            ErrorKind::ER_UNKNOWN_COM_ERROR,
            b"Prepared statements not supported",
        )
        .await
    }

    async fn on_execute<'a>(
        &'a mut self,
        _id: u32,
        _params: ParamParser<'a>,
        results: QueryResultWriter<'a, W>,
    ) -> Result<(), Self::Error> {
        // Prepared statements are not supported yet.
        results
            .error(
                ErrorKind::ER_UNKNOWN_COM_ERROR,
                b"Prepared statements not supported",
            )
            .await
    }

    async fn on_close<'a>(&'a mut self, _stmt: u32)
    where
        W: 'async_trait,
    {
        // Nothing to clean up yet – statements are not tracked in the shim.
    }

    async fn on_query<'a>(
        &'a mut self,
        query: &'a str,
        results: QueryResultWriter<'a, W>,
    ) -> Result<(), Self::Error> {
        let sql = query.trim();
        let sql_lower = sql.to_lowercase();

        // Handle simple protocol / compatibility commands here so we don't
        // burden the core engine with client probes.

        // USE db; – update session database and acknowledge.
        if sql_lower.starts_with("use ") {
            let db = sql_lower
                .strip_prefix("use ")
                .unwrap_or("")
                .trim_end_matches(';')
                .trim();
            if db.is_empty() {
                return results
                    .error(ErrorKind::ER_BAD_DB_ERROR, b"Unknown database ''")
                    .await
                    .map_err(Into::into);
            }

            let catalog = catalog();
            if !catalog.database_exists(db) {
                let msg = format!("Unknown database '{}'", db);
                return results
                    .error(ErrorKind::ER_BAD_DB_ERROR, msg.as_bytes())
                    .await
                    .map_err(Into::into);
            }

            info!("opensrv: changing database to '{}'", db);
            self.session.database = Some(db.to_string());

            return results.completed(OkResponse::default()).await.map_err(Into::into);
        }

        // SET statements – accept without touching core for now.
        if sql_lower.starts_with("set ") {
            return results.completed(OkResponse::default()).await.map_err(Into::into);
        }

        // SELECT DATABASE() – return current database name.
        if sql_lower == "select database()" {
            let cols = vec![Column {
                table: "".to_string(),
                column: "DATABASE()".to_string(),
                coltype: opensrv_mysql::ColumnType::MYSQL_TYPE_VAR_STRING,
                colflags: ColumnFlags::empty(),
            }];

            let mut row_writer = results.start(&cols).await?;
            let db = self.session.database.as_deref().unwrap_or("NULL");
            row_writer.write_row(std::iter::once(db)).await?;
            row_writer.finish().await?;
            return Ok(());
        }

        // Delegate all other statements to the core executor.
        let mut session = &mut self.session;
        let exec_result = self
            .query_executor
            .execute_sql(&mut session, sql, &self.be_client_pool)
            .await;

        match exec_result {
            Ok(result) => {
                if result.is_dml {
                    let mut ok = OkResponse::default();
                    ok.affected_rows = result.affected_rows;
                    results.completed(ok).await.map_err(Into::into)
                } else {
                    // Map core `QueryResult` into opensrv columns + rows.
                    let cols: Vec<Column> = result
                        .columns
                        .iter()
                        .map(|col| Column {
                            table: col.table.clone(),
                            column: col.name.clone(),
                            coltype: map_column_type(col.column_type),
                            colflags: ColumnFlags::empty(),
                        })
                        .collect();

                    let mut row_writer = results.start(&cols).await?;
                    for row in result.rows {
                        let values: Vec<&str> = row
                            .values
                            .iter()
                            .map(|v| v.as_deref().unwrap_or("NULL"))
                            .collect();
                        row_writer.write_row(values).await?;
                    }
                    row_writer.finish().await?;
                    Ok(())
                }
            }
            Err(e) => {
                error!("opensrv query execution failed: {}", e);
                let (kind, msg) = map_doris_error(&e);
                results.error(kind, msg.as_bytes()).await.map_err(Into::into)
            }
        }
    }

    async fn on_init<'a>(
        &'a mut self,
        database: &'a str,
        w: opensrv_mysql::InitWriter<'a, W>,
    ) -> Result<(), Self::Error> {
        let catalog = catalog();
        if !catalog.database_exists(database) {
            let msg = format!("Unknown database '{}'", database);
            return w
                .error(ErrorKind::ER_BAD_DB_ERROR, msg.as_bytes())
                .await;
        }

        info!("opensrv: INIT_DB to '{}'", database);
        self.session.database = Some(database.to_string());
        w.ok().await
    }
}

fn map_column_type(doris_type: u8) -> opensrv_mysql::ColumnType {
    use opensrv_mysql::ColumnType as SrvColumnType;

    match doris_type {
        0x00 => SrvColumnType::MYSQL_TYPE_DECIMAL,
        0x01 => SrvColumnType::MYSQL_TYPE_TINY,
        0x02 => SrvColumnType::MYSQL_TYPE_SHORT,
        0x03 => SrvColumnType::MYSQL_TYPE_LONG,
        0x04 => SrvColumnType::MYSQL_TYPE_FLOAT,
        0x05 => SrvColumnType::MYSQL_TYPE_DOUBLE,
        0x08 => SrvColumnType::MYSQL_TYPE_LONGLONG,
        0x0f => SrvColumnType::MYSQL_TYPE_VARCHAR,
        0xfd => SrvColumnType::MYSQL_TYPE_VAR_STRING,
        0xfe => SrvColumnType::MYSQL_TYPE_STRING,
        _ => SrvColumnType::MYSQL_TYPE_VAR_STRING,
    }
}

fn map_doris_error(err: &DorisError) -> (ErrorKind, String) {
    match err {
        DorisError::AuthenticationFailed(_) => {
            (ErrorKind::ER_ACCESS_DENIED_ERROR, format!("Access denied: {}", err))
        }
        DorisError::QueryExecution(msg) => {
            let m = msg.as_str();
            if m.contains("Table '") && m.contains("does not exist in database") {
                (
                    ErrorKind::ER_NO_SUCH_TABLE,
                    format!("Query failed: {}", err),
                )
            } else if m.contains("Database '") && m.contains("does not exist") {
                (
                    ErrorKind::ER_BAD_DB_ERROR,
                    format!("Query failed: {}", err),
                )
            } else {
                // Default for parse / validation errors.
                (
                    ErrorKind::ER_PARSE_ERROR,
                    format!("Query failed: {}", err),
                )
            }
        }
        DorisError::BackendCommunication(_) => {
            (ErrorKind::ER_UNKNOWN_ERROR, format!("Backend error: {}", err))
        }
        DorisError::QueueFull => (
            ErrorKind::ER_QUERY_INTERRUPTED,
            "Query queue full".to_string(),
        ),
        DorisError::MysqlProtocol(_) | DorisError::InvalidPacket(_) => {
            (ErrorKind::ER_UNKNOWN_ERROR, format!("MySQL protocol error: {}", err))
        }
        DorisError::Io(_) => (
            ErrorKind::ER_UNKNOWN_ERROR,
            format!("IO error: {}", err),
        ),
        DorisError::Config(_) => (
            ErrorKind::ER_UNKNOWN_ERROR,
            format!("Configuration error: {}", err),
        ),
        DorisError::ConnectionClosed => (
            ErrorKind::ER_UNKNOWN_ERROR,
            err.to_string(),
        ),
    }
}
