// Copyright 2025 Apache Doris Community
// Licensed under the Apache License, Version 2.0

//! TPC-H Schema Loader
//!
//! CLAUDE.MD Principle #2: Use Java FE behavior as specification
//! This module replicates the exact TPC-H schema created by minimal_mysql_client.rs
//! to enable query execution without requiring running Java FE.
//!
//! Schema source: fe-backend-client/examples/minimal_mysql_client.rs lines 125-149

use fe_common::Result;
use fe_common::types::{KeysType, DataType};
use crate::catalog::Catalog;
use crate::table::OlapTable;
use crate::column::Column;
use crate::partition::Partition;

/// Load TPC-H schema into catalog
///
/// Creates:
/// - Database: tpch
/// - Table: lineitem (TPC-H scale factor 0.01 for testing)
///
/// This matches the schema created by minimal_mysql_client.rs:
/// ```sql
/// CREATE TABLE lineitem (
///     l_orderkey BIGINT,
///     l_partkey BIGINT,
///     l_suppkey BIGINT,
///     l_linenumber INT,
///     l_quantity DECIMAL(15,2),
///     l_extendedprice DECIMAL(15,2),
///     l_discount DECIMAL(15,2),
///     l_tax DECIMAL(15,2),
///     l_returnflag CHAR(1),
///     l_linestatus CHAR(1),
///     l_shipdate DATE,
///     l_commitdate DATE,
///     l_receiptdate DATE,
///     l_shipinstruct CHAR(25),
///     l_shipmode CHAR(10),
///     l_comment VARCHAR(44)
/// )
/// DUPLICATE KEY(l_orderkey)
/// DISTRIBUTED BY HASH(l_orderkey) BUCKETS 1
/// ```
pub fn load_tpch_schema(catalog: &Catalog) -> Result<()> {
    println!("Loading TPC-H schema into catalog");

    // Create tpch database
    let db_id = catalog.create_database("tpch".to_string(), "default_cluster".to_string())?;
    println!("Created database 'tpch' with ID {}", db_id);

    // Create lineitem table
    let lineitem_table = create_lineitem_table(db_id)?;
    let table_id = catalog.create_table("tpch", lineitem_table)?;
    println!("Created table 'lineitem' with ID {}", table_id);

    // Add default partition (unpartitioned table)
    let table = catalog.get_table(table_id)?;
    let table_guard = table.read();

    let partition = Partition::new(
        table_id, // Use table_id as partition_id for unpartitioned tables
        "lineitem".to_string(),
        1, // replication_num = 1 (from CREATE TABLE)
    );

    table_guard.add_partition(partition)?;
    println!("Added default partition to 'lineitem'");

    Ok(())
}

/// Create lineitem table definition
///
/// Schema reference: fe-backend-client/examples/minimal_mysql_client.rs:125-149
fn create_lineitem_table(db_id: i64) -> Result<OlapTable> {
    // Table ID allocation (using fixed IDs for now, can be dynamic later)
    const LINEITEM_TABLE_ID: i64 = 10001;

    // Define columns matching the CREATE TABLE statement
    let columns = vec![
        // Key column: l_orderkey (id=0)
        Column::new_key(0, "l_orderkey".to_string(), DataType::BigInt).with_position(0),
        // Other columns (non-key)
        Column::new(1, "l_partkey".to_string(), DataType::BigInt).with_position(1),
        Column::new(2, "l_suppkey".to_string(), DataType::BigInt).with_position(2),
        Column::new(3, "l_linenumber".to_string(), DataType::Int).with_position(3),
        Column::new(4, "l_quantity".to_string(), DataType::Decimal { precision: 15, scale: 2 }).with_position(4),
        Column::new(5, "l_extendedprice".to_string(), DataType::Decimal { precision: 15, scale: 2 }).with_position(5),
        Column::new(6, "l_discount".to_string(), DataType::Decimal { precision: 15, scale: 2 }).with_position(6),
        Column::new(7, "l_tax".to_string(), DataType::Decimal { precision: 15, scale: 2 }).with_position(7),
        Column::new(8, "l_returnflag".to_string(), DataType::Char { len: 1 }).with_position(8),
        Column::new(9, "l_linestatus".to_string(), DataType::Char { len: 1 }).with_position(9),
        Column::new(10, "l_shipdate".to_string(), DataType::Date).with_position(10),
        Column::new(11, "l_commitdate".to_string(), DataType::Date).with_position(11),
        Column::new(12, "l_receiptdate".to_string(), DataType::Date).with_position(12),
        Column::new(13, "l_shipinstruct".to_string(), DataType::Char { len: 25 }).with_position(13),
        Column::new(14, "l_shipmode".to_string(), DataType::Char { len: 10 }).with_position(14),
        Column::new(15, "l_comment".to_string(), DataType::Varchar { len: 44 }).with_position(15),
    ];

    // Create OLAP table with DUPLICATE KEY model
    let table = OlapTable::new(
        LINEITEM_TABLE_ID,
        "lineitem".to_string(),
        db_id,
        KeysType::DupKeys, // DUPLICATE KEY(l_orderkey)
        columns,
    );

    Ok(table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_tpch_schema() {
        let catalog = Catalog::new();

        // Load TPC-H schema
        load_tpch_schema(&catalog).expect("Failed to load TPC-H schema");

        // Verify database exists
        let db = catalog.get_database("tpch").expect("tpch database not found");
        let db_guard = db.read();
        assert_eq!(db_guard.name, "tpch");

        // Verify table exists
        let table = catalog.get_table_by_name("tpch", "lineitem")
            .expect("lineitem table not found");
        let table_guard = table.read();

        assert_eq!(table_guard.name, "lineitem");
        assert_eq!(table_guard.keys_type, KeysType::DupKeys);
        assert_eq!(table_guard.columns.len(), 16);

        // Verify key column
        let key_cols = table_guard.key_columns();
        assert_eq!(key_cols.len(), 1);
        assert_eq!(key_cols[0].name, "l_orderkey");

        // Verify some columns
        assert_eq!(table_guard.columns[0].name, "l_orderkey");
        assert_eq!(table_guard.columns[0].data_type, DataType::BigInt);
        assert_eq!(table_guard.columns[0].is_key, true);

        assert_eq!(table_guard.columns[1].name, "l_partkey");
        assert_eq!(table_guard.columns[1].data_type, DataType::BigInt);
        assert_eq!(table_guard.columns[1].is_key, false);

        // Verify partition exists
        assert_eq!(table_guard.partition_count(), 1);
    }
}
