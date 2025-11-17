use std::fs::File;
use std::io::Write;

use doris_rust_fe::be::load::BeLoadHelper;
use doris_rust_fe::be::doris::PBlock;
use doris_rust_fe::metadata::{catalog, schema::{Database, Table, ColumnDef}};
use doris_rust_fe::metadata::types::DataType;
use doris_rust_fe::query::ParsedInsert;
use prost::Message;

fn main() {
    // For now, generate a PBlock for the simple INT/VARCHAR test table
    // used in BeLoadHelper tests, and write it to target/pblock_simple.bin.
    let cat = catalog::catalog();

    // Register the pblock_test.simple table if it does not already exist.
    if !cat.database_exists("pblock_test") {
        let mut db = Database::new("pblock_test".to_string());
        db.add_table(
            Table::new("simple".to_string()).with_columns(vec![
                ColumnDef::new("id".to_string(), DataType::Int).not_null(),
                ColumnDef::new("name".to_string(), DataType::Varchar { length: 10 }).not_null(),
            ]),
        );
        cat.add_database(db);
    }

    let insert = ParsedInsert {
        database: "pblock_test".to_string(),
        table: "simple".to_string(),
        columns: vec!["id".to_string(), "name".to_string()],
        rows: vec![
            vec!["1".to_string(), "alice".to_string()],
            vec!["2".to_string(), "bob".to_string()],
        ],
    };

    let block: PBlock =
        BeLoadHelper::build_pblock_for_insert("pblock_test", "simple", &insert).expect(
            "PBlock build should succeed for pblock_test.simple (INT/VARCHAR) table",
        );

    let mut buf = Vec::new();
    block.encode(&mut buf).expect("encode PBlock");

    std::fs::create_dir_all("target").ok();
    let path = "target/pblock_simple.bin";
    let mut file = File::create(path).expect("create pblock_simple.bin");
    file.write_all(&buf).expect("write PBlock bytes");

    println!(
        "Wrote PBlock for pblock_test.simple ({} bytes) to {}",
        buf.len(),
        path
    );
}

