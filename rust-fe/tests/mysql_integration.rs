use std::sync::Arc;

use bytes::{Buf, BytesMut};
use doris_rust_fe::{QueryExecutor, Result};
use doris_rust_fe::be::BackendClientPool;
use doris_rust_fe::mysql::{
    DorisShim,
    Packet,
    read_lenenc_int,
    write_null_terminated_str,
    CLIENT_PROTOCOL_41,
    CLIENT_SECURE_CONNECTION,
    CLIENT_CONNECT_WITH_DB,
    CLIENT_DEPRECATE_EOF,
    UTF8MB4_GENERAL_CI,
};
use opensrv_mysql::AsyncMysqlIntermediary;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

/// Read a single MySQL packet from the stream into a Packet structure.
async fn read_packet(stream: &mut TcpStream, buf: &mut BytesMut) -> Result<Packet> {
    loop {
        if let Some(pkt) = Packet::decode(buf)? {
            return Ok(pkt);
        }

        let mut tmp = [0u8; 1024];
        let n = stream.read(&mut tmp).await?;
        if n == 0 {
            return Err(doris_rust_fe::DorisError::ConnectionClosed);
        }
        buf.extend_from_slice(&tmp[..n]);
    }
}

async fn read_packet_with_timeout(stream: &mut TcpStream, buf: &mut BytesMut) -> Result<Packet> {
    match timeout(Duration::from_secs(5), read_packet(stream, buf)).await {
        Ok(result) => result,
        Err(_) => Err(doris_rust_fe::DorisError::MysqlProtocol(
            "read_packet timed out in mysql_integration test".to_string(),
        )),
    }
}

/// Build a minimal Handshake Response packet payload suitable for our server.
fn build_handshake_response_with_flags(user: &str, db: Option<&str>, extra_flags: u32) -> bytes::Bytes {
    use bytes::BufMut;

    let mut buf = BytesMut::new();

    // Capability flags: protocol 4.1, secure connection, plus any extra flags,
    // optionally CONNECT_WITH_DB.
    let mut flags = CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION | extra_flags;
    if db.is_some() {
        flags |= CLIENT_CONNECT_WITH_DB;
    }

    buf.put_u32_le(flags);
    buf.put_u32_le(1024 * 1024 * 16); // max_packet_size
    buf.put_u8(UTF8MB4_GENERAL_CI);   // character set
    buf.put_bytes(0, 23);            // reserved

    // username (null-terminated)
    write_null_terminated_str(&mut buf, user);

    // auth_response in secure connection format: length byte + data
    buf.put_u8(0); // zero-length auth response (accepted in PoC auth)

    // optional database name (null-terminated)
    if let Some(dbname) = db {
        write_null_terminated_str(&mut buf, dbname);
    }

    buf.freeze()
}

fn build_handshake_response(user: &str, db: Option<&str>) -> bytes::Bytes {
    build_handshake_response_with_flags(user, db, 0)
}

async fn start_single_connection_server(
    listener: TcpListener,
    executor: Arc<QueryExecutor>,
    be_pool: Arc<BackendClientPool>,
) -> Result<()> {
    if let Ok((stream, _)) = listener.accept().await {
        let (r, w) = stream.into_split();
        let shim = DorisShim::new(executor, be_pool);
        let _ = AsyncMysqlIntermediary::run_on(shim, r, w).await;
    }

    Ok(())
}

async fn setup_mysql_connection() -> Result<(TcpStream, tokio::task::JoinHandle<()>)> {
    // Create executor and BE pool
    let executor = Arc::new(QueryExecutor::with_datafusion(16, 4).await);
    let be_pool = Arc::new(BackendClientPool::new(Vec::new()));

    // Bind to an ephemeral port and keep the listener so the port is
    // guaranteed to be listening before the client connects.
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // Spawn server that accepts a single connection using the existing listener
    let server_executor = executor.clone();
    let server_be_pool = be_pool.clone();
    let server_handle = tokio::spawn(async move {
        let _ = start_single_connection_server(listener, server_executor, server_be_pool).await;
    });

    // Connect client
    let mut stream = TcpStream::connect(addr).await?;

    // Read handshake (with timeout to avoid hanging tests)
    let mut buf = BytesMut::with_capacity(1024);
    let _handshake_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // Send handshake response (without CLIENT_DEPRECATE_EOF by default)
    let response_payload = build_handshake_response("root", Some("tpch"));
    let response_packet = Packet::new(1, response_payload);
    stream.write_all(&response_packet.encode()).await?;
    stream.flush().await?;

    // Read OK after auth
    let ok_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!ok_packet.payload.is_empty());
    assert_eq!(ok_packet.payload[0], 0x00);

    Ok((stream, server_handle))
}

