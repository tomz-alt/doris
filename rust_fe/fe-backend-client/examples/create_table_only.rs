// Just create the table
use std::io::{Read, Write};
use std::net::TcpStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating tpch.lineitem table...\n");

    execute_query("CREATE DATABASE IF NOT EXISTS tpch")?;
    println!("✅ Database created\n");

    let create_table = r#"
        CREATE TABLE IF NOT EXISTS tpch.lineitem (
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

    println!("Creating table...");
    execute_query(create_table)?;
    println!("✅ TABLE CREATED!\n");

    println!("═══════════════════════════════════════");
    println!("SUCCESS! tpch.lineitem table created!");
    println!("═══════════════════════════════════════");

    Ok(())
}

fn execute_query(query: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:9030")?;
    let mut seq = 0u8;

    let (_, s) = read_packet(&mut stream)?;
    seq = s.wrapping_add(1);

    let auth = build_auth();
    write_packet(&mut stream, &auth, seq)?;

    let (auth_resp, _) = read_packet(&mut stream)?;
    if auth_resp[0] == 0xFF {
        return Err(format!("Auth failed").into());
    }

    seq = 0;
    let q = build_query(query);
    write_packet(&mut stream, &q, seq)?;

    let (resp, _) = read_packet(&mut stream)?;
    match resp[0] {
        0x00 => Ok(()),
        0xFF => {
            let msg = String::from_utf8_lossy(&resp[9..]);
            Err(format!("Query failed: {}", msg).into())
        },
        _ => {
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
