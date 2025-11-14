use tokio::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::{info, error};

use crate::error::Result;
use crate::query::QueryExecutor;
use crate::be::BackendClientPool;
use super::connection::MysqlConnection;

pub struct MysqlServer {
    port: u16,
    connection_id_counter: Arc<AtomicU32>,
    query_executor: Arc<QueryExecutor>,
    be_client_pool: Arc<BackendClientPool>,
}

impl MysqlServer {
    pub fn new(
        port: u16,
        query_executor: Arc<QueryExecutor>,
        be_client_pool: Arc<BackendClientPool>,
    ) -> Self {
        Self {
            port,
            connection_id_counter: Arc::new(AtomicU32::new(1)),
            query_executor,
            be_client_pool,
        }
    }

    pub async fn serve(self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("MySQL server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let connection_id = self.connection_id_counter.fetch_add(1, Ordering::SeqCst);
                    let query_executor = self.query_executor.clone();
                    let be_client_pool = self.be_client_pool.clone();

                    info!("Accepted connection from {}, ID: {}", addr, connection_id);

                    tokio::spawn(async move {
                        let conn = MysqlConnection::new(
                            stream,
                            connection_id,
                            query_executor,
                            be_client_pool,
                        );

                        if let Err(e) = conn.handle().await {
                            error!("Connection {} error: {}", connection_id, e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}
