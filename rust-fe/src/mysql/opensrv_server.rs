use std::sync::Arc;

use opensrv_mysql::AsyncMysqlIntermediary;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::be::BackendClientPool;
use crate::error::Result;
use crate::query::QueryExecutor;

use super::opensrv_shim::DorisShim;

pub struct MysqlServer {
    port: u16,
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
            query_executor,
            be_client_pool,
        }
    }

    pub async fn serve(self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("MySQL server (opensrv) listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let query_executor = self.query_executor.clone();
                    let be_client_pool = self.be_client_pool.clone();

                    info!("Accepted MySQL connection from {}", peer_addr);

                    tokio::spawn(async move {
                        let (r, w) = stream.into_split();
                        let shim = DorisShim::new(query_executor, be_client_pool);

                        if let Err(e) = AsyncMysqlIntermediary::run_on(shim, r, w).await {
                            error!("opensrv connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept MySQL connection: {}", e);
                }
            }
        }
    }
}

