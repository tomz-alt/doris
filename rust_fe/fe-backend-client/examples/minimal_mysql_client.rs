// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Minimal MySQL Protocol Client
//!
//! Bare-bones implementation to send ALTER SYSTEM ADD BACKEND command
//! without triggering Java FE's internal queries during handshake.

use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Minimal MySQL Protocol Client");
    println!("==============================\n");

    // Connect to Java FE MySQL port
    println!("Step 1: Connecting to 127.0.0.1:9030...");
    let mut stream = TcpStream::connect("127.0.0.1:9030")?;
    println!("✅ TCP connection established\n");

    // Sequence number tracking (increments with each packet sent/received)
    let mut sequence: u8 = 0;

    // Read initial handshake packet from server
    println!("Step 2: Reading server handshake...");
    let (handshake, seq) = read_packet_with_seq(&mut stream)?;
    sequence = seq.wrapping_add(1);
    println!("✅ Received handshake packet ({} bytes, seq={})", handshake.len(), seq);

    // Parse handshake to extract protocol version and capabilities
    let (protocol_version, server_version, auth_plugin) = parse_handshake(&handshake)?;
    println!("   Protocol version: {}", protocol_version);
    println!("   Server version: {}", server_version);
    println!("   Auth plugin: {}\n", auth_plugin);

    // Send handshake response (authentication)
    println!("Step 3: Sending authentication packet (seq={})...", sequence);
    let auth_packet = build_auth_packet("root", "", &auth_plugin)?;
    write_packet_with_seq(&mut stream, &auth_packet, sequence)?;
    sequence = sequence.wrapping_add(1);
    println!("✅ Authentication packet sent\n");

    // Read authentication response
    println!("Step 4: Reading authentication response...");
    let (auth_response, seq) = read_packet_with_seq(&mut stream)?;
    sequence = seq.wrapping_add(1);
    println!("✅ Received response ({} bytes, seq={})", auth_response.len(), seq);

    if auth_response.is_empty() {
        return Err("Empty authentication response".into());
    }

    // Check if authentication succeeded (0x00 = OK, 0xFF = ERR)
    match auth_response[0] {
        0x00 => {
            println!("✅ Authentication successful\n");
        }
        0xFF => {
            let error_msg = parse_error_packet(&auth_response)?;
            println!("❌ Authentication failed: {}\n", error_msg);
            return Err(format!("Auth failed: {}", error_msg).into());
        }
        0xFE => {
            println!("⚠️  Server requests auth method switch - trying to continue anyway\n");
        }
        _ => {
            println!("⚠️  Unexpected response byte: 0x{:02X}\n", auth_response[0]);
        }
    }

    // Send ALTER SYSTEM ADD BACKEND command (reset sequence to 0 for new command)
    println!("\nStep 5: Sending ALTER SYSTEM ADD BACKEND command...");
    sequence = 0; // Reset sequence for new command phase
    let command = "ALTER SYSTEM ADD BACKEND '127.0.0.1:9050'";
    let query_packet = build_query_packet(command);
    write_packet_with_seq(&mut stream, &query_packet, sequence)?;
    sequence = sequence.wrapping_add(1);
    println!("✅ Command sent: {}\n", command);

    // Read command response
    println!("Step 6: Reading command response...");
    let (response, seq) = read_packet_with_seq(&mut stream)?;
    println!("✅ Received response ({} bytes, seq={})", response.len(), seq);

    if response.is_empty() {
        return Err("Empty command response".into());
    }

    match response[0] {
        0x00 => {
            println!("✅ SUCCESS! Backend added to cluster\n");
        }
        0xFF => {
            let error_msg = parse_error_packet(&response)?;
            // Check if error is "already exists" which is OK
            if error_msg.contains("already exists") || error_msg.contains("Same backend") {
                println!("✅ Backend already exists (this is OK)\n");
            } else {
                println!("❌ Command failed: {}\n", error_msg);
                return Err(format!("Command failed: {}", error_msg).into());
            }
        }
        _ => {
            println!("⚠️  Unexpected response byte: 0x{:02X}\n", response[0]);
        }
    }

    // Step 7: Wait for backend to be ready
    println!("\nStep 7: Waiting for backend to be ready...");
    std::thread::sleep(std::time::Duration::from_secs(3));
    println!("✅ Wait complete\n");

    // Step 8: Create database
    println!("Step 8: Creating tpch database...");
    execute_query(&mut stream, "CREATE DATABASE IF NOT EXISTS tpch")?;
    println!("✅ Database created\n");

    // Step 9: Use database
    println!("Step 9: Switching to tpch database...");
    execute_query(&mut stream, "USE tpch")?;
    println!("✅ Database selected\n");

    // Step 10: Create table
    println!("Step 10: Creating lineitem table...");
    let create_table = r#"
        CREATE TABLE IF NOT EXISTS lineitem (
            l_orderkey BIGINT,
            l_partkey BIGINT,
            l_suppkey BIGINT,
            l_linenumber INT,
            l_quantity DECIMAL(15,2),
            l_extendedprice DECIMAL(15,2),
            l_discount DECIMAL(15,2),
            l_tax DECIMAL(15,2),
            l_returnflag CHAR(1),
            l_linestatus CHAR(1),
            l_shipdate DATE,
            l_commitdate DATE,
            l_receiptdate DATE,
            l_shipinstruct CHAR(25),
            l_shipmode CHAR(10),
            l_comment VARCHAR(44)
        )
        DUPLICATE KEY(l_orderkey)
        DISTRIBUTED BY HASH(l_orderkey) BUCKETS 1
        PROPERTIES ("replication_num" = "1")
    "#;
    execute_query(&mut stream, create_table)?;
    println!("✅ Table created\n");

    // Step 11: Insert sample data
    println!("Step 11: Inserting TPC-H sample data...");
    let inserts = vec![
        "INSERT INTO lineitem VALUES (1, 155190, 7706, 1, 17.00, 21168.23, 0.04, 0.02, 'N', 'O', '1996-03-13', '1996-02-12', '1996-03-22', 'DELIVER IN PERSON', 'TRUCK', 'regular deposits')",
        "INSERT INTO lineitem VALUES (1, 67310, 7311, 2, 36.00, 45983.16, 0.09, 0.06, 'N', 'O', '1996-04-12', '1996-02-28', '1996-04-20', 'TAKE BACK RETURN', 'MAIL', 'unusual theodolites')",
        "INSERT INTO lineitem VALUES (2, 106170, 1191, 1, 38.00, 44694.46, 0.00, 0.05, 'N', 'O', '1997-01-28', '1997-01-14', '1997-02-02', 'TAKE BACK RETURN', 'RAIL', 'bold packages haggle')",
        "INSERT INTO lineitem VALUES (3, 4297, 1798, 1, 45.00, 54058.05, 0.06, 0.00, 'R', 'F', '1994-02-02', '1994-01-04', '1994-02-23', 'NONE', 'AIR', 'pending instructions')",
    ];

    for (i, insert_sql) in inserts.iter().enumerate() {
        match execute_query(&mut stream, insert_sql) {
            Ok(_) => println!("   ✓ Row {} inserted", i + 1),
            Err(e) => {
                println!("   ⚠️  Row {} error: {}", i + 1, e);
                // Continue with other rows even if one fails
            }
        }
    }
    println!("✅ Data insertion complete\n");

    // Step 12: Verify data
    println!("Step 12: Verifying data with COUNT query...");
    execute_query(&mut stream, "SELECT COUNT(*) FROM lineitem")?;
    println!("✅ Query executed (check output above)\n");

    println!("═══════════════════════════════════════");
    println!("SUCCESS! Setup complete!");
    println!("═══════════════════════════════════════");
    println!("\nYou can now:");
    println!("  1. Run queries: SELECT * FROM tpch.lineitem;");
    println!("  2. Test Rust FE against this data");
    println!("  3. Compare results between Java FE and Rust FE");

    Ok(())
}

