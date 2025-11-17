// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Version information

use serde::{Deserialize, Serialize};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_COMMIT: &str = env!("VERGEN_GIT_SHA");
pub const BUILD_TIME: &str = env!("VERGEN_BUILD_TIMESTAMP");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub git_commit: String,
    pub build_time: String,
    pub rust_version: String,
}

impl VersionInfo {
    pub fn new() -> Self {
        Self {
            version: VERSION.to_string(),
            git_commit: GIT_COMMIT.to_string(),
            build_time: BUILD_TIME.to_string(),
            rust_version: rustc_version_runtime::version().to_string(),
        }
    }
}

impl Default for VersionInfo {
    fn default() -> Self {
        Self::new()
    }
}
