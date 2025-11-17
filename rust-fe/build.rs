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

    // Decide which proto API to compile.
    //
    // - Default: compile proto/backend_service.proto (PoC SQL-based protocol)
    // - With feature "real_be_proto": compile proto/internal_service.proto and its
    //   supporting files (real Doris BE internal service).
    //
    // The "real_be_proto" Cargo feature is surfaced to build.rs via the
    // CARGO_FEATURE_REAL_BE_PROTO env var.
    let use_real_proto = env::var("CARGO_FEATURE_REAL_BE_PROTO").is_ok();

    let mut proto_files = Vec::new();
    let mut has_backend = false;
    let mut has_cloud = false;

    if use_real_proto {
        // Real Doris BE protocol: internal_service.proto and its dependencies.
        let internal_proto = std::path::Path::new("proto/internal_service.proto");
        if internal_proto.exists() {
            println!("cargo:warning=Using real Doris internal_service.proto (feature=real_be_proto)");
            proto_files.push("proto/data.proto");
            proto_files.push("proto/types.proto");
            proto_files.push("proto/segment_v2.proto");
            proto_files.push("proto/olap_common.proto");
            proto_files.push("proto/olap_file.proto");
            proto_files.push("proto/runtime_profile.proto");
            proto_files.push("proto/internal_service.proto");
            has_backend = true;
        } else {
            println!("cargo:warning=real_be_proto enabled but proto/internal_service.proto not found, falling back to backend_service.proto");
        }
    }

    if !use_real_proto {
        // Default PoC backend service.
        let backend_proto = std::path::Path::new("proto/backend_service.proto");
        if backend_proto.exists() {
            println!("cargo:warning=Using PoC backend_service.proto");
            proto_files.push("proto/backend_service.proto");
            has_backend = true;
        }

        // Always compile segment_v2.proto so that the generated Rust module exists
        // for types referenced from other protos. This does not change the PoC
        // wire protocol surface.
        let segment_proto = std::path::Path::new("proto/segment_v2.proto");
        if segment_proto.exists() {
            println!("cargo:warning=Also compiling segment_v2.proto for PoC build");
            proto_files.push("proto/segment_v2.proto");
        }

        let cloud_proto = std::path::Path::new("proto/cloud.proto");
        if cloud_proto.exists() {
            println!("cargo:warning=Found cloud.proto");
            // cloud.proto depends on olap_file.proto which depends on segment_v2.proto
            // We need to compile all of them
            proto_files.push("proto/olap_file.proto");
            proto_files.push("proto/cloud.proto");
            has_cloud = true;
        }
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

        // Continue to Thrift codegen if enabled.
    }

    if proto_files.is_empty() {
        eprintln!("No proto files found. Run with SKIP_PROTO=1 to use fallback.");
        std::process::exit(1);
    }

    Ok(())
}
