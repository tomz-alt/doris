use std::env;
use std::path::PathBuf;
use std::io::Result;

fn main() -> Result<()> {
    // Check if we should skip proto compilation
    if env::var("SKIP_PROTO").is_ok() {
        println!("cargo:warning=Skipping proto compilation (SKIP_PROTO is set)");
        println!("cargo:warning=Backend communication will use fallback implementation");

        // Create a dummy file to satisfy the include
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
        std::fs::write(
            out_dir.join("doris.rs"),
            "// Proto generation skipped - using fallback\n\
             pub mod doris {\n\
                 pub struct PUniqueId { pub hi: i64, pub lo: i64 }\n\
                 pub struct PExecPlanFragmentRequest {}\n\
                 pub struct PExecPlanFragmentResult { pub status: Option<PStatus> }\n\
                 pub struct PStatus { pub status_code: i32 }\n\
                 pub struct PFetchDataRequest {}\n\
                 pub struct PFetchDataResult {}\n\
                 pub struct PCancelPlanFragmentRequest {}\n\
                 pub struct PCancelPlanFragmentResult {}\n\
             }\n"
        )?;
        return Ok(());
    }

    // Compile proto files using protoc built from source!
    println!("cargo:rerun-if-changed=proto/");
    println!("cargo:warning=Building protoc from source using protobuf-src...");

    // Build protoc from source
    let protoc_path = protobuf_src::protoc();
    println!("cargo:warning=Successfully built protoc at: {:?}", protoc_path);

    // Set the PROTOC environment variable so prost-build can find it
    env::set_var("PROTOC", &protoc_path);

    // Use only the backend_service.proto which is self-contained
    let simple_proto = std::path::Path::new("proto/backend_service.proto");

    if simple_proto.exists() {
        println!("cargo:warning=Found backend_service.proto - compiling with prost-build...");

        // Use prost-build with the compiled protoc
        let mut prost_config = prost_build::Config::new();
        prost_config
            .out_dir({
                let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
                out_dir
            })
            .compile_protos(
                &["proto/backend_service.proto"],
                &["proto"]
            )
            .map_err(|e| {
                println!("cargo:warning=Failed to compile proto: {}", e);
                println!("cargo:warning=Run: SKIP_PROTO=1 cargo build to skip");
                e
            })?;

        println!("cargo:warning=Successfully compiled protobuf messages!");

        // Now compile the gRPC service definitions using tonic-build
        println!("cargo:warning=Compiling gRPC service definitions...");

        tonic_build::configure()
            .build_server(true)  // Enable server for mock BE
            .build_client(true)
            .compile(
                &["proto/backend_service.proto"],
                &["proto"]
            )
            .map_err(|e| {
                println!("cargo:warning=Failed to compile gRPC service: {}", e);
                println!("cargo:warning=Continuing without gRPC client generation");
                // Don't fail - we can manually implement the client
            })
            .ok();

        println!("cargo:warning=Proto compilation complete!");

        return Ok(());
    }

    eprintln!("No proto files found. Run with SKIP_PROTO=1 to use fallback.");
    std::process::exit(1);
}
