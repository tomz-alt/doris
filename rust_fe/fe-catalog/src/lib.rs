// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! Catalog module for managing database metadata
//!
//! This module handles:
//! - Database definitions and operations
//! - Table definitions and operations
//! - Column definitions
//! - Partition management
//! - Index management
//! - Replica management
//! - In-memory data storage (for testing before BE integration)

pub mod database;
pub mod table;
pub mod column;
pub mod partition;
pub mod index;
pub mod replica;
pub mod catalog;
pub mod datastore;

pub use database::Database;
pub use table::{Table, OlapTable};
pub use column::Column;
pub use partition::Partition;
pub use index::Index;
pub use replica::Replica;
pub use catalog::Catalog;
pub use datastore::DataStore;
