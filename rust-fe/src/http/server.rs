use axum::{
    routing::{get, put},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::error::Result;
use crate::query::QueryExecutor;
use crate::be::BackendClientPool;
use super::handlers::{AppState, stream_load_handler, health_handler, status_handler};

pub struct HttpServer {
    port: u16,
    query_executor: Arc<QueryExecutor>,
    be_client_pool: Arc<BackendClientPool>,
}

impl HttpServer {
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
        let state = Arc::new(AppState {
            query_executor: self.query_executor,
            be_client_pool: self.be_client_pool,
        });

        let app = Router::new()
            // Stream load endpoint
            .route("/api/:db/:table/_stream_load", put(stream_load_handler))
            // Health check
            .route("/api/health", get(health_handler))
            // Status endpoint
            .route("/api/status", get(status_handler))
            // Add tracing layer
            .layer(TraceLayer::new_for_http())
            .with_state(state);

        let addr = format!("0.0.0.0:{}", self.port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        info!("HTTP server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}
