use axum::{
    extract::{Path, State},
    http::{StatusCode, HeaderMap},
    body::Body,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use std::sync::Arc;
use tracing::{info, error};
use serde_json::json;

use crate::query::QueryExecutor;
use crate::be::BackendClientPool;

pub struct AppState {
    pub query_executor: Arc<QueryExecutor>,
    pub be_client_pool: Arc<BackendClientPool>,
}

// Stream load handler: PUT /api/{db}/{table}/_stream_load
pub async fn stream_load_handler(
    Path((db, table)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    info!("Stream load request: db={}, table={}, size={} bytes", db, table, body.len());

    // Extract headers
    let label = headers.get("label")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default_label");

    let format = headers.get("format")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("csv");

    let column_separator = headers.get("column_separator")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(",");

    info!("Stream load params: label={}, format={}, separator={}",
          label, format, column_separator);

    // For PoC: Parse CSV data and convert to INSERT statement
    let insert_query = match parse_csv_to_insert(&db, &table, &body, column_separator) {
        Ok(query) => query,
        Err(e) => {
            error!("Failed to parse CSV: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                json!({
                    "Status": "Fail",
                    "Message": format!("Parse error: {}", e),
                }).to_string(),
            ).into_response();
        }
    };

    // Queue and execute the insert query
    let query_id = uuid::Uuid::new_v4();

    match state.query_executor.queue_query(query_id, insert_query, Some(db.clone())).await {
        Ok(()) => {
            match state.query_executor.execute_query(query_id, &state.be_client_pool).await {
                Ok(result) => {
                    info!("Stream load completed: {} rows affected", result.affected_rows);

                    (
                        StatusCode::OK,
                        json!({
                            "TxnId": query_id.to_string(),
                            "Label": label,
                            "Status": "Success",
                            "Message": "OK",
                            "NumberTotalRows": result.affected_rows,
                            "NumberLoadedRows": result.affected_rows,
                            "NumberFilteredRows": 0,
                            "NumberUnselectedRows": 0,
                            "LoadBytes": body.len(),
                            "LoadTimeMs": 100,
                        }).to_string(),
                    ).into_response()
                }
                Err(e) => {
                    error!("Stream load execution failed: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({
                            "Status": "Fail",
                            "Message": format!("Execution error: {}", e),
                        }).to_string(),
                    ).into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to queue stream load: {}", e);
            (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({
                    "Status": "Fail",
                    "Message": format!("Queue full: {}", e),
                }).to_string(),
            ).into_response()
        }
    }
}

// Health check handler
pub async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        json!({
            "status": "healthy",
            "service": "doris-rust-fe",
        }).to_string(),
    )
}

// Status handler
pub async fn status_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let (queue_size, available_slots, max_concurrent) = state.query_executor.queue_stats();

    (
        StatusCode::OK,
        json!({
            "service": "doris-rust-fe",
            "query_queue": {
                "size": queue_size,
                "available_slots": available_slots,
                "max_concurrent": max_concurrent,
            },
            "backend_nodes": state.be_client_pool.backend_count(),
        }).to_string(),
    )
}

fn parse_csv_to_insert(db: &str, table: &str, data: &Bytes, separator: &str) -> Result<String, String> {
    let content = String::from_utf8_lossy(data);
    let lines: Vec<&str> = content.lines().collect();

    if lines.is_empty() {
        return Err("Empty data".to_string());
    }

    // Parse header (first line)
    let header: Vec<&str> = lines[0].split(separator).map(|s| s.trim()).collect();

    if header.is_empty() {
        return Err("Empty header".to_string());
    }

    // Build INSERT statement
    let mut values = Vec::new();

    for line in &lines[1..] {
        if line.trim().is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split(separator).map(|s| s.trim()).collect();

        if fields.len() != header.len() {
            continue; // Skip malformed rows
        }

        // Quote string values
        let quoted_fields: Vec<String> = fields.iter()
            .map(|f| {
                // Simple heuristic: if it contains letters or starts with 0, treat as string
                if f.chars().any(|c| c.is_alphabetic()) || f.starts_with('0') {
                    format!("'{}'", f.replace('\'', "''"))
                } else {
                    f.to_string()
                }
            })
            .collect();

        values.push(format!("({})", quoted_fields.join(", ")));
    }

    if values.is_empty() {
        return Err("No data rows".to_string());
    }

    let columns = header.join(", ");
    let query = format!(
        "INSERT INTO {}.{} ({}) VALUES {}",
        db,
        table,
        columns,
        values.join(", ")
    );

    Ok(query)
}
