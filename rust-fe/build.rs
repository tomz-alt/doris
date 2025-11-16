use std::env;
use std::path::PathBuf;
use std::io::Result;

fn main() -> Result<()> {
    // Check if we should skip proto compilation
    if env::var("SKIP_PROTO").is_ok() {
        println!("cargo:warning=Skipping proto compilation (SKIP_PROTO is set)");
        println!("cargo:warning=Backend communication will use fallback implementation");
        // Expose a cfg flag so code can avoid depending on generated gRPC types.
        println!("cargo:rustc-cfg=skip_proto");

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

    // Compile proto files
    let backend_proto = std::path::Path::new("proto/backend_service.proto");
    let cloud_proto = std::path::Path::new("proto/cloud.proto");

    let mut proto_files = Vec::new();
    let mut has_backend = false;
    let mut has_cloud = false;

    if backend_proto.exists() {
        println!("cargo:warning=Found backend_service.proto");
        proto_files.push("proto/backend_service.proto");
        has_backend = true;
    }

    if cloud_proto.exists() {
        println!("cargo:warning=Found cloud.proto");
        // cloud.proto depends on olap_file.proto which depends on segment_v2.proto
        // We need to compile all of them
        proto_files.push("proto/segment_v2.proto");
        proto_files.push("proto/olap_file.proto");
        proto_files.push("proto/cloud.proto");
        has_cloud = true;
    }

    if !proto_files.is_empty() {
        println!("cargo:warning=Compiling {} proto files with tonic-build...", proto_files.len());

        // Use tonic-build to compile all protos in one go
        // This generates both message types AND gRPC service definitions
        tonic_build::configure()
            .build_server(has_cloud)  // Enable server for MetaService if cloud.proto exists
            .build_client(has_backend) // Enable client for backend service
            .out_dir({
                let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
                out_dir
            })
            .compile(
                &proto_files,
                &["proto"]
            )
            .map_err(|e| {
                println!("cargo:warning=Failed to compile proto: {}", e);
                println!("cargo:warning=Run: SKIP_PROTO=1 cargo build to skip");
                e
            })?;

        println!("cargo:warning=Proto compilation complete!");

        return Ok(());
    }

    eprintln!("No proto files found. Run with SKIP_PROTO=1 to use fallback.");
    std::process::exit(1);
}