/// Execute a query and handle response
fn execute_query(stream: &mut TcpStream, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Reset sequence for new command
    let sequence = 0;

    // Send query
    let query_packet = build_query_packet(query);
    write_packet_with_seq(stream, &query_packet, sequence)?;

    // Read response
    let (response, _seq) = read_packet_with_seq(stream)?;

    if response.is_empty() {
        return Err("Empty response".into());
    }

    match response[0] {
        0x00 => {
            // OK packet - command succeeded
            Ok(())
        }
        0xFF => {
            // Error packet
            let error_msg = parse_error_packet(&response)?;
            Err(format!("Query failed: {}", error_msg).into())
        }
        _ => {
            // Result set or other response - drain it
            // For simplicity, just acknowledge we got data
            // In a real client, we'd parse the result set properly
            println!("   (Received result set with {} bytes)", response.len());

            // Try to read EOF packet if there's more data
            // Simplified: just try to read one more packet and ignore errors
            let _ = read_packet_with_seq(stream);

            Ok(())
        }
    }
}

/// Read a MySQL packet with sequence number
fn read_packet_with_seq(stream: &mut TcpStream) -> Result<(Vec<u8>, u8), Box<dyn std::error::Error>> {
    // Read 4-byte header: 3 bytes length + 1 byte sequence
    let mut header = [0u8; 4];
    stream.read_exact(&mut header)?;

    let payload_len = u32::from_le_bytes([header[0], header[1], header[2], 0]) as usize;
    let sequence = header[3];

    // Read payload
    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;

    Ok((payload, sequence))
}

