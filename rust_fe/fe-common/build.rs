// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

use std::process::Command;

fn main() {
    // Get git commit hash
    let git_hash = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    println!("cargo:rustc-env=VERGEN_GIT_SHA={}", git_hash);

    // Get build timestamp
    let timestamp = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=VERGEN_BUILD_TIMESTAMP={}", timestamp);

    // Rerun if git HEAD changes
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
