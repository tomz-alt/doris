// Re-register BE to new FE instance
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Re-registering BE to FE...\n");

    // Step 1: DROP old backend
    println!("Step 1: Dropping old backend registration...");
    execute_query("ALTER SYSTEM DROP BACKEND '127.0.0.1:9050'")?;
    println!("✅ Old backend dropped\n");

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Step 2: ADD backend again
    println!("Step 2: Adding backend to new FE instance...");
    execute_query("ALTER SYSTEM ADD BACKEND '127.0.0.1:9050'")?;
    println!("✅ Backend added\n");

    // Wait for heartbeat
    println!("Step 3: Waiting for heartbeat...");
    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("✅ Wait complete\n");

    // Check backends
    println!("Step 4: Checking backend status...");
    execute_query("SHOW BACKENDS")?;
    println!("✅ Check complete\n");

    println!("═══════════════════════════════════════");
    println!("Backend re-registration complete!");
    println!("Now try creating the table again.");
    println!("═══════════════════════════════════════");

    Ok(())
}

fn execute_query(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:9030")?;
    let mut seq = 0u8;

    // Read handshake
    let (_, s) = read_packet(&mut stream)?;
    seq = s.wrapping_add(1);

    // Send auth
    let auth = build_auth();
    write_packet(&mut stream, &auth, seq)?;
    seq = s.wrapping_add(1);

    // Read auth response
    let (auth_resp, _) = read_packet(&mut stream)?;
    if auth_resp[0] == 0xFF {
        return Err(format!("Auth failed: {:?}", String::from_utf8_lossy(&auth_resp[9..])).into());
    }

    // Send query
    seq = 0;
    let q = build_query(query);
    write_packet(&mut stream, &q, seq)?;

    // Read response
    let (resp, _) = read_packet(&mut stream)?;
    match resp[0] {
        0x00 => {
            println!("   ✓ Command succeeded");
            Ok(())
        },
        0xFF => {
            let msg = String::from_utf8_lossy(&resp[9..]);
            if msg.contains("does not exist") || msg.contains("already exists") {
                println!("   ⚠ {}", msg);
                Ok(())
            } else {
                Err(format!("Query failed: {}", msg).into())
            }
        },
        _ => {
            // Result set - drain it
            println!("   ✓ Query returned {} bytes", resp.len());
            loop {
                match read_packet(&mut stream) {
                    Ok((pkt, _)) => {
                        if pkt.len() > 0 && pkt[0] == 0xFE && pkt.len() < 9 {
                            break;
                        }
                    },
                    Err(_) => break,
                }
            }
            Ok(())
        }
    }
}

fn read_packet(stream: &mut TcpStream) -> Result<(Vec<u8>, u8), Box<dyn std::error::Error>> {
    let mut hdr = [0u8; 4];
    stream.read_exact(&mut hdr)?;
    let len = u32::from_le_bytes([hdr[0], hdr[1], hdr[2], 0]) as usize;
    let seq = hdr[3];
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload)?;
    Ok((payload, seq))
}

fn write_packet(stream: &mut TcpStream, payload: &[u8], seq: u8) -> Result<(), Box<dyn std::error::Error>> {
    let len = payload.len() as u32;
    let hdr = [(len & 0xFF) as u8, ((len >> 8) & 0xFF) as u8, ((len >> 16) & 0xFF) as u8, seq];
    stream.write_all(&hdr)?;
    stream.write_all(payload)?;
    stream.flush()?;
    Ok(())
}

fn build_auth() -> Vec<u8> {
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&0x0000A285u32.to_le_bytes());
    pkt.extend_from_slice(&0x01000000u32.to_le_bytes());
    pkt.push(0x21);
    pkt.extend_from_slice(&[0u8; 23]);
    pkt.extend_from_slice(b"root\0");
    pkt.push(0x00);
    pkt
}

fn build_query(q: &str) -> Vec<u8> {
    let mut pkt = Vec::new();
    pkt.push(0x03);
    pkt.extend_from_slice(q.as_bytes());
    pkt
}
