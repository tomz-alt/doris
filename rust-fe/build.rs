use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if we should skip proto compilation
    if env::var("SKIP_PROTO").is_ok() {
        println!("cargo:warning=Skipping proto compilation (SKIP_PROTO is set)");
        println!("cargo:warning=Backend communication will use fallback implementation");

        // Create a dummy file to satisfy the include
        let out_dir = PathBuf::from(env::var("OUT_DIR")?);
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

    // Try to compile proto files
    println!("cargo:rerun-if-changed=proto/");

    // Use only the backend_service.proto which is self-contained
    let simple_proto = std::path::Path::new("proto/backend_service.proto");

    if simple_proto.exists() {
        println!("cargo:warning=Compiling simplified backend_service.proto");

        return tonic_build::configure()
            .build_server(false)
            .build_client(true)
            .compile(&["proto/backend_service.proto"], &["proto"])
            .map_err(|e| {
                println!("cargo:warning=Failed to compile proto: {}", e);
                println!("cargo:warning=Run: SKIP_PROTO=1 cargo build to skip");
                e.into()
            });
    }

    // If we have the full Doris protos, try to compile them
    let doris_proto = std::path::Path::new("proto/internal_service.proto");

    if doris_proto.exists() {
        println!("cargo:warning=Compiling Doris internal_service.proto");
        println!("cargo:warning=This requires protoc to be installed");

        // Compile all dependent proto files
        return tonic_build::configure()
            .build_server(false)
            .build_client(true)
            .protoc_arg("--experimental_allow_proto3_optional")
            .compile(
                &["proto/internal_service.proto"],
                &["proto"]
            )
            .map_err(|e| {
                println!("cargo:warning=Failed to compile Doris proto files: {}", e);
                println!("cargo:warning=Error: {}", e);
                println!("cargo:warning=Set SKIP_PROTO=1 to use fallback implementation");
                e.into()
            });
    }

    Err("No proto files found. Run with SKIP_PROTO=1 to use fallback.".into())
}
