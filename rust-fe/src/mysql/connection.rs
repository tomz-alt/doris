use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::{BytesMut, Bytes};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::error::{DorisError, Result};
use crate::query::QueryExecutor;
use crate::be::BackendClientPool;
use super::packet::*;
use super::protocol::*;

pub struct MysqlConnection {
    stream: TcpStream,
    sequence_id: u8,
    read_buffer: BytesMut,
    connection_id: u32,
    current_db: Option<String>,
    username: String,
    authenticated: bool,
    query_executor: Arc<QueryExecutor>,
    be_client_pool: Arc<BackendClientPool>,
    salt: Vec<u8>,
}

impl MysqlConnection {
    pub fn new(
        stream: TcpStream,
        connection_id: u32,
        query_executor: Arc<QueryExecutor>,
        be_client_pool: Arc<BackendClientPool>,
    ) -> Self {
        Self {
            stream,
            sequence_id: 0,
            read_buffer: BytesMut::with_capacity(8192),
            connection_id,
            current_db: None,
            username: String::new(),
            authenticated: false,
            query_executor,
            be_client_pool,
            salt: Vec::new(),
        }
    }

    pub async fn handle(mut self) -> Result<()> {
        info!("New MySQL connection: {}", self.connection_id);

        // Send handshake
        if let Err(e) = self.send_handshake().await {
            error!("Failed to send handshake: {}", e);
            return Err(e);
        }

        // Receive handshake response
        if let Err(e) = self.receive_handshake_response().await {
            error!("Failed to authenticate: {}", e);
            let _ = self.send_error(1045, "Access denied".to_string()).await;
            return Err(e);
        }

        // Send OK packet
        self.send_ok().await?;

        // Command loop
        loop {
            match self.handle_command().await {
                Ok(true) => continue,
                Ok(false) => {
                    info!("Client disconnected: {}", self.connection_id);
                    break;
                }
                Err(e) => {
                    error!("Error handling command: {}", e);
                    let _ = self.send_error(1064, e.to_string()).await;
                    // Continue on error, don't disconnect
                }
            }
        }

        Ok(())
    }

    async fn send_handshake(&mut self) -> Result<()> {
        let handshake = HandshakePacket::new(self.connection_id);
        self.salt = handshake.salt().to_vec();

        let payload = handshake.encode();
        let packet = Packet::new(0, payload);

        self.write_packet(packet).await?;
        self.sequence_id = 1;

        Ok(())
    }

    async fn receive_handshake_response(&mut self) -> Result<()> {
        let packet = self.read_packet().await?;

        let response = HandshakeResponse::decode(packet.payload)?;

        debug!("Handshake response - username: {}, db: {:?}",
               response.username, response.database);

        // Simple authentication (accept any password for PoC)
        // In production, verify response.auth_response against scrambled password
        self.username = response.username.clone();
        self.current_db = response.database;
        self.authenticated = true;

        Ok(())
    }

    async fn handle_command(&mut self) -> Result<bool> {
        let packet = self.read_packet().await?;

        if packet.payload.is_empty() {
            return Err(DorisError::InvalidPacket("Empty command packet".to_string()));
        }

        let mut payload = packet.payload;
        let cmd_byte = payload[0];
        let command = Command::from(cmd_byte);

        debug!("Command: {:?}", command);

        // Reset sequence ID for response
        self.sequence_id = 1;

        match command {
            Command::Quit => {
                return Ok(false);
            }
            Command::InitDb => {
                let db_name = String::from_utf8_lossy(&payload[1..]).to_string();
                self.current_db = Some(db_name.clone());
                info!("Changed database to: {}", db_name);
                self.send_ok().await?;
            }
            Command::Query => {
                let query = String::from_utf8_lossy(&payload[1..]).to_string();
                self.handle_query(query).await?;
            }
            Command::Ping => {
                self.send_ok().await?;
            }
            Command::FieldList => {
                // Send empty result set
                self.send_result_set(vec![], vec![]).await?;
            }
            Command::StmtPrepare => {
                // For PoC, return error for prepared statements
                self.send_error(1047, "Prepared statements not supported in PoC".to_string()).await?;
            }
            Command::StmtExecute | Command::StmtClose => {
                // Not implemented for PoC
                self.send_error(1047, "Prepared statements not supported in PoC".to_string()).await?;
            }
            Command::Unknown(b) => {
                warn!("Unknown command: 0x{:02x}", b);
                self.send_error(1047, format!("Unknown command: {}", b)).await?;
            }
        }

        Ok(true)
    }

    async fn handle_query(&mut self, query: String) -> Result<()> {
        info!("Query: {}", query.trim());

        let query_trimmed = query.trim().to_lowercase();

        // Handle special queries
        if query_trimmed == "select 1" || query_trimmed == "select 1;" {
            let columns = vec![
                ColumnDefinition::new("1".to_string(), ColumnType::Long)
            ];
            let rows = vec![
                ResultRow::new(vec![Some("1".to_string())])
            ];
            return self.send_result_set(columns, rows).await;
        }

        if query_trimmed.starts_with("select @@") || query_trimmed.starts_with("show ") {
            // Handle metadata queries
            return self.handle_metadata_query(&query).await;
        }

        if query_trimmed.starts_with("set ") {
            // Accept SET statements without doing anything (for PoC)
            return self.send_ok().await;
        }

        if query_trimmed.starts_with("use ") {
            // Change database
            let db_name = query_trimmed.strip_prefix("use ").unwrap().trim_end_matches(';').trim();
            self.current_db = Some(db_name.to_string());
            info!("Changed database to: {}", db_name);
            return self.send_ok().await;
        }

        // Queue query for execution
        let query_id = uuid::Uuid::new_v4();
        let current_db = self.current_db.clone();

        match self.query_executor.queue_query(query_id, query.clone(), current_db).await {
            Ok(()) => {
                // Execute query
                match self.query_executor.execute_query(query_id, &self.be_client_pool).await {
                    Ok(result) => {
                        self.send_query_result(result).await?;
                    }
                    Err(e) => {
                        error!("Query execution failed: {}", e);
                        self.send_error(1064, format!("Query failed: {}", e)).await?;
                    }
                }
            }
            Err(e) => {
                error!("Failed to queue query: {}", e);
                self.send_error(1203, "Too many queries queued".to_string()).await?;
            }
        }

        Ok(())
    }

