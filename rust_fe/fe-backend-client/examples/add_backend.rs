// Add backend to FE
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Adding backend to FE...\n");

    execute_query("ALTER SYSTEM ADD BACKEND '127.0.0.1:9050'")?;
    println!("✅ Backend added!\n");

    println!("Waiting for heartbeat...");
    std::thread::sleep(std::time::Duration::from_secs(10));
    println!("✅ Wait complete\n");

    println!("Checking backends...");
    execute_query("SHOW BACKENDS")?;
    println!("✅ Done\n");

    Ok(())
}

fn execute_query(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:9030")?;
    let (_, s) = read_packet(&mut stream)?;
    write_packet(&mut stream, &build_auth(), s.wrapping_add(1))?;
    let (auth_resp, _) = read_packet(&mut stream)?;
    if auth_resp[0] == 0xFF {
        return Err("Auth failed".into());
    }
    write_packet(&mut stream, &build_query(query), 0)?;
    let (resp, _) = read_packet(&mut stream)?;
    match resp[0] {
        0x00 => {
            println!("   ✓ OK");
            Ok(())
        },
        0xFF => {
            let msg = String::from_utf8_lossy(&resp[9..]);
            if msg.contains("already exists") {
                println!("   ✓ Already exists (OK)");
                Ok(())
            } else {
                println!("   ✗ Error: {}", msg);
                Err(format!("{}", msg).into())
            }
        },
        _ => {
            println!("   ✓ Result set");
            loop {
                match read_packet(&mut stream) {
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
