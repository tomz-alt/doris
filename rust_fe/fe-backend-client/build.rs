// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Build script to generate Rust code from Protobuf definitions

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from("../../gensrc/proto");

    if !proto_root.exists() {
        eprintln!("Warning: {} not found. Skipping protobuf generation.", proto_root.display());
        eprintln!("This is expected in environments without the full Doris source tree.");
        return Ok(());
    }

    println!("cargo:rerun-if-changed={}", proto_root.display());

    // Try to generate protobuf, but don't fail if protoc is not installed
    match tonic_build::configure()
        .build_server(false)  // We only need the client
        .build_client(true)
        .out_dir("src/generated")
        .compile(
            &[
                proto_root.join("internal_service.proto").to_str().unwrap(),
            ],
            &[proto_root.to_str().unwrap()],
        ) {
        Ok(_) => {
            println!("cargo:warning=Successfully generated protobuf bindings");
        }
        Err(e) => {
            eprintln!("Warning: Failed to generate protobuf bindings: {}", e);
            eprintln!("Install protoc to enable BE communication:");
            eprintln!("  Debian/Ubuntu: apt-get install protobuf-compiler");
            eprintln!("  macOS: brew install protobuf");
            eprintln!("Continuing without protobuf generation...");
        }
    }

    Ok(())
}
