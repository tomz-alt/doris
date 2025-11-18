// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Java FE Specification Tests
//!
//! CLAUDE.md Principle #2: "Use Java FE behavior as the specification"
//!
//! These tests query the running Java FE to document expected behavior
//! that Rust FE must match exactly. Each test is a specification requirement.

use std::io::{Read, Write};
use std::net::TcpStream;

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  Java FE Specification Suite - CLAUDE.md Principle #2     ║");
    println!("║  \"Use Java FE behavior as the specification\"              ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    spec_database_operations();
    spec_metadata_queries();
    spec_aggregation_queries();
    spec_group_by_queries();
    spec_order_limit_queries();
    spec_where_clause();
    spec_tpch_query_1();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  All Specifications Documented ✅                          ║");
    println!("║  Rust FE must match this behavior exactly                 ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
}

/// Java FE Specification: Database operations
fn spec_database_operations() {
    println!("\n=== Java FE Specification: Database Operations ===\n");

    let mut conn = connect_java_fe().expect("Java FE not running on 9030");

    // SPEC: SHOW DATABASES returns list of databases
    println!("SPEC: SHOW DATABASES");
    let result = execute_select(&mut conn, "SHOW DATABASES").unwrap();
    println!("✓ Returns database list: {}", result);

    // SPEC: USE <database> switches context
    println!("\nSPEC: USE tpch");
    execute_query(&mut conn, "USE tpch").unwrap();
    println!("✓ Database context switched");

    // SPEC: SHOW TABLES returns list of tables in current database
    println!("\nSPEC: SHOW TABLES");
    let result = execute_select(&mut conn, "SHOW TABLES").unwrap();
    println!("✓ Returns table list: {}", result);

    println!("\n=== Specification Documented ===\n");
}

/// Java FE Specification: Metadata queries
fn spec_metadata_queries() {
    println!("\n=== Java FE Specification: Metadata Queries ===\n");

    let mut conn = connect_java_fe().expect("Java FE not running");
    execute_query(&mut conn, "USE tpch").unwrap();

    // SPEC: SHOW CREATE TABLE returns table DDL
    println!("SPEC: SHOW CREATE TABLE lineitem");
    let result = execute_select(&mut conn, "SHOW CREATE TABLE lineitem").unwrap();
    println!("✓ Returns DDL: {}", result);

    // SPEC: DESC table returns column information
    println!("\nSPEC: DESC lineitem");
    let result = execute_select(&mut conn, "DESC lineitem").unwrap();
    println!("✓ Returns column info: {}", result);

    // SPEC: SHOW COLUMNS returns detailed column metadata
    println!("\nSPEC: SHOW COLUMNS FROM lineitem");
    let result = execute_select(&mut conn, "SHOW COLUMNS FROM lineitem").unwrap();
    println!("✓ Returns column metadata: {}", result);

    println!("\n=== Specification Documented ===\n");
}

/// Java FE Specification: Aggregation queries


fn spec_aggregation_queries() {
    println!("\n=== Java FE Specification: Aggregation Queries ===\n");

    let mut conn = connect_java_fe().expect("Java FE not running");
    execute_query(&mut conn, "USE tpch").unwrap();

    // SPEC: COUNT(*) aggregation
    println!("SPEC: SELECT COUNT(*) FROM lineitem");
    let result = execute_select(&mut conn, "SELECT COUNT(*) FROM lineitem").unwrap();
    println!("✓ COUNT(*): {}", result);

    // SPEC: SUM aggregation
    println!("\nSPEC: SELECT SUM(l_quantity) FROM lineitem");
    let result = execute_select(&mut conn, "SELECT SUM(l_quantity) FROM lineitem").unwrap();
    println!("✓ SUM(): {}", result);

    // SPEC: AVG aggregation
    println!("\nSPEC: SELECT AVG(l_quantity) FROM lineitem");
    let result = execute_select(&mut conn, "SELECT AVG(l_quantity) FROM lineitem").unwrap();
    println!("✓ AVG(): {}", result);

    // SPEC: MIN/MAX aggregation
    println!("\nSPEC: SELECT MIN(l_quantity), MAX(l_quantity) FROM lineitem");
    let result = execute_select(&mut conn, "SELECT MIN(l_quantity), MAX(l_quantity) FROM lineitem").unwrap();
    println!("✓ MIN/MAX: {}", result);

    println!("\n=== Specification Documented ===\n");
}

/// Java FE Specification: GROUP BY queries


fn spec_group_by_queries() {
    println!("\n=== Java FE Specification: GROUP BY Queries ===\n");

    let mut conn = connect_java_fe().expect("Java FE not running");
    execute_query(&mut conn, "USE tpch").unwrap();

    // SPEC: Simple GROUP BY
    println!("SPEC: SELECT l_orderkey, COUNT(*) FROM lineitem GROUP BY l_orderkey");
    let result = execute_select(&mut conn,
        "SELECT l_orderkey, COUNT(*) FROM lineitem GROUP BY l_orderkey").unwrap();
    println!("✓ GROUP BY: {}", result);

    // SPEC: GROUP BY with multiple columns
    println!("\nSPEC: GROUP BY l_orderkey, l_returnflag");
    let result = execute_select(&mut conn,
        "SELECT l_orderkey, l_returnflag, COUNT(*) FROM lineitem GROUP BY l_orderkey, l_returnflag").unwrap();
    println!("✓ Multi-column GROUP BY: {}", result);

    // SPEC: GROUP BY with HAVING
    println!("\nSPEC: GROUP BY with HAVING COUNT(*) > 1");
    let result = execute_select(&mut conn,
        "SELECT l_orderkey, COUNT(*) FROM lineitem GROUP BY l_orderkey HAVING COUNT(*) > 1").unwrap();
    println!("✓ HAVING clause: {}", result);

    println!("\n=== Specification Documented ===\n");
}

/// Java FE Specification: ORDER BY and LIMIT


fn spec_order_limit_queries() {
    println!("\n=== Java FE Specification: ORDER BY and LIMIT ===\n");

    let mut conn = connect_java_fe().expect("Java FE not running");
    execute_query(&mut conn, "USE tpch").unwrap();

    // SPEC: ORDER BY ASC
    println!("SPEC: SELECT l_orderkey FROM lineitem ORDER BY l_orderkey ASC");
    let result = execute_select(&mut conn,
        "SELECT l_orderkey FROM lineitem ORDER BY l_orderkey ASC LIMIT 2").unwrap();
    println!("✓ ORDER BY ASC: {}", result);

    // SPEC: ORDER BY DESC
    println!("\nSPEC: SELECT l_orderkey FROM lineitem ORDER BY l_orderkey DESC");
    let result = execute_select(&mut conn,
        "SELECT l_orderkey FROM lineitem ORDER BY l_orderkey DESC LIMIT 2").unwrap();
    println!("✓ ORDER BY DESC: {}", result);

    // SPEC: LIMIT without ORDER BY
    println!("\nSPEC: SELECT * FROM lineitem LIMIT 1");
    let result = execute_select(&mut conn, "SELECT * FROM lineitem LIMIT 1").unwrap();
    println!("✓ LIMIT: {}", result);

    println!("\n=== Specification Documented ===\n");
}

/// Java FE Specification: WHERE clause filtering


fn spec_where_clause() {
    println!("\n=== Java FE Specification: WHERE Clause ===\n");

    let mut conn = connect_java_fe().expect("Java FE not running");
    execute_query(&mut conn, "USE tpch").unwrap();

    // SPEC: Equality filter
    println!("SPEC: WHERE l_orderkey = 1");
    let result = execute_select(&mut conn,
        "SELECT COUNT(*) FROM lineitem WHERE l_orderkey = 1").unwrap();
    println!("✓ Equality: {}", result);

    // SPEC: Range filter
    println!("\nSPEC: WHERE l_quantity > 30");
    let result = execute_select(&mut conn,
        "SELECT COUNT(*) FROM lineitem WHERE l_quantity > 30").unwrap();
    println!("✓ Range: {}", result);

    // SPEC: Date filter
    println!("\nSPEC: WHERE l_shipdate <= '1998-12-01'");
    let result = execute_select(&mut conn,
        "SELECT COUNT(*) FROM lineitem WHERE l_shipdate <= '1998-12-01'").unwrap();
    println!("✓ Date filter: {}", result);

    // SPEC: String filter
    println!("\nSPEC: WHERE l_returnflag = 'N'");
    let result = execute_select(&mut conn,
        "SELECT COUNT(*) FROM lineitem WHERE l_returnflag = 'N'").unwrap();
    println!("✓ String filter: {}", result);

    println!("\n=== Specification Documented ===\n");
}

/// Java FE Specification: TPC-H Query 1 (simplified)


fn spec_tpch_query_1() {
    println!("\n=== Java FE Specification: TPC-H Query 1 ===\n");

    let mut conn = connect_java_fe().expect("Java FE not running");
    execute_query(&mut conn, "USE tpch").unwrap();

    let tpch_q1 = r#"
        SELECT
            l_returnflag,
            l_linestatus,
            COUNT(*) as count_order,
            SUM(l_quantity) as sum_qty,
            SUM(l_extendedprice) as sum_base_price,
            AVG(l_quantity) as avg_qty,
            AVG(l_extendedprice) as avg_price
        FROM lineitem
        WHERE l_shipdate <= '1998-12-01'
        GROUP BY l_returnflag, l_linestatus
        ORDER BY l_returnflag, l_linestatus
    "#;

    println!("SPEC: TPC-H Query 1 (Pricing Summary Report)");
    println!("Query:\n{}", tpch_q1);

    let result = execute_select(&mut conn, tpch_q1).unwrap();
    println!("✓ Result: {}", result);

    println!("\n=== TPC-H Q1 Specification Documented ===\n");
}

// Helper functions

fn connect_java_fe() -> Result<TcpStream, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:9030")?;

    // Handshake
    let (handshake, _) = read_packet_with_seq(&mut stream)?;
    let _ = parse_handshake(&handshake)?;

    // Authenticate
    let auth_packet = build_auth_packet("root", "")?;
    write_packet_with_seq(&mut stream, &auth_packet, 1)?;

    let (auth_response, _) = read_packet_with_seq(&mut stream)?;
    if auth_response[0] != 0x00 && auth_response[0] != 0xFE {
        return Err("Authentication failed".into());
    }

    Ok(stream)
}