async fn setup_mysql_connection_with_deprecate_eof() -> Result<(TcpStream, tokio::task::JoinHandle<()>)> {
    // Create executor and BE pool
    let executor = Arc::new(QueryExecutor::with_datafusion(16, 4).await);
    let be_pool = Arc::new(BackendClientPool::new(Vec::new()));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server_executor = executor.clone();
    let server_be_pool = be_pool.clone();
    let server_handle = tokio::spawn(async move {
        let _ = start_single_connection_server(listener, server_executor, server_be_pool).await;
    });

    let mut stream = TcpStream::connect(addr).await?;

    let mut buf = BytesMut::with_capacity(1024);
    let _handshake_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // Include CLIENT_DEPRECATE_EOF in the client capabilities.
    let response_payload = build_handshake_response_with_flags("root", Some("tpch"), CLIENT_DEPRECATE_EOF);
    let response_packet = Packet::new(1, response_payload);
    stream.write_all(&response_packet.encode()).await?;
    stream.flush().await?;

    let ok_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!ok_packet.payload.is_empty());
    assert_eq!(ok_packet.payload[0], 0x00);

    Ok((stream, server_handle))
}

#[tokio::test]
async fn mysql_integration_select_1() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    // Send COM_QUERY "SELECT 1"
    use bytes::BufMut;
    let mut qpayload = BytesMut::new();
    // COM_QUERY command byte (0x03)
    qpayload.put_u8(0x03);
    qpayload.put_slice(b"SELECT 1");
    let qpacket = Packet::new(0, qpayload.freeze());
    stream.write_all(&qpacket.encode()).await?;
    stream.flush().await?;

    // Read column count packet
    let col_count_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut payload = col_count_packet.payload.clone();
    let col_count = read_lenenc_int(&mut payload).unwrap();
    assert_eq!(col_count, 1);

    // Read single column definition packet (ignore content)
    let _col_def_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // Read EOF/OK after column definitions
    let _after_columns = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // Read row packet
    let row_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut row_payload = row_packet.payload.clone();
    let val_len = read_lenenc_int(&mut row_payload).unwrap() as usize;
    let val_bytes = row_payload.copy_to_bytes(val_len);
    assert_eq!(&val_bytes[..], b"1");

    // Read final EOF/OK packet and ignore
    let _final_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // Drop the client stream to let server task finish
    drop(stream);
    let _ = server_handle.await;

    Ok(())
}

