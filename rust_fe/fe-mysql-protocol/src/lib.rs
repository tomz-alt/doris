// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! MySQL Protocol Implementation
//!
//! This module implements the MySQL wire protocol for client connections.
//! Supports:
//! - Handshake and authentication
//! - Command processing (COM_QUERY, COM_INIT_DB, etc.)
//! - Result set encoding

pub mod constants;
pub mod packet;

// TODO: Implement these modules
// pub mod codec;
// pub mod server;
// pub mod auth;