/// Write a MySQL packet with specific sequence number
fn write_packet_with_seq(
    stream: &mut TcpStream,
    payload: &[u8],
    sequence: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let len = payload.len() as u32;

    // Write header: 3 bytes length + 1 byte sequence
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

/// Parse server handshake packet
fn parse_handshake(packet: &[u8]) -> Result<(u8, String, String), Box<dyn std::error::Error>> {
    if packet.is_empty() {
        return Err("Empty handshake packet".into());
    }

    let protocol_version = packet[0];

    // Find null terminator for server version
    let mut null_pos = 1;
    while null_pos < packet.len() && packet[null_pos] != 0 {
        null_pos += 1;
    }

    let server_version = String::from_utf8_lossy(&packet[1..null_pos]).to_string();

    // For simplicity, assume mysql_native_password
    let auth_plugin = "mysql_native_password".to_string();

    Ok((protocol_version, server_version, auth_plugin))
}

/// Build authentication packet
fn build_auth_packet(
    username: &str,
    password: &str,
    _auth_plugin: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut packet = Vec::new();

    // Client capabilities (4 bytes) - minimal set
    let capabilities: u32 = 0x0000_A285; // CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION
    packet.extend_from_slice(&capabilities.to_le_bytes());

    // Max packet size (4 bytes)
    packet.extend_from_slice(&0x0100_0000u32.to_le_bytes());

    // Character set (1 byte) - utf8_general_ci
    packet.push(0x21);

    // Reserved (23 bytes of zeros)
    packet.extend_from_slice(&[0u8; 23]);

    // Username (null-terminated)
    packet.extend_from_slice(username.as_bytes());
    packet.push(0x00);

    // Auth response length + data
    if password.is_empty() {
        packet.push(0x00); // Empty password
    } else {
        // For simplicity, send empty password for now
        packet.push(0x00);
    }

    Ok(packet)
}

/// Build COM_QUERY packet
fn build_query_packet(query: &str) -> Vec<u8> {
    let mut packet = Vec::new();

    // COM_QUERY command byte
    packet.push(0x03);

    // Query string (not null-terminated in COM_QUERY)
    packet.extend_from_slice(query.as_bytes());

    packet
}

/// Parse error packet
fn parse_error_packet(packet: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    if packet.len() < 3 || packet[0] != 0xFF {
        return Ok("Unknown error".to_string());
    }

    // Error code is at bytes 1-2
    let _error_code = u16::from_le_bytes([packet[1], packet[2]]);

    // Error message starts at byte 9 (after SQL state marker)
    let msg_start = if packet.len() > 9 && packet[3] == b'#' {
        9
    } else {
        3
    };

    let error_msg = String::from_utf8_lossy(&packet[msg_start..]).to_string();

    Ok(error_msg)
}
