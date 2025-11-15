// Catalog management for Rust FE
// Handles table registration and metadata

pub mod tpch_tables;
pub mod be_table;

pub use tpch_tables::register_tpch_tables;
pub use be_table::BETableProvider;
