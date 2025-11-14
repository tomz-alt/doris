fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Skip proto compilation if SKIP_PROTO is set (for environments without protoc)
    if std::env::var("SKIP_PROTO").is_ok() {
        println!("cargo:warning=Skipping proto compilation (SKIP_PROTO is set)");
        println!("cargo:warning=Backend communication will not be available");
        return Ok(());
    }

    // Build gRPC proto files for BE communication
    // For PoC, we'll define a simplified proto
    match tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(&["proto/backend_service.proto"], &["proto"])
    {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("cargo:warning=Failed to compile proto files: {}", e);
            println!("cargo:warning=Set SKIP_PROTO=1 to skip proto compilation");
            println!("cargo:warning=Or install protoc: apt-get install protobuf-compiler");
            Err(e)
        }
    }
}
