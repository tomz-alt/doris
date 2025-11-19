// Insert TPC-H sample data into existing lineitem table
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════");
    println!("  Inserting TPC-H Sample Data");
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

    // Use database
    println!("Switching to tpch database...");
    execute_query(&mut stream, "USE tpch")?;
    println!("✅ Database selected\n");

    // Insert sample data
    println!("Inserting 4 sample rows...");
    let inserts = vec![
        "INSERT INTO lineitem VALUES (1, 155190, 7706, 1, 17.00, 21168.23, 0.04, 0.02, 'N', 'O', '1996-03-13', '1996-02-12', '1996-03-22', 'DELIVER IN PERSON', 'TRUCK', 'regular deposits')",
        "INSERT INTO lineitem VALUES (1, 67310, 7311, 2, 36.00, 45983.16, 0.09, 0.06, 'N', 'O', '1996-04-12', '1996-02-28', '1996-04-20', 'TAKE BACK RETURN', 'MAIL', 'unusual theodolites')",
        "INSERT INTO lineitem VALUES (2, 106170, 1191, 1, 38.00, 44694.46, 0.00, 0.05, 'N', 'O', '1997-01-28', '1997-01-14', '1997-02-02', 'TAKE BACK RETURN', 'RAIL', 'bold packages haggle')",
        "INSERT INTO lineitem VALUES (3, 4297, 1798, 1, 45.00, 54058.05, 0.06, 0.00, 'R', 'F', '1994-02-02', '1994-01-04', '1994-02-23', 'NONE', 'AIR', 'pending instructions')",
    ];

    for (i, insert_sql) in inserts.iter().enumerate() {
        match execute_query(&mut stream, insert_sql) {
            Ok(_) => println!("  ✓ Row {} inserted", i + 1),
            Err(e) => {
                println!("  ✗ Row {} error: {}", i + 1, e);
                return Err(e);
            }
        }
    }
    println!("\n✅ All 4 rows inserted successfully!\n");

    // Verify data count
    println!("Verifying data with COUNT query...");
    execute_query(&mut stream, "SELECT COUNT(*) FROM lineitem")?;
    println!("✅ Query executed\n");

    println!("═══════════════════════════════════════");
    println!("SUCCESS! Sample data loaded!");
    println!("═══════════════════════════════════════");

    Ok(())
}

fn execute_query(stream: &mut TcpStream, query: &str) -> Result<(), Box<dyn std::error::Error>> {
    write_packet(stream, &build_query(query), 0)?;
    let (resp, _) = read_packet(stream)?;

    match resp[0] {
        0x00 => Ok(()),
        0xFF => {
            let msg = String::from_utf8_lossy(&resp[9..]);
            Err(format!("{}", msg).into())
        },
        _ => {
            // Result set - drain it
            loop {
                match read_packet(stream) {
                    Ok((pkt, _)) => if pkt.len() > 0 && pkt[0] == 0xFE && pkt.len() < 9 { break; },
                    Err(_) => break,
                }
            }
            Ok(())
        }
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