fn execute_query(stream: &mut TcpStream, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let query_packet = build_query_packet(query);
    write_packet_with_seq(stream, &query_packet, 0)?;

    let (response, _) = read_packet_with_seq(stream)?;

    match response[0] {
        0x00 => Ok(()),
        0xFF => {
            let error_msg = parse_error_packet(&response)?;
            Err(format!("Query failed: {}", error_msg).into())
        }
        _ => {
            let _ = read_packet_with_seq(stream);
            Ok(())
        }
    }
}

fn execute_select(stream: &mut TcpStream, query: &str) -> Result<String, Box<dyn std::error::Error>> {
    let query_packet = build_query_packet(query);
    write_packet_with_seq(stream, &query_packet, 0)?;

    let (response, _) = read_packet_with_seq(stream)?;

    match response[0] {
        0xFF => {
            let error_msg = parse_error_packet(&response)?;
            Err(format!("Query failed: {}", error_msg).into())
        }
        _ => {
            let mut row_count = 0;
            let mut total_bytes = response.len();

            loop {
                match read_packet_with_seq(stream) {
                    Ok((pkt, _)) => {
                        if pkt.is_empty() || pkt[0] == 0xFE {
                            break;
                        }
                        row_count += 1;
                        total_bytes += pkt.len();
                    }
                    Err(_) => break,
                }
            }

            Ok(format!("{} rows, {} bytes", row_count, total_bytes))
        }
    }
}

