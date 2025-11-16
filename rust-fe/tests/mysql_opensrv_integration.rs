#![cfg(feature = "opensrv")]

use std::sync::Arc;

use bytes::{Buf, BytesMut};
use doris_rust_fe::{QueryExecutor, Result};
use doris_rust_fe::be::BackendClientPool;
use doris_rust_fe::mysql::{
    DorisShim,
    Packet,
    read_lenenc_int,
    write_null_terminated_str,
    CLIENT_CONNECT_WITH_DB,
    CLIENT_DEPRECATE_EOF,
    CLIENT_PROTOCOL_41,
    CLIENT_SECURE_CONNECTION,
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
            "read_packet timed out in mysql_opensrv_integration test".to_string(),
        )),
    }
}

/// Build a minimal Handshake Response packet payload suitable for opensrv.
fn build_handshake_response_with_flags(user: &str, db: Option<&str>, extra_flags: u32) -> bytes::Bytes {
    use bytes::BufMut;

    let mut buf = BytesMut::new();

    let mut flags = CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION | extra_flags;
    if db.is_some() {
        flags |= CLIENT_CONNECT_WITH_DB;
    }

    buf.put_u32_le(flags);
    buf.put_u32_le(1024 * 1024 * 16); // max_packet_size
    buf.put_u8(UTF8MB4_GENERAL_CI);   // character set
    buf.put_bytes(0, 23);            // reserved

    write_null_terminated_str(&mut buf, user);

    // zero-length auth response
    buf.put_u8(0);

    if let Some(dbname) = db {
        write_null_terminated_str(&mut buf, dbname);
    }

    buf.freeze()
}

fn build_handshake_response(user: &str, db: Option<&str>) -> bytes::Bytes {
    build_handshake_response_with_flags(user, db, 0)
}

async fn start_single_connection_opensrv_server(
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

async fn setup_opensrv_connection() -> Result<(TcpStream, tokio::task::JoinHandle<()>)> {
    let executor = Arc::new(QueryExecutor::with_datafusion(16, 4).await);
    let be_pool = Arc::new(BackendClientPool::new(Vec::new()));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server_executor = executor.clone();
    let server_be_pool = be_pool.clone();
    let server_handle = tokio::spawn(async move {
        let _ = start_single_connection_opensrv_server(listener, server_executor, server_be_pool).await;
    });

    let mut stream = TcpStream::connect(addr).await?;

    let mut buf = BytesMut::with_capacity(1024);
    let _handshake_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

    let response_payload = build_handshake_response("root", Some("tpch"));
    let response_packet = Packet::new(1, response_payload);
    stream.write_all(&response_packet.encode()).await?;
    stream.flush().await?;

    let ok_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!ok_packet.payload.is_empty());
    assert_eq!(ok_packet.payload[0], 0x00);

    Ok((stream, server_handle))
}

async fn setup_opensrv_connection_with_deprecate_eof() -> Result<(TcpStream, tokio::task::JoinHandle<()>)> {
    let executor = Arc::new(QueryExecutor::with_datafusion(16, 4).await);
    let be_pool = Arc::new(BackendClientPool::new(Vec::new()));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    let server_executor = executor.clone();
    let server_be_pool = be_pool.clone();
    let server_handle = tokio::spawn(async move {
        let _ = start_single_connection_opensrv_server(listener, server_executor, server_be_pool).await;
    });

    let mut stream = TcpStream::connect(addr).await?;

    let mut buf = BytesMut::with_capacity(1024);
    let _handshake_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

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
async fn opensrv_select_1() -> Result<()> {
    let (mut stream, server_handle) = setup_opensrv_connection().await?;
    let mut buf = BytesMut::with_capacity(1024);

    use bytes::BufMut;
    let mut qpayload = BytesMut::new();
    qpayload.put_u8(0x03); // COM_QUERY
    qpayload.put_slice(b"SELECT 1");
    let qpacket = Packet::new(0, qpayload.freeze());
    stream.write_all(&qpacket.encode()).await?;
    stream.flush().await?;

    let col_count_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut payload = col_count_packet.payload.clone();
    let col_count = read_lenenc_int(&mut payload).unwrap();
    assert_eq!(col_count, 1);

    let _col_def_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let _after_columns = read_packet_with_timeout(&mut stream, &mut buf).await?;

    let row_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut row_payload = row_packet.payload.clone();
    let val_len = read_lenenc_int(&mut row_payload).unwrap() as usize;
    let val_bytes = row_payload.copy_to_bytes(val_len);
    assert_eq!(&val_bytes[..], b"1");

    let _final_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

#[tokio::test]
async fn opensrv_select_1_with_deprecate_eof() -> Result<()> {
    let (mut stream, server_handle) = setup_opensrv_connection_with_deprecate_eof().await?;
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

    // After columns: with CLIENT_DEPRECATE_EOF, this should be an OK, not EOF.
    let after_cols = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!after_cols.payload.is_empty());
    assert_eq!(after_cols.payload[0], 0x00, "expected OK after columns");

    // Row and final terminator (also OK under deprecate EOF)
    let row_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    let mut row_payload = row_packet.payload.clone();
    let val_len = read_lenenc_int(&mut row_payload).unwrap() as usize;
    let val_bytes = row_payload.copy_to_bytes(val_len);
    assert_eq!(&val_bytes[..], b"1");

    let final_packet = read_packet_with_timeout(&mut stream, &mut buf).await?;
    assert!(!final_packet.payload.is_empty());
    assert_eq!(final_packet.payload[0], 0x00, "expected OK as final terminator");

    drop(stream);
    let _ = server_handle.await;
    Ok(())
}