    async fn handle_metadata_query(&mut self, query: &str) -> Result<()> {
        let query_lower = query.to_lowercase();

        if query_lower.contains("@@version_comment") {
            let columns = vec![
                ColumnDefinition::new("@@version_comment".to_string(), ColumnType::VarString)
            ];
            let rows = vec![
                ResultRow::new(vec![Some("Doris Rust FE PoC".to_string())])
            ];
            return self.send_result_set(columns, rows).await;
        }

        if query_lower.contains("@@version") || query_lower.contains("version()") {
            let columns = vec![
                ColumnDefinition::new("version()".to_string(), ColumnType::VarString)
            ];
            let rows = vec![
                ResultRow::new(vec![Some("8.0.0-doris-rust".to_string())])
            ];
            return self.send_result_set(columns, rows).await;
        }

        if query_lower.starts_with("show databases") {
            let columns = vec![
                ColumnDefinition::new("Database".to_string(), ColumnType::VarString)
            ];
            let rows = vec![
                ResultRow::new(vec![Some("information_schema".to_string())]),
                ResultRow::new(vec![Some("test".to_string())]),
            ];
            return self.send_result_set(columns, rows).await;
        }

        if query_lower.starts_with("show tables") {
            let columns = vec![
                ColumnDefinition::new("Tables_in_db".to_string(), ColumnType::VarString)
            ];
            let rows = vec![
                ResultRow::new(vec![Some("example_table".to_string())]),
            ];
            return self.send_result_set(columns, rows).await;
        }

        // Default: return empty result set
        self.send_result_set(vec![], vec![]).await
    }

    async fn send_query_result(&mut self, result: crate::query::QueryResult) -> Result<()> {
        if result.is_dml {
            // For DML queries (INSERT, UPDATE, DELETE), send OK packet
            let mut ok = OkPacket::new();
            ok.affected_rows = result.affected_rows;
            self.send_ok_packet(ok).await
        } else {
            // For SELECT queries, send result set
            self.send_result_set(result.columns, result.rows).await
        }
    }

    async fn send_result_set(&mut self, columns: Vec<ColumnDefinition>, rows: Vec<ResultRow>) -> Result<()> {
        // Column count
        let mut col_count_buf = BytesMut::new();
        write_lenenc_int(&mut col_count_buf, columns.len() as u64);
        self.write_packet(Packet::new(self.sequence_id, col_count_buf.freeze())).await?;
        self.sequence_id += 1;

        // Column definitions
        for col in columns {
            self.write_packet(Packet::new(self.sequence_id, col.encode())).await?;
            self.sequence_id += 1;
        }

        // EOF packet after columns (if not using CLIENT_DEPRECATE_EOF)
        let eof = EofPacket::new();
        self.write_packet(Packet::new(self.sequence_id, eof.encode())).await?;
        self.sequence_id += 1;

        // Rows
        for row in rows {
            self.write_packet(Packet::new(self.sequence_id, row.encode())).await?;
            self.sequence_id += 1;
        }

        // EOF packet after rows
        let eof = EofPacket::new();
        self.write_packet(Packet::new(self.sequence_id, eof.encode())).await?;
        self.sequence_id += 1;

        Ok(())
    }

    async fn send_ok(&mut self) -> Result<()> {
        let ok = OkPacket::new();
        self.send_ok_packet(ok).await
    }

    async fn send_ok_packet(&mut self, ok: OkPacket) -> Result<()> {
        let packet = Packet::new(self.sequence_id, ok.encode());
        self.write_packet(packet).await?;
        self.sequence_id += 1;
        Ok(())
    }

    async fn send_error(&mut self, code: u16, message: String) -> Result<()> {
        let err = ErrPacket::new(code, message);
        let packet = Packet::new(self.sequence_id, err.encode());
        self.write_packet(packet).await?;
        self.sequence_id += 1;
        Ok(())
    }

    async fn read_packet(&mut self) -> Result<Packet> {
        loop {
            // Try to decode a packet from buffer
            if let Some(packet) = Packet::decode(&mut self.read_buffer)? {
                self.sequence_id = packet.sequence_id.wrapping_add(1);
                return Ok(packet);
            }

            // Read more data
            let mut buf = vec![0u8; 8192];
            let n = self.stream.read(&mut buf).await?;

            if n == 0 {
                return Err(DorisError::ConnectionClosed);
            }

            self.read_buffer.extend_from_slice(&buf[..n]);
        }
    }

    async fn write_packet(&mut self, packet: Packet) -> Result<()> {
        let encoded = packet.encode();
        self.stream.write_all(&encoded).await?;
        self.stream.flush().await?;
        Ok(())
    }
}
