//! Thrift binary decoder for inspection of Doris payloads.
//!
//! This example reads a binary file (e.g. a captured
//! `TPipelineFragmentParamsList` from Java FE or Rust FE) and prints the
//! Thrift `Struct`/field tree using `thrift_codec`'s dynamic model.
//!
//! Usage:
//!   cargo run --example decode_thrift -- <path-to-bin>

use std::env;
use std::fs::File;
use std::io::Read;

use thrift_codec::{BinaryDecode, Result as ThriftResult};
use thrift_codec::data::{Struct as ThriftStruct, Field as ThriftField, Data, DataRef};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).expect("usage: decode_thrift <file>");

    let mut buf = Vec::new();
    File::open(&path)?.read_to_end(&mut buf)?;

    let mut slice: &[u8] = &buf;
    let s = ThriftStruct::binary_decode(&mut slice)
        .map_err(|e| format!("failed to decode Thrift struct: {e}"))?;

    println!("Decoded Thrift struct from '{}':", path);
    print_struct(&s, 0);

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

fn print_data(data: &Data, indent: usize) {
    let pad = "  ".repeat(indent);
    match data {
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
            print!("{:?}", other);
        }
    }
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

