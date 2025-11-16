use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};
use bytes::{BytesMut, Bytes};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::error::{DorisError, Result};
use crate::query::{QueryExecutor, SessionCtx};
use crate::be::BackendClientPool;
use super::packet::*;
use super::protocol::*;

pub struct MysqlConnection {
    stream: TcpStream,
    sequence_id: u8,
    read_buffer: BytesMut,
    connection_id: u32,
    current_db: Option<String>,
    session: SessionCtx,
    username: String,
    authenticated: bool,
    query_executor: Arc<QueryExecutor>,
    be_client_pool: Arc<BackendClientPool>,
    salt: Vec<u8>,
    client_capabilities: u32,
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
            session: SessionCtx::new(),
            username: String::new(),
            authenticated: false,
            query_executor,
            be_client_pool,
            salt: Vec::new(),
            client_capabilities: 0,
        }
    }

    pub async fn handle(mut self) -> Result<()> {
        info!("New MySQL connection: {}", self.connection_id);

        // Send handshake
        info!("Sending handshake to connection: {}", self.connection_id);
        if let Err(e) = self.send_handshake().await {
            error!("Failed to send handshake: {}", e);
            return Err(e);
        }
        info!("Handshake sent successfully to connection: {}", self.connection_id);

        // Receive handshake response
        info!("Waiting for handshake response from connection: {}", self.connection_id);
        if let Err(e) = self.receive_handshake_response().await {
            error!("Failed to authenticate connection {}: {}", self.connection_id, e);
            let _ = self.send_error(1045, "Access denied".to_string()).await;
            return Err(e);
        }
        info!("Authentication successful for connection: {}", self.connection_id);

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
                Err(DorisError::ConnectionClosed) => {
                    info!(
                        "Client closed connection while handling command: {}",
                        self.connection_id
                    );
                    break;
                }
                Err(e) => {
                    error!("Error handling command: {}", e);
                    let _ = self.send_error(1064, e.to_string()).await;
                    // Treat unexpected protocol errors as fatal to avoid busy loops.
                    break;
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

        self.client_capabilities = response.capability_flags;
        info!(
            "Handshake response - username: {}, db: {:?}, capabilities: 0x{:08x}",
            response.username, response.database, self.client_capabilities
        );

        // Simple authentication (accept any password for PoC)
        // In production, verify response.auth_response against scrambled password
        self.username = response.username.clone();
        self.current_db = response.database.clone();
        self.session.database = response.database;
        self.session.user = self.username.clone();
        self.session.protocol = crate::query::ProtocolType::Mysql;
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
                self.session.database = Some(db_name.clone());
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

        // Handle MySQL client probe queries - just send OK
        if query_trimmed.contains("$$") {
            // This is a MySQL client delimiter probe - just acknowledge it
            return self.send_ok().await;
        }

        if query_trimmed.starts_with("select @@")
            || query_trimmed.starts_with("select version()")
            || query_trimmed.starts_with("show ")
            || query_trimmed.starts_with("desc ")
            || query_trimmed.starts_with("describe ") {
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
            self.session.database = Some(db_name.to_string());
            info!("Changed database to: {}", db_name);
            return self.send_ok().await;
        }

        // Update session context from current database
        self.session.database = self.current_db.clone();

        match self
            .query_executor
            .execute_sql(&mut self.session, &query, &self.be_client_pool)
            .await
        {
            Ok(result) => {
                self.send_query_result(result).await?;
            }
            Err(e) => {
                error!("Query execution failed: {}", e);
                self.send_error(1064, format!("Query failed: {}", e)).await?;
            }
        }

        Ok(())
    }

    async fn handle_metadata_query(&mut self, query: &str) -> Result<()> {
        info!("handle_metadata_query called with query: {}", query);
        let query_lower = query.to_lowercase();

        if query_lower.contains("@@version_comment") {
            info!("Handling @@version_comment query, sending result set");
            let columns = vec![
                ColumnDefinition::new("@@version_comment".to_string(), ColumnType::VarString)
            ];
            let rows = vec![
                ResultRow::new(vec![Some("Doris Rust FE PoC".to_string())])
            ];
            let result = self.send_result_set(columns, rows).await;
            info!("Result set send completed with: {:?}", result);
            return result;
        }

        if query_lower.contains("@@version") || query_lower.contains("version()") {
            let columns = vec![
                ColumnDefinition::new("version()".to_string(), ColumnType::VarString)
            ];
            let rows = vec![
                // Match the server version string advertised in the MySQL
                // handshake for consistent client behavior.
                ResultRow::new(vec![Some("5.7.99".to_string())])
            ];
            return self.send_result_set(columns, rows).await;
        }

        if query_lower.starts_with("show databases") {
            let columns = vec![
                ColumnDefinition::new("Database".to_string(), ColumnType::VarString)
            ];

            let catalog = crate::metadata::catalog::catalog();
            let rows: Vec<ResultRow> = catalog
                .list_databases()
                .into_iter()
                .map(|db| ResultRow::new(vec![Some(db)]))
                .collect();

            return self.send_result_set(columns, rows).await;
        }

        if query_lower.starts_with("show tables") {
            // Get current database name for column header
            let db_name = self.current_db.as_ref().map(|s| s.as_str()).unwrap_or("db");
            let column_name = format!("Tables_in_{}", db_name);

            let columns = vec![
                ColumnDefinition::new(column_name, ColumnType::VarString)
            ];

            // Get actual table list from query executor
            let rows = match self.query_executor.list_tables(db_name).await {
                Ok(tables) => tables.into_iter()
                    .map(|t| ResultRow::new(vec![Some(t)]))
                    .collect(),
                Err(_) => vec![
                    ResultRow::new(vec![Some("lineitem".to_string())]),
                    ResultRow::new(vec![Some("orders".to_string())]),
                    ResultRow::new(vec![Some("customer".to_string())]),
                    ResultRow::new(vec![Some("part".to_string())]),
                    ResultRow::new(vec![Some("partsupp".to_string())]),
                    ResultRow::new(vec![Some("supplier".to_string())]),
                    ResultRow::new(vec![Some("nation".to_string())]),
                    ResultRow::new(vec![Some("region".to_string())]),
                ],
            };

            return self.send_result_set(columns, rows).await;
        }

        // Handle DESCRIBE/DESC commands
        if query_lower.starts_with("describe ") || query_lower.starts_with("desc ") {
            // Extract table name
            let table_name = query_lower
                .trim_start_matches("describe ")
                .trim_start_matches("desc ")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c| c == '`' || c == '\'' || c == '"' || c == ';');

            // MySQL DESCRIBE format: Field, Type, Null, Key, Default, Extra
            let columns = vec![
                ColumnDefinition::new("Field".to_string(), ColumnType::VarString),
                ColumnDefinition::new("Type".to_string(), ColumnType::VarString),
                ColumnDefinition::new("Null".to_string(), ColumnType::VarString),
                ColumnDefinition::new("Key".to_string(), ColumnType::VarString),
                ColumnDefinition::new("Default".to_string(), ColumnType::VarString),
                ColumnDefinition::new("Extra".to_string(), ColumnType::VarString),
            ];

            // Use catalog-backed schema from QueryExecutor
            let db_name = self.current_db.as_deref().unwrap_or("tpch");

            let rows = match self
                .query_executor
                .describe_table(db_name, table_name)
                .await
            {
                Ok(schema) => schema.into_iter()
                    .map(|(field_name, field_type, nullable)| {
                        ResultRow::new(vec![
                            Some(field_name),
                            Some(field_type),
                            Some(if nullable { "YES" } else { "NO" }.to_string()),
                            None,  // Key
                            None,  // Default
                            Some("".to_string()),  // Extra
                        ])
                    })
                    .collect(),
                Err(_) => vec![
                    ResultRow::new(vec![
                        Some("id".to_string()),
                        Some("BIGINT".to_string()),
                        Some("NO".to_string()),
                        None,
                        None,
                        Some("".to_string()),
                    ]),
                ],
            };

            return self.send_result_set(columns, rows).await;
        }

        // Default: return empty result set with proper structure
        // Don't send truly empty result sets as they cause protocol issues
        warn!("Unhandled metadata query: {}", query);
        let columns = vec![
            ColumnDefinition::new("Result".to_string(), ColumnType::VarString)
        ];
        self.send_result_set(columns, vec![]).await
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
        use super::protocol::CLIENT_DEPRECATE_EOF;

        // Only enable CLIENT_DEPRECATE_EOF behavior if both client and server
        // agree on the capability.
        let negotiated_caps = self.client_capabilities & server_capabilities();
        let deprecate_eof = (negotiated_caps & CLIENT_DEPRECATE_EOF) != 0;
        debug!("Sending result set: {} columns, {} rows, deprecate_eof: {}",
               columns.len(), rows.len(), deprecate_eof);

        // Column count
        let mut col_count_buf = BytesMut::new();
        write_lenenc_int(&mut col_count_buf, columns.len() as u64);
        self.write_packet(Packet::new(self.sequence_id, col_count_buf.freeze())).await?;
        self.sequence_id += 1;
        debug!("Sent column count");

        // Column definitions
        for (i, col) in columns.iter().enumerate() {
            self.write_packet(Packet::new(self.sequence_id, col.encode())).await?;
            self.sequence_id += 1;
            debug!("Sent column {} definition", i);
        }

        // Delimiter after column definitions:
        // - Old behavior: EOF packet (0xFE) when CLIENT_DEPRECATE_EOF is not set.
        // - New behavior: OK packet when CLIENT_DEPRECATE_EOF is set, matching MySQL 5.7+.
        if deprecate_eof {
            debug!("Sending OK after columns (CLIENT_DEPRECATE_EOF)");
            let ok = OkPacket::new();
            self.write_packet(Packet::new(self.sequence_id, ok.encode())).await?;
            self.sequence_id += 1;
        } else {
            debug!("Sending EOF after columns");
            let eof = EofPacket::new();
            self.write_packet(Packet::new(self.sequence_id, eof.encode())).await?;
            self.sequence_id += 1;
        }

        // Rows
        for (i, row) in rows.iter().enumerate() {
            self.write_packet(Packet::new(self.sequence_id, row.encode())).await?;
            self.sequence_id += 1;
            debug!("Sent row {}", i);
        }

        // Final packet: OK if CLIENT_DEPRECATE_EOF is set, otherwise EOF.
        //
        // MySQL 5.7+ uses an OK packet (header 0x00) to replace EOF when
        // CLIENT_DEPRECATE_EOF is negotiated. Older clients still expect a
        // traditional EOF packet (0xFE, 5-byte payload).
        if deprecate_eof {
            debug!("Sending final OK packet (CLIENT_DEPRECATE_EOF)");
            let mut ok = OkPacket::new();
            ok.status_flags &= !SERVER_MORE_RESULTS_EXISTS;
            self.write_packet(Packet::new(self.sequence_id, ok.encode())).await?;
        } else {
            debug!("Sending final EOF packet");
            let eof = EofPacket::new();
            self.write_packet(Packet::new(self.sequence_id, eof.encode())).await?;
        }
        self.sequence_id += 1;
        debug!("Result set sent successfully");

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
            debug!("Trying to decode packet, buffer len: {}", self.read_buffer.len());
            if let Some(packet) = Packet::decode(&mut self.read_buffer)? {
                debug!("Decoded packet: seq={}, len={}", packet.sequence_id, packet.payload.len());
                self.sequence_id = packet.sequence_id.wrapping_add(1);
                return Ok(packet);
            }

            // Read more data
            debug!("Reading more data from socket...");
            let mut buf = vec![0u8; 8192];
            // Apply a reasonable read timeout to avoid hanging connections.
            let n = match timeout(Duration::from_secs(30), self.stream.read(&mut buf)).await {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => return Err(DorisError::Io(e)),
                Err(_) => {
                    warn!(
                        "MySQL connection {} read timed out while waiting for packet",
                        self.connection_id
                    );
                    return Err(DorisError::MysqlProtocol(
                        "MySQL connection read timeout".to_string(),
                    ));
                }
            };

            debug!("Read {} bytes from socket", n);
            if n == 0 {
                warn!("Client closed connection (read 0 bytes)");
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
