// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! MySQL Server Implementation
//!
//! TCP server that implements MySQL wire protocol:
//! 1. Accept connections
//! 2. Send initial handshake
//! 3. Receive auth response
//! 4. Process commands (COM_QUERY, COM_INIT_DB, etc.)
//! 5. Return results

use crate::constants::*;
use crate::packet::Packet;
use crate::handshake::{InitialHandshake, HandshakeResponse41, verify_native_password};
use crate::resultset::{ResultSet, ColumnDefinition, TextResultRow, datatype_to_mysql_type};
use fe_common::Result;
use fe_catalog::Catalog;
use fe_analysis::DorisParser;
use fe_qe::{QueryExecutor, result::Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::io::Cursor;

/// MySQL server state
pub struct MysqlServer {
    catalog: Arc<Catalog>,
    connection_counter: AtomicU32,
    port: u16,
}

impl MysqlServer {
    pub fn new(catalog: Arc<Catalog>, port: u16) -> Self {
        Self {
            catalog,
            connection_counter: AtomicU32::new(1),
            port,
        }
    }

    /// Start server and listen for connections
    pub async fn start(&self) -> Result<()> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await
            .map_err(|e| fe_common::DorisError::InternalError(e.to_string()))?;

        println!("MySQL server listening on {}", addr);

        loop {
            let (socket, _) = listener.accept().await
                .map_err(|e| fe_common::DorisError::InternalError(e.to_string()))?;

            let connection_id = self.connection_counter.fetch_add(1, Ordering::SeqCst);
            let catalog = self.catalog.clone();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(socket, connection_id, catalog).await {
                    eprintln!("Connection {} error: {:?}", connection_id, e);
                }
            });
        }
    }
}

/// Handle a single client connection
async fn handle_connection(
    mut socket: TcpStream,
    connection_id: u32,
    catalog: Arc<Catalog>,
) -> Result<()> {
    println!("Connection {} established", connection_id);

    // 1. Send initial handshake
    let handshake = InitialHandshake::new(connection_id);
    let auth_plugin_data = handshake.auth_plugin_data.clone();
    let handshake_packet = handshake.to_packet(0);

    write_packet(&mut socket, &handshake_packet).await?;

    // 2. Receive auth response
    let auth_packet = read_packet(&mut socket).await?;
    let auth_response = HandshakeResponse41::from_packet(&auth_packet)
        .map_err(|e| fe_common::DorisError::InternalError(e.to_string()))?;

    println!("Connection {}: User '{}' attempting to connect",
        connection_id, auth_response.username);

    // 3. Verify authentication (allow all for MVP)
    // TODO: Implement proper user authentication
    let _auth_ok = verify_native_password("", &auth_response.auth_response, &auth_plugin_data);

    // 4. Send OK packet
    let ok_packet = Packet::ok(0, 0, SERVER_STATUS_AUTOCOMMIT, 0, 2);
    write_packet(&mut socket, &ok_packet).await?;

    println!("Connection {}: Authenticated successfully", connection_id);

    // 5. Set default database if specified
    let mut current_db = auth_response.database.unwrap_or_else(|| "default".to_string());

    // 6. Command loop
    let executor = QueryExecutor::new(catalog.clone());

    loop {
        let cmd_packet = match read_packet(&mut socket).await {
            Ok(p) => p,
            Err(_) => {
                println!("Connection {} closed", connection_id);
                break;
            }
        };

        if cmd_packet.payload.is_empty() {
            continue;
        }

        let cmd = cmd_packet.payload[0];
        let payload = &cmd_packet.payload[1..];

        match cmd {
            COM_QUIT => {
                println!("Connection {}: COM_QUIT received", connection_id);
                break;
            }

            COM_INIT_DB => {
                let db_name = String::from_utf8_lossy(payload).to_string();
                println!("Connection {}: COM_INIT_DB '{}'", connection_id, db_name);

                // Check if database exists (try to get it)
                if catalog.get_database(&db_name).is_ok() {
                    current_db = db_name;
                    let ok = Packet::ok(0, 0, SERVER_STATUS_AUTOCOMMIT, 0, cmd_packet.sequence_id + 1);
                    write_packet(&mut socket, &ok).await?;
                } else {
                    let err = Packet::err(
                        1049,
                        "42000",
                        &format!("Unknown database '{}'", db_name),
                        cmd_packet.sequence_id + 1,
                    );
                    write_packet(&mut socket, &err).await?;
                }
            }

            COM_QUERY => {
                let query = String::from_utf8_lossy(payload).to_string();
                println!("Connection {}: COM_QUERY '{}'", connection_id, query.trim());

                match handle_query(&query, &current_db, &executor, cmd_packet.sequence_id + 1).await {
                    Ok(packets) => {
                        for packet in packets {
                            write_packet(&mut socket, &packet).await?;
                        }
                    }
                    Err(e) => {
                        let err = Packet::err(
                            1064,
                            "42000",
                            &format!("{:?}", e),
                            cmd_packet.sequence_id + 1,
                        );
                        write_packet(&mut socket, &err).await?;
                    }
                }
            }

            COM_PING => {
                println!("Connection {}: COM_PING", connection_id);
                let ok = Packet::ok(0, 0, SERVER_STATUS_AUTOCOMMIT, 0, cmd_packet.sequence_id + 1);
                write_packet(&mut socket, &ok).await?;
            }

            _ => {
                println!("Connection {}: Unsupported command: 0x{:02X}", connection_id, cmd);
                let err = Packet::err(
                    1047,
                    "08S01",
                    "Unknown command",
                    cmd_packet.sequence_id + 1,
                );
                write_packet(&mut socket, &err).await?;
            }
        }
    }

    Ok(())
}

