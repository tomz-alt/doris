use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Generate Rust code from the Doris Thrift IDL using the official
    // Apache Thrift compiler. This crate is a thin wrapper around the
    // generated types so that the main FE crate can depend on a single
    // module boundary for all Thrift structs.
    println!("cargo:rerun-if-changed=../gensrc/thrift");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));

    // The Rust FE crate lives at `rust-fe/` inside the Doris repo, and the
    // Thrift IDL files live at `../gensrc/thrift` from there. This Thrift
    // helper crate lives at `rust-fe/doris-thrift`, so we need to walk two
    // levels up from our manifest directory to reach the Doris repo root.
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let thrift_dir = manifest_dir
        .join("..")
        .join("..")
        .join("gensrc")
        .join("thrift");

    let thrift_files: [(&str, &str); 15] = [
        ("Data.thrift", "data.rs"),
        ("DataSinks.thrift", "data_sinks.rs"),
        ("Descriptors.thrift", "descriptors.rs"),
        ("ExternalTableSchema.thrift", "external_table_schema.rs"),
        ("Exprs.thrift", "exprs.rs"),
        ("Metrics.thrift", "metrics.rs"),
        ("Opcodes.thrift", "opcodes.rs"),
        ("Partitions.thrift", "partitions.rs"),
        ("PlanNodes.thrift", "plan_nodes.rs"),
        ("Planner.thrift", "planner.rs"),
        ("QueryCache.thrift", "query_cache.rs"),
        ("RuntimeProfile.thrift", "runtime_profile.rs"),
        ("Status.thrift", "status.rs"),
        ("Types.thrift", "types.rs"),
        ("PaloInternalService.thrift", "palo_internal_service.rs"),
    ];

    for (idl, rs_name) in thrift_files {
        let src = thrift_dir.join(idl);
        if !src.exists() {
            panic!(
                "Expected Thrift IDL file not found: {}",
                src.display()
            );
        }

        let status = Command::new("thrift")
            .arg("--gen")
            .arg("rs")
            .arg("-out")
            .arg(&out_dir)
            .arg(&src)
            .status()
            .expect("failed to invoke `thrift` compiler");

        if !status.success() {
            panic!(
                "thrift codegen failed for {} with status {}",
                src.display(),
                status
            );
        }

        // The Rust generator uses inner attributes like `#![allow(dead_code)]`
        // at the top of each file. When we `include!` these files inside a
        // module, those must become outer attributes instead. Rewrite them
        // from `#![...]` to `#[...]` so that they apply to the next item.
        let rs_path = out_dir.join(rs_name);
        let contents = fs::read_to_string(&rs_path)
            .unwrap_or_else(|e| panic!("failed to read generated file {}: {}", rs_path.display(), e));
        let fixed = contents.replace("#![", "#[");
        fs::write(&rs_path, fixed)
            .unwrap_or_else(|e| panic!("failed to rewrite generated file {}: {}", rs_path.display(), e));
    }
}
