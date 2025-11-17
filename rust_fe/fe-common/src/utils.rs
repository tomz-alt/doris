// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Utility functions

use std::time::{SystemTime, UNIX_EPOCH};

/// Get current timestamp in milliseconds
pub fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

/// Get current timestamp in seconds
pub fn current_timestamp_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

/// Parse size string (e.g., "10GB", "512MB") to bytes
pub fn parse_size(size_str: &str) -> Result<u64, String> {
    let size_str = size_str.trim().to_uppercase();

    let (num_str, unit) = if let Some(pos) = size_str.find(|c: char| !c.is_numeric() && c != '.') {
        (&size_str[..pos], size_str[pos..].trim())
    } else {
        (size_str.as_str(), "")
    };

    let number: f64 = num_str.parse()
        .map_err(|e| format!("Invalid number: {}", e))?;

    let multiplier: u64 = match unit {
        "" | "B" => 1,
        "KB" | "K" => 1024,
        "MB" | "M" => 1024 * 1024,
        "GB" | "G" => 1024 * 1024 * 1024,
        "TB" | "T" => 1024 * 1024 * 1024 * 1024,
        "PB" | "P" => 1024 * 1024 * 1024 * 1024 * 1024,
        _ => return Err(format!("Unknown unit: {}", unit)),
    };

    Ok((number * multiplier as f64) as u64)
}

/// Format bytes to human readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];

    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024").unwrap(), 1024);
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("1MB").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1GB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("1.5GB").unwrap(), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
        assert_eq!(parse_size("  2 MB  ").unwrap(), 2 * 1024 * 1024);
    }

    #[test]
    fn test_parse_size_error() {
        assert!(parse_size("invalid").is_err());
        assert!(parse_size("10XX").is_err());
        assert!(parse_size("").is_err());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1536 * 1024), "1.50 MB");
    }

    #[test]
    fn test_timestamps() {
        let ts_ms = current_timestamp_ms();
        let ts_s = current_timestamp_s();

        assert!(ts_ms > 0);
        assert!(ts_s > 0);
        assert!(ts_ms / 1000 >= ts_s - 1); // Allow 1s tolerance
    }
}