/// Handle COM_QUERY command
async fn handle_query(
    query: &str,
    _current_db: &str,
    executor: &QueryExecutor,
    sequence_id: u8,
) -> Result<Vec<Packet>> {
    // Handle special queries that we don't fully support yet
    let query_lower = query.trim().to_lowercase();

    // System variable queries - return OK to keep client happy
    if query_lower.starts_with("select @@") || query_lower.starts_with("set ") || query_lower.starts_with("show ") {
        // Return OK packet - client compatibility
        return Ok(vec![Packet::ok(0, 0, SERVER_STATUS_AUTOCOMMIT, 0, sequence_id)]);
    }

    // Parse SQL
    let stmt = DorisParser::parse_one(query)
        .map_err(|e| fe_common::DorisError::AnalysisError(format!("Parse error: {:?}", e)))?;

    // Execute statement
    let result = executor.execute(&stmt)?;

    match result {
        fe_qe::QueryResult::Ok(_msg) => {
            // DDL or DML that returns OK
            Ok(vec![Packet::ok(0, 0, SERVER_STATUS_AUTOCOMMIT, 0, sequence_id)])
        }

        fe_qe::QueryResult::ResultSet(rs) => {
            // SELECT query - encode result set
            let mut mysql_rs = ResultSet::new();

            // Add columns
            for col in &rs.columns {
                let mysql_type = datatype_to_mysql_type(&col.data_type);
                mysql_rs.add_column(ColumnDefinition::new(col.name.clone(), mysql_type));
            }

            // Add rows
            for row in &rs.rows {
                let values: Vec<Option<Vec<u8>>> = row.values.iter()
                    .map(|v| match v {
                        Value::Null => None,
                        Value::Boolean(b) => Some(if *b { b"1" } else { b"0" }.to_vec()),
                        Value::TinyInt(i) => Some(i.to_string().into_bytes()),
                        Value::SmallInt(i) => Some(i.to_string().into_bytes()),
                        Value::Int(i) => Some(i.to_string().into_bytes()),
                        Value::BigInt(i) => Some(i.to_string().into_bytes()),
                        Value::Float(f) => Some(f.to_string().into_bytes()),
                        Value::Double(f) => Some(f.to_string().into_bytes()),
                        Value::String(s) => Some(s.as_bytes().to_vec()),
                        Value::Date(s) => Some(s.as_bytes().to_vec()),
                        Value::DateTime(s) => Some(s.as_bytes().to_vec()),
                    })
                    .collect();

                mysql_rs.add_row(TextResultRow::new(values));
            }

            Ok(mysql_rs.to_packets(sequence_id, false))
        }

        fe_qe::QueryResult::Error(msg) => {
            Err(fe_common::DorisError::InternalError(msg))
        }
    }
}

/// Read a packet from the socket
async fn read_packet(socket: &mut TcpStream) -> Result<Packet> {
    use crate::packet::PacketHeader;

    // Read 4-byte header
    let mut header_buf = [0u8; 4];
    socket.read_exact(&mut header_buf).await
        .map_err(|e| fe_common::DorisError::InternalError(e.to_string()))?;

    let mut cursor = Cursor::new(&header_buf);
    let header = PacketHeader::read_from(&mut cursor)
        .map_err(|e| fe_common::DorisError::InternalError(e.to_string()))?;

    // Read payload
    let mut payload = vec![0u8; header.payload_length as usize];
    socket.read_exact(&mut payload).await
        .map_err(|e| fe_common::DorisError::InternalError(e.to_string()))?;

    Ok(Packet::new(header.sequence_id, payload))
}

/// Write a packet to the socket
async fn write_packet(socket: &mut TcpStream, packet: &Packet) -> Result<()> {
    let mut buf = Vec::new();
    packet.write_to(&mut buf)
        .map_err(|e| fe_common::DorisError::InternalError(e.to_string()))?;

    socket.write_all(&buf).await
        .map_err(|e| fe_common::DorisError::InternalError(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let catalog = Arc::new(Catalog::new());
        let server = MysqlServer::new(catalog, 19030);
        assert_eq!(server.port, 19030);
        assert_eq!(server.connection_counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_handle_query_ddl() {
        let catalog = Arc::new(Catalog::new());
        catalog.create_database("test".to_string(), "default".to_string()).unwrap();

        let executor = QueryExecutor::new(catalog);
        let query = "CREATE TABLE test.users (id INT, name VARCHAR(100))";

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(handle_query(query, "test", &executor, 1));

        assert!(result.is_ok());
        let packets = result.unwrap();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].payload[0], 0x00); // OK packet
    }

    #[test]
    fn test_handle_query_parse_error() {
        let catalog = Arc::new(Catalog::new());
        let executor = QueryExecutor::new(catalog);
        let query = "INVALID SQL SYNTAX";

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(handle_query(query, "test", &executor, 1));

        assert!(result.is_err());
    }
}