/// Verify result-set framing when CLIENT_DEPRECATE_EOF is negotiated:
/// - No EOF packet after column definitions (OK instead)
/// - Final terminator is OK instead of EOF.
#[tokio::test]
async fn mysql_integration_select_1_with_deprecate_eof() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection_with_deprecate_eof().await?;
    let mut buf = BytesMut::with_capacity(1024);

    use bytes::BufMut;
    let mut qpayload = BytesMut::new();
    qpayload.put_u8(0x03);
    qpayload.put_slice(b"SELECT 1");
    let qpacket = Packet::new(0, qpayload.freeze());
    stream.write_all(&qpacket.encode()).await?;
    stream.flush().await?;

    // Column count
    let col_count_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut payload = col_count_packet.payload.clone();
    let col_count = read_lenenc_int(&mut payload).unwrap();
    assert_eq!(col_count, 1);

    // Column definition
    let _col_def_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // With CLIENT_DEPRECATE_EOF, we must see an OK packet here, not EOF.
    let after_columns = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!after_columns.payload.is_empty());
    assert_eq!(after_columns.payload[0], 0x00, "expected OK after columns when CLIENT_DEPRECATE_EOF is set");

    // Row packet
    let row_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut row_payload = row_packet.payload.clone();
    let val_len = read_lenenc_int(&mut row_payload).unwrap() as usize;
    let val_bytes = row_payload.copy_to_bytes(val_len);
    assert_eq!(&val_bytes[..], b"1");

    // Final terminator: OK (0x00), not EOF.
    let final_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!final_packet.payload.is_empty());
    assert_eq!(final_packet.payload[0], 0x00, "expected final OK instead of EOF when CLIENT_DEPRECATE_EOF is set");

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn mysql_integration_select_version() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    // COM_QUERY "SELECT VERSION()"
    use bytes::BufMut;
    let mut qpayload = BytesMut::new();
    qpayload.put_u8(0x03); // COM_QUERY
    qpayload.put_slice(b"SELECT VERSION()");
    let qpacket = Packet::new(0, qpayload.freeze());
    stream.write_all(&qpacket.encode()).await?;
    stream.flush().await?;

    // Column count packet
    let col_count_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut payload = col_count_packet.payload.clone();
    let col_count = read_lenenc_int(&mut payload).unwrap();
    assert_eq!(col_count, 1);

    // Column definition and delimiter (EOF/OK)
    let _col_def_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let _after_columns = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // Row packet
    let row_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut row_payload = row_packet.payload.clone();
    let val_len = read_lenenc_int(&mut row_payload).unwrap() as usize;
    let val_bytes = row_payload.copy_to_bytes(val_len);
    let version_str = String::from_utf8(val_bytes.to_vec()).unwrap();
    assert_eq!(version_str, "5.7.99");

    // Final EOF/OK
    let _final_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn mysql_integration_version_comment_probe() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    // COM_QUERY "SELECT @@version_comment LIMIT 1"
    use bytes::BufMut;
    let mut qpayload = BytesMut::new();
    qpayload.put_u8(0x03); // COM_QUERY
    qpayload.put_slice(b"SELECT @@version_comment LIMIT 1");
    let qpacket = Packet::new(0, qpayload.freeze());
    stream.write_all(&qpacket.encode()).await?;
    stream.flush().await?;

    // Column count
    let col_count_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut payload = col_count_packet.payload.clone();
    let col_count = read_lenenc_int(&mut payload).unwrap();
    assert_eq!(col_count, 1);

    // Column definition + delimiter
    let _col_def_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let _after_columns = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // Row
    let row_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut row_payload = row_packet.payload.clone();
    let val_len = read_lenenc_int(&mut row_payload).unwrap() as usize;
    let val_bytes = row_payload.copy_to_bytes(val_len);
    let comment = String::from_utf8(val_bytes.to_vec()).unwrap();
    assert_eq!(comment, "Doris Rust FE PoC");

    // Final EOF/OK
    let _final_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn mysql_integration_set_statements_and_ping_quit() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    use bytes::BufMut;

    // Helper to send a COM_QUERY and assert OK response.
    async fn send_query_and_expect_ok(
        stream: &mut TcpStream,
        buf: &mut BytesMut,
        sql: &str,
    ) -> Result<()> {
        let mut payload = BytesMut::new();
        payload.put_u8(0x03); // COM_QUERY
        payload.put_slice(sql.as_bytes());
        let pkt = Packet::new(0, payload.freeze());
        stream.write_all(&pkt.encode()).await?;
        stream.flush().await?;

        let ok = read_packet_with_timeout(stream, buf).await?;
        assert!(!ok.payload.is_empty());
        assert_eq!(ok.payload[0], 0x00, "expected OK packet for '{}'", sql);
        Ok(())
    }

    // SET statements commonly used by clients.
    send_query_and_expect_ok(&mut stream, &mut buf, "SET NAMES utf8").await?;
    send_query_and_expect_ok(&mut stream, &mut buf, "SET autocommit=1").await?;
    send_query_and_expect_ok(&mut stream, &mut buf, "SET sql_mode='STRICT_TRANS_TABLES'").await?;

    // COM_PING (0x0e) should return OK.
    let mut ping_payload = BytesMut::new();
    ping_payload.put_u8(0x0e);
    let ping_packet = Packet::new(0, ping_payload.freeze());
    stream.write_all(&ping_packet.encode()).await?;
    stream.flush().await?;

    let ping_ok = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!ping_ok.payload.is_empty());
    assert_eq!(ping_ok.payload[0], 0x00, "PING should return OK");

    // COM_QUIT (0x01) should close the connection.
    let mut quit_payload = BytesMut::new();
    quit_payload.put_u8(0x01);
    let quit_packet = Packet::new(0, quit_payload.freeze());
    stream.write_all(&quit_packet.encode()).await?;
    stream.flush().await?;

    // Next read should fail with ConnectionClosed.
    let err = read_packet_with_timeout(&mut stream, &mut buf).await.unwrap_err();
    match err {
        doris_rust_fe::DorisError::ConnectionClosed => {}
        other => panic!("expected ConnectionClosed after QUIT, got {:?}", other),
    }

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn mysql_integration_syntax_error_returns_err_packet() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    use bytes::BufMut;

    // Intentionally invalid SQL to trigger a parse/execute error.
    let mut payload = BytesMut::new();
    payload.put_u8(0x03); // COM_QUERY
    payload.put_slice(b"SELECT * FROM"); // missing table name
    let pkt = Packet::new(0, payload.freeze());
    stream.write_all(&pkt.encode()).await?;
    stream.flush().await?;

    let err_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!err_packet.payload.is_empty());
    assert_eq!(err_packet.payload[0], 0xff, "expected ERR packet header");

    // Error code is next 2 bytes (little-endian).
    let code = u16::from_le_bytes([err_packet.payload[1], err_packet.payload[2]]);
    assert_eq!(code, 1064, "expected MySQL syntax error code 1064");

    // SQL state marker and state should follow (for parse errors, 42000).
    assert_eq!(err_packet.payload[3], b'#');

    let sql_state = &err_packet.payload[4..9];
    assert_eq!(sql_state, b"42000");

    // Error message should be non-empty and include "Query failed".
    let msg_bytes = &err_packet.payload[9..];
    let msg = String::from_utf8(msg_bytes.to_vec()).unwrap();
    assert!(
        msg.contains("Query failed"),
        "expected error message to contain 'Query failed', got: {}",
        msg
    );

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

