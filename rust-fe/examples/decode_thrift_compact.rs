//! Thrift compact-protocol decoder for inspection of Doris payloads.
//!
//! This example reads one or more binary files (e.g. captured
//! `TPipelineFragmentParamsList` payloads from Java FE or Rust FE)
//! and prints the Thrift `Struct`/field tree using `thrift_codec`'s
//! dynamic model and the Compact protocol.
//!
//! Usage:
//!   cargo run --features real_be_proto --example decode_thrift_compact -- [files...]
//!
//! If no files are provided, it defaults to:
//!   - /tmp/rust-fe-thrift.bin
//!   - /tmp/java-fe-thrift-extracted.bin

use std::env;
use std::fs::File;
use std::io::Read;

use thrift_codec::{CompactDecode, Result as ThriftResult};
use thrift_codec::data::{Struct as ThriftStruct, Field as ThriftField, Data, DataRef};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        args = vec![
            "/tmp/rust-fe-thrift.bin".to_string(),
            "/tmp/java-fe-thrift-extracted.bin".to_string(),
        ];
    }

    for path in args {
        match decode_file(&path) {
            Ok(()) => {}
            Err(e) => eprintln!("Failed to decode '{}': {}", path, e),
        }
    }

    Ok(())
}

fn decode_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = Vec::new();
    File::open(path)?.read_to_end(&mut buf)?;

    let mut slice: &[u8] = &buf;
    let s = ThriftStruct::compact_decode(&mut slice)
        .map_err(|e| format!("failed to compact-decode Thrift struct: {e}"))?;

    println!("Decoded Thrift (compact) struct from '{}':", path);
    print_struct(&s, 0);
    println!();

    Ok(())
}

fn print_struct(s: &ThriftStruct, indent: usize) {
    for field in s.fields() {
        print_field(field, indent);
    }
}

fn print_field(field: &ThriftField, indent: usize) {
    let pad = "  ".repeat(indent);
    print!("{}- field {}: ", pad, field.id());
    match field.data() {
        Data::Struct(inner) => {
            println!("Struct {{");
            print_struct(inner, indent + 1);
            println!("{}}}", pad);
        }
        Data::List(list) => {
            println!("List(len={}) [", list.len());
            for (i, elem) in list.iter().enumerate() {
                print!("{}  [{}]: ", pad, i);
                print_data_ref(elem, indent + 1);
            }
            println!("{}]", pad);
        }
        Data::Map(map) => {
            println!("Map(len={}) {{", map.len());
            for (k, v) in map.iter() {
                print!("{}  ", pad);
                print_data_ref(k, indent + 1);
                print!(" => ");
                print_data_ref(v, indent + 1);
                println!();
            }
            println!("{}}}", pad);
        }
        other => {
            println!("{:?}", other);
        }
    }
}

fn print_data(_data: &Data, _indent: usize) {
    // Kept for parity with the binary decoder; currently unused.
}

fn print_data_ref(data: DataRef, indent: usize) {
    let pad = "  ".repeat(indent);
    match data {
        DataRef::Struct(inner) => {
            println!("Struct {{");
            print_struct(inner, indent + 1);
            println!("{}}}", pad);
        }
        DataRef::List(list) => {
            println!("List(len={}) [", list.len());
            for (i, elem) in list.iter().enumerate() {
                print!("{}  [{}]: ", pad, i);
                print_data_ref(elem, indent + 1);
            }
            println!("{}]", pad);
        }
        DataRef::Map(map) => {
            println!("Map(len={}) {{", map.len());
            for (k, v) in map.iter() {
                print!("{}  ", pad);
                print_data_ref(k, indent + 1);
                print!(" => ");
                print_data_ref(v, indent + 1);
                println!();
            }
            println!("{}}}", pad);
        }
        other => {
            print!("{:?}", other);
        }
    }
}