fn read_packet_with_seq(stream: &mut TcpStream) -> Result<(Vec<u8>, u8), Box<dyn std::error::Error>> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header)?;
    let payload_len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
    let sequence = header[3];
    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;
    Ok((payload, sequence))
}

fn write_packet_with_seq(
    stream: &mut TcpStream,
    payload: &[u8],
    sequence: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let len = payload.len() as u32;
    let header = [
        (len & 0xFF) as u8,
        ((len >> 8) & 0xFF) as u8,
        ((len >> 16) & 0xFF) as u8,
        sequence,
    ];
    stream.write_all(&header)?;
    stream.write_all(payload)?;
    stream.flush()?;
    Ok(())
}

fn parse_handshake(packet: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let mut null_pos = 1;
    while null_pos < packet.len() && packet[null_pos] != 0 {
        null_pos += 1;
    }
    Ok(String::from_utf8_lossy(&packet[1..null_pos]).to_string())
}

fn build_auth_packet(username: &str, _password: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut packet = Vec::new();
    let capabilities: u32 = 0x0000_A285;
    packet.extend_from_slice(&capabilities.to_le_bytes());
    packet.extend_from_slice(&0x0100_0000u32.to_le_bytes());
    packet.push(0x21);
    packet.extend_from_slice(&[0u8; 23]);
    packet.extend_from_slice(username.as_bytes());
    packet.push(0x00);
    packet.push(0x00);
    Ok(packet)
}

fn build_query_packet(query: &str) -> Vec<u8> {
    let mut packet = Vec::new();
    packet.push(0x03);
    packet.extend_from_slice(query.as_bytes());
    packet
}

fn parse_error_packet(packet: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    if packet.len() < 3 || packet[0] != 0xFF {
        return Ok("Unknown error".to_string());
    }
    let msg_start = if packet.len() > 9 && packet[3] == b'#' { 9 } else { 3 };
    Ok(String::from_utf8_lossy(&packet[msg_start..]).to_string())
}
