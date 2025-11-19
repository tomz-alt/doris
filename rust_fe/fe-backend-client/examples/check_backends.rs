// Quick script to check backends
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:9030")?;
    let mut seq = 0u8;

    // Read handshake
    let (_, s) = read_packet(&mut stream)?;
    seq = s.wrapping_add(1);

    // Send auth
    let auth = build_auth();
    write_packet(&mut stream, &auth, seq)?;
    seq = seq.wrapping_add(1);

    // Read auth response
    read_packet(&mut stream)?;

    // Send SHOW BACKENDS
    seq = 0;
    let query = build_query("SHOW BACKENDS");
    write_packet(&mut stream, &query, seq)?;

    // Read response
    let (resp, _) = read_packet(&mut stream)?;

    if resp[0] == 0xFF {
        println!("Error: {:?}", String::from_utf8_lossy(&resp[9..]));
    } else {
        println!("Response: {} bytes", resp.len());
        // Try to read result set
        for _ in 0..10 {
            match read_packet(&mut stream) {
                Ok((pkt, _)) => {
                    if pkt.len() > 0 && pkt[0] == 0xFE && pkt.len() < 9 {
                        break; // EOF
                    }
                    println!("Packet: {} bytes, first byte: 0x{:02X}", pkt.len(), pkt[0]);
                },
                Err(_) => break,
            }
        }
    }

    Ok(())
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
    pkt.extend_from_slice(&0x0000A285u32.to_le_bytes()); // capabilities
    pkt.extend_from_slice(&0x01000000u32.to_le_bytes()); // max packet
    pkt.push(0x21); // charset
    pkt.extend_from_slice(&[0u8; 23]); // reserved
    pkt.extend_from_slice(b"root\0"); // username
    pkt.push(0x00); // empty password
    pkt
}

fn build_query(q: &str) -> Vec<u8> {
    let mut pkt = Vec::new();
    pkt.push(0x03); // COM_QUERY
    pkt.extend_from_slice(q.as_bytes());
    pkt
}