/// Unknown table should return ER_NO_SUCH_TABLE (1146) with SQLSTATE 42S02.
#[tokio::test]
async fn mysql_integration_unknown_table_returns_1146() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    use bytes::BufMut;

    // COM_QUERY with a non-existent table.
    let mut payload = BytesMut::new();
    payload.put_u8(0x03); // COM_QUERY
    payload.put_slice(b\"SELECT * FROM no_such_table\");
    let pkt = Packet::new(0, payload.freeze());
    stream.write_all(&pkt.encode()).await?;
    stream.flush().await?;

    let err_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!err_packet.payload.is_empty());
    assert_eq!(err_packet.payload[0], 0xff, \"expected ERR packet header\");

    let code = u16::from_le_bytes([err_packet.payload[1], err_packet.payload[2]]);
    assert_eq!(code, 1146, \"expected ER_NO_SUCH_TABLE (1146) for unknown table\");

    assert_eq!(err_packet.payload[3], b'#');
    let sql_state = &err_packet.payload[4..9];
    assert_eq!(sql_state, b\"42S02\", \"expected SQLSTATE 42S02 for unknown table\");

    let msg_bytes = &err_packet.payload[9..];
    let msg = String::from_utf8(msg_bytes.to_vec()).unwrap();
    assert!(
        msg.contains(\"Query failed\") && msg.contains(\"no_such_table\"),
        \"expected error message to mention 'Query failed' and table name, got: {}\", msg
    );

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

/// Unknown database in USE should return ER_BAD_DB_ERROR (1049).
#[tokio::test]
async fn mysql_integration_unknown_database_returns_1049() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    use bytes::BufMut;

    let mut payload = BytesMut::new();
    payload.put_u8(0x03); // COM_QUERY
    payload.put_slice(b\"USE no_such_db\");
    let pkt = Packet::new(0, payload.freeze());
    stream.write_all(&pkt.encode()).await?;
    stream.flush().await?;

    let err_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!err_packet.payload.is_empty());
    assert_eq!(err_packet.payload[0], 0xff, \"expected ERR packet header\");

    let code = u16::from_le_bytes([err_packet.payload[1], err_packet.payload[2]]);
    assert_eq!(code, 1049, \"expected ER_BAD_DB_ERROR (1049) for unknown database\");

    assert_eq!(err_packet.payload[3], b'#');
    let sql_state = &err_packet.payload[4..9];
    // ER_BAD_DB_ERROR is grouped with 42000 in opensrv's errorcodes.
    assert_eq!(sql_state, b\"42000\", \"expected SQLSTATE 42000 for bad database\");

    let msg_bytes = &err_packet.payload[9..];
    let msg = String::from_utf8(msg_bytes.to_vec()).unwrap();
    assert!(
        msg.contains(\"Unknown database 'no_such_db'\"),
        \"expected error message to mention unknown database, got: {}\", msg
    );

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn mysql_integration_show_databases_and_tables() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    // COM_QUERY: SHOW DATABASES
    use bytes::BufMut;
    let mut qpayload = BytesMut::new();
    qpayload.put_u8(0x03); // COM_QUERY
    qpayload.put_slice(b"SHOW DATABASES");
    let qpacket = Packet::new(0, qpayload.freeze());
    stream.write_all(&qpacket.encode()).await?;
    stream.flush().await?;

    // Column count + column definition + EOF
    let _col_count_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let _col_def_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let _after_columns = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // Rows: collect database names until EOF (payload[0] == 0xfe and small len)
    let mut dbs = Vec::new();
    loop {
        let pkt = read_packet_with_timeout(&mut stream, &mut buf).await?;
        if pkt.payload.is_empty() {
            break;
        }
        if pkt.payload[0] == 0xfe && pkt.payload.len() <= 5 {
            // EOF
            break;
        }

        let mut row_payload = pkt.payload.clone();
        let len = read_lenenc_int(&mut row_payload).unwrap() as usize;
        let val_bytes = row_payload.copy_to_bytes(len);
        let name = String::from_utf8(val_bytes.to_vec()).unwrap();
        dbs.push(name);
    }

    assert!(dbs.contains(&"tpch".to_string()));

    // COM_QUERY: USE tpch
    let mut upayload = BytesMut::new();
    upayload.put_u8(0x03);
    upayload.put_slice(b"USE tpch");
    let upacket = Packet::new(0, upayload.freeze());
    stream.write_all(&upacket.encode()).await?;
    stream.flush().await?;

    // Read OK for USE
    let _use_ok = read_packet_with_timeout(&mut stream, &mut buf).await?;

    // COM_QUERY: SHOW TABLES
    let mut tpayload = BytesMut::new();
    tpayload.put_u8(0x03);
    tpayload.put_slice(b"SHOW TABLES");
    let tpacket = Packet::new(0, tpayload.freeze());
    stream.write_all(&tpacket.encode()).await?;
    stream.flush().await?;

    let _t_col_count = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let _t_col_def = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let _t_after_cols = read_packet_with_timeout(&mut stream, &mut buf).await?;

    let mut tables = Vec::new();
    loop {
        let pkt = read_packet_with_timeout(&mut stream, &mut buf).await?;
        if pkt.payload.is_empty() {
            break;
        }
        if pkt.payload[0] == 0xfe && pkt.payload.len() <= 5 {
            break;
        }

        let mut row_payload = pkt.payload.clone();
        let len = read_lenenc_int(&mut row_payload).unwrap() as usize;
        let val_bytes = row_payload.copy_to_bytes(len);
        let name = String::from_utf8(val_bytes.to_vec()).unwrap();
        tables.push(name);
    }

    // At least TPC-H tables should be present
    assert!(tables.contains(&"lineitem".to_string()));
    assert!(tables.contains(&"orders".to_string()));

    drop(stream);
    let _ = server_handle.await;

    Ok(())
}

#[tokio::test]
async fn mysql_integration_com_init_db_changes_database() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    use bytes::BufMut;

    // Send COM_INIT_DB with "tpch"
    let mut payload = BytesMut::new();
    payload.put_u8(0x02); // COM_INIT_DB
    payload.put_slice(b"tpch");
    let pkt = Packet::new(0, payload.freeze());
    stream.write_all(&pkt.encode()).await?;
    stream.flush().await?;

    // Expect OK packet
    let ok = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!ok.payload.is_empty());
    assert_eq!(ok.payload[0], 0x00);

    // Verify that SHOW TABLES uses the new database implicitly
    let mut qpayload = BytesMut::new();
    qpayload.put_u8(0x03); // COM_QUERY
    qpayload.put_slice(b"SHOW TABLES");
    let qpacket = Packet::new(0, qpayload.freeze());
    stream.write_all(&qpacket.encode()).await?;
    stream.flush().await?;

    let _col_count = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let _col_def = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let _after_cols = read_packet_with_timeout(&mut stream, &mut buf).await?;

    let mut tables = Vec::new();
    loop {
        let pkt = read_packet_with_timeout(&mut stream, &mut buf).await?;
        if pkt.payload.is_empty() {
            break;
        }
        if pkt.payload[0] == 0xfe && pkt.payload.len() <= 5 {
            break;
        }

        let mut row_payload = pkt.payload.clone();
        let len = read_lenenc_int(&mut row_payload).unwrap() as usize;
        let val_bytes = row_payload.copy_to_bytes(len);
        let name = String::from_utf8(val_bytes.to_vec()).unwrap();
        tables.push(name);
    }

    assert!(tables.contains(&"lineitem".to_string()));

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn mysql_integration_com_field_list_empty_result() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    use bytes::BufMut;

    // COM_FIELD_LIST (0x04) with arbitrary table and field pattern.
    let mut payload = BytesMut::new();
    payload.put_u8(0x04);
    payload.put_slice(b"some_table");
    payload.put_u8(0); // null terminator
    payload.put_slice(b"%");
    let pkt = Packet::new(0, payload.freeze());
    stream.write_all(&pkt.encode()).await?;
    stream.flush().await?;

    // Implementation currently returns an empty result set:
    // column count = 0, and then EOF/OK.
    let col_count = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut payload = col_count.payload.clone();
    let count = read_lenenc_int(&mut payload).unwrap();
    assert_eq!(count, 0);

    // Next packet should be EOF or OK terminator.
    let terminator = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!terminator.payload.is_empty());
    let header = terminator.payload[0];
    assert!(
        header == 0xfe || header == 0x00,
        "expected EOF(0xfe) or OK(0x00), got 0x{:02x}",
        header
    );

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn mysql_integration_prepared_statement_commands_return_error() -> Result<()> {
    let (mut stream, server_handle) = setup_mysql_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    use bytes::BufMut;

    // Helper to send a single-byte command and expect 1047 error.
    async fn send_cmd_and_expect_1047(
        stream: &mut TcpStream,
        buf: &mut BytesMut,
        cmd: u8,
    ) -> Result<()> {
        let mut payload = BytesMut::new();
        payload.put_u8(cmd);
        let pkt = Packet::new(0, payload.freeze());
        stream.write_all(&pkt.encode()).await?;
        stream.flush().await?;

        let resp = read_packet_with_timeout(stream, buf).await?;
        assert!(!resp.payload.is_empty());
        assert_eq!(resp.payload[0], 0xff, "expected ERR header for cmd 0x{:02x}", cmd);

        let code = u16::from_le_bytes([resp.payload[1], resp.payload[2]]);
        assert_eq!(code, 1047, "expected ER_UNKNOWN_COM_ERROR for cmd 0x{:02x}", cmd);
        Ok(())
    }

    // COM_STMT_PREPARE (0x16)
    send_cmd_and_expect_1047(&mut stream, &mut buf, 0x16).await?;
    // COM_STMT_EXECUTE (0x17)
    send_cmd_and_expect_1047(&mut stream, &mut buf, 0x17).await?;
    // COM_STMT_CLOSE (0x19)
    send_cmd_and_expect_1047(&mut stream, &mut buf, 0x19).await?;

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}
