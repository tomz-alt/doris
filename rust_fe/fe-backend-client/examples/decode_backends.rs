// Decode SHOW BACKENDS result properly
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== SHOW BACKENDS Analysis ===\n");

    let mut stream = TcpStream::connect("127.0.0.1:9030")?;

    // Auth
    let (_, s) = read_packet(&mut stream)?;
    write_packet(&mut stream, &build_auth(), s.wrapping_add(1))?;
    let (auth_resp, _) = read_packet(&mut stream)?;
    if auth_resp[0] == 0xFF {
        return Err("Auth failed".into());
    }

    // Query
    write_packet(&mut stream, &build_query("SHOW BACKENDS"), 0)?;
    let (resp, _) = read_packet(&mut stream)?;

    if resp[0] == 0xFF {
        println!("Error: {:?}", String::from_utf8_lossy(&resp[9..]));
        return Err("Query failed".into());
    }

    // Result set metadata - column definitions
    println!("Response type: 0x{:02X}", resp[0]);
    println!("Number of columns in result set: {}", resp[0]);
    println!();

    // Read column definitions
    let num_cols = resp[0] as usize;
    let mut col_names = Vec::new();

    for i in 0..num_cols {
        let (col_def, _) = read_packet(&mut stream)?;
        // Parse column name (simplified - actual parsing is complex)
        if let Some(name_str) = parse_column_name(&col_def) {
            println!("Column {}: {}", i + 1, name_str);
            col_names.push(name_str);
        }
    }
    println!();

    // Read EOF packet after column definitions
    read_packet(&mut stream)?;

    // Read data rows
    println!("=== Backend Rows ===");
    let mut row_num = 0;
    loop {
        let (row_pkt, _) = read_packet(&mut stream)?;

        // Check for EOF packet (0xFE and length < 9)
        if row_pkt.len() > 0 && row_pkt[0] == 0xFE && row_pkt.len() < 9 {
            println!("\nTotal backends: {}", row_num);
            break;
        }

        row_num += 1;
        println!("\nRow {}:", row_num);
        println!("  Raw packet length: {} bytes", row_pkt.len());
        println!("  First 50 bytes (hex): {}", hex_dump(&row_pkt[..row_pkt.len().min(50)]));

        // Try to parse as Length Coded Binary strings
        let fields = parse_row_fields(&row_pkt);
        for (i, field) in fields.iter().enumerate() {
            if i < col_names.len() {
                println!("  {}: {}", col_names[i], field);
            } else {
                println!("  Field {}: {}", i, field);
            }
        }
    }

    Ok(())
}

fn parse_column_name(col_def: &[u8]) -> Option<String> {
    // Column definition packet structure is complex
    // Try to extract the name (usually near the end before field type)
    if col_def.len() > 10 {
        // Skip catalog, database, table, org_table
        let mut pos = 0;
        for _ in 0..4 {
            if pos >= col_def.len() {
                return None;
            }
            let len = col_def[pos] as usize;
            pos += 1 + len;
        }

        // Now should be the name
        if pos < col_def.len() {
            let name_len = col_def[pos] as usize;
            if pos + 1 + name_len <= col_def.len() {
                return Some(String::from_utf8_lossy(&col_def[pos+1..pos+1+name_len]).to_string());
            }
        }
    }
    None
}

fn parse_row_fields(row: &[u8]) -> Vec<String> {
    let mut fields = Vec::new();
    let mut pos = 0;

    while pos < row.len() {
        if row[pos] == 0xFB {
            // NULL value
            fields.push("NULL".to_string());
            pos += 1;
        } else {
            // Length-coded binary
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

fn hex_dump(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")
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
