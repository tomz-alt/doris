// Discover actual tablet IDs for a table from Java FE metadata
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════");
    println!("  Discovering Tablet Metadata");
    println!("═══════════════════════════════════════\n");

    let mut stream = TcpStream::connect("127.0.0.1:9030")?;

    // Auth
    let (_, s) = read_packet(&mut stream)?;
    write_packet(&mut stream, &build_auth(), s.wrapping_add(1))?;
    let (auth_resp, _) = read_packet(&mut stream)?;
    if auth_resp[0] == 0xFF {
        return Err("Auth failed".into());
    }
    println!("✅ Authenticated\n");

    // Try multiple approaches to discover tablet metadata

    println!("Approach 1: SHOW PARTITIONS FROM tpch.lineitem");
    println!("─────────────────────────────────────────────────────────");
    match execute_select(&mut stream, "SHOW PARTITIONS FROM tpch.lineitem") {
        Ok(rows) => {
            if !rows.is_empty() {
                println!("  ✓ Found {} partition(s)", rows.len());
                for (i, row) in rows.iter().enumerate() {
                    println!("\nPartition {}:", i + 1);
                    for (j, field) in row.iter().enumerate() {
                        println!("  [{}]: {}", j, field);
                    }
                }
            } else {
                println!("  ⚠ No partitions found");
            }
        }
        Err(e) => {
            println!("  ✗ Error: {}", e);
        }
    }
    println!();

    println!("Approach 2: SHOW CREATE TABLE tpch.lineitem");
    println!("─────────────────────────────────────────────────────────");
    match execute_select(&mut stream, "SHOW CREATE TABLE tpch.lineitem") {
        Ok(rows) => {
            if !rows.is_empty() {
                println!("  ✓ Table definition:");
                for row in &rows {
                    for field in row {
                        println!("{}", field);
                    }
                }
            }
        }
        Err(e) => {
            println!("  ✗ Error: {}", e);
        }
    }
    println!();

    println!("Approach 3: SHOW PROC '/dbs/tpch'");
    println!("─────────────────────────────────────────────────────────");
    match execute_select(&mut stream, "SHOW PROC '/dbs/tpch'") {
        Ok(rows) => {
            if !rows.is_empty() {
                println!("  ✓ Database info:");
                for (i, row) in rows.iter().enumerate() {
                    println!("\nRow {}:", i + 1);
                    for (j, field) in row.iter().enumerate() {
                        println!("  [{}]: {}", j, field);
                    }
                }
            }
        }
        Err(e) => {
            println!("  ✗ Error: {}", e);
        }
    }
    println!();

    println!("Approach 4: ADMIN SHOW REPLICA STATUS FROM tpch.lineitem");
    println!("─────────────────────────────────────────────────────────");
    match execute_select(&mut stream, "ADMIN SHOW REPLICA STATUS FROM tpch.lineitem") {
        Ok(rows) => {
            if !rows.is_empty() {
                println!("  ✓ Found {} replica(s)", rows.len());
                for (i, row) in rows.iter().enumerate() {
                    println!("\nReplica {}:", i + 1);
                    for (j, field) in row.iter().enumerate() {
                        println!("  [{}]: {}", j, field);
                    }
                }
            }
        }
        Err(e) => {
            println!("  ✗ Error: {}", e);
        }
    }

    println!("\n═══════════════════════════════════════");
    println!("Discovery complete!");
    println!("═══════════════════════════════════════");

    Ok(())
}

fn execute_select(stream: &mut TcpStream, query: &str) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    write_packet(stream, &build_query(query), 0)?;
    let (resp, _) = read_packet(stream)?;

    if resp[0] == 0xFF {
        let msg = String::from_utf8_lossy(&resp[9..]);
        return Err(format!("{}", msg).into());
    }

    let num_cols = resp[0] as usize;

    // Read column definitions
    for _ in 0..num_cols {
        read_packet(stream)?;
    }

    // Read EOF after column defs
    read_packet(stream)?;

    // Read data rows
    let mut rows = Vec::new();
    loop {
        let (row_pkt, _) = read_packet(stream)?;

        // Check for EOF
        if row_pkt.len() > 0 && row_pkt[0] == 0xFE && row_pkt.len() < 9 {
            break;
        }

        let fields = parse_row_fields(&row_pkt);
        rows.push(fields);
    }

    Ok(rows)
}

fn parse_row_fields(row: &[u8]) -> Vec<String> {
    let mut fields = Vec::new();
    let mut pos = 0;

    while pos < row.len() {
        if row[pos] == 0xFB {
            fields.push("NULL".to_string());
            pos += 1;
        } else {
            let (len, len_size) = decode_length_coded_binary(&row[pos..]);
            pos += len_size;

            if pos + len <= row.len() {
                let field = String::from_utf8_lossy(&row[pos..pos+len]).to_string();
                fields.push(field);
                pos += len;
            } else {
                break;
            }
        }
    }

    fields
}

fn decode_length_coded_binary(data: &[u8]) -> (usize, usize) {
    if data.is_empty() {
        return (0, 0);
    }

    match data[0] {
        0..=250 => (data[0] as usize, 1),
        252 => {
            if data.len() >= 3 {
                let len = u16::from_le_bytes([data[1], data[2]]) as usize;
                (len, 3)
            } else {
                (0, 1)
            }
        }
        253 => {
            if data.len() >= 4 {
                let len = u32::from_le_bytes([data[1], data[2], data[3], 0]) as usize;
                (len, 4)
            } else {
                (0, 1)
            }
        }
        _ => (0, 1),
    }
}

fn read_packet(s: &mut TcpStream) -> Result<(Vec<u8>, u8), Box<dyn std::error::Error>> {
    let mut h = [0u8; 4];
    s.read_exact(&mut h)?;
    let len = u32::from_le_bytes([h[0], h[1], h[2], 0]) as usize;
    let mut p = vec![0u8; len];
    s.read_exact(&mut p)?;
    Ok((p, h[3]))
}

fn write_packet(s: &mut TcpStream, p: &[u8], seq: u8) -> Result<(), Box<dyn std::error::Error>> {
    let l = p.len() as u32;
    s.write_all(&[(l & 0xFF) as u8, ((l >> 8) & 0xFF) as u8, ((l >> 16) & 0xFF) as u8, seq])?;
    s.write_all(p)?;
    s.flush()?;
    Ok(())
}

fn build_auth() -> Vec<u8> {
    let mut p = Vec::new();
    p.extend_from_slice(&0x0000A285u32.to_le_bytes());
    p.extend_from_slice(&0x01000000u32.to_le_bytes());
    p.push(0x21);
    p.extend_from_slice(&[0u8; 23]);
    p.extend_from_slice(b"root\0");
    p.push(0x00);
    p
}

fn build_query(q: &str) -> Vec<u8> {
    let mut p = Vec::new();
    p.push(0x03);
    p.extend_from_slice(q.as_bytes());
    p
}
