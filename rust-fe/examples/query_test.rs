/// Simple test client to execute queries against the Rust FE
/// Usage: cargo run --example query_test
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connecting to Rust FE MySQL server at 127.0.0.1:9030...");

    let mut stream = TcpStream::connect("127.0.0.1:9030").await?;
    println!("Connected!");

    // Read handshake packet
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    println!("Received handshake: {} bytes", n);

    // For now, just verify we can connect
    // Full MySQL protocol handshake would be needed for actual queries
    println!("\n✓ Connection successful!");
    println!("Server is listening and responding on port 9030");

    Ok(())
}
