// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! SQL Analysis and Parsing
//!
//! This module provides SQL parsing and semantic analysis.
//! Uses sqlparser-rs for parsing with Doris-specific extensions.

pub mod parser;
pub mod ast;
pub mod analyzer;

pub use parser::DorisParser;
pub use ast::{Statement, CreateTableStatement, SelectStatement};
