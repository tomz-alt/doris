// Decode the hex dump byte-by-byte to understand the structure

fn main() {
    let bytes = std::fs::read("/tmp/rust_thrift_dump.bin")
        .expect("Read binary dump");

    println!("Total bytes: {}", bytes.len());
    println!("\nDetailed decode:\n");

    let mut pos = 0;

    println!("[{:04x}] TPipelineFragmentParamsList", pos);
    pos = decode_byte(pos, bytes[pos], "Field 1 header (list)", &bytes);
    pos = decode_byte(pos, bytes[pos], "List type & size", &bytes);
    pos = decode_byte(pos, bytes[pos], "List count", &bytes);

    println!("\n[{:04x}] TPipelineFragmentParams[0]", pos);
    pos = decode_byte(pos, bytes[pos], "Field 1 (protocol_version) header", &bytes);
    pos = decode_byte(pos, bytes[pos], "protocol_version value", &bytes);

    pos = decode_byte(pos, bytes[pos], "Field 2 (query_id) header", &bytes);
    println!("  → TUniqueId struct:");
    pos = decode_varint(pos, "  Field 1 (hi)", &bytes);
    pos = decode_varint(pos, "  Field 2 (lo)", &bytes);
    pos = decode_byte(pos, bytes[pos], "  Field stop", &bytes);

    pos = decode_byte(pos, bytes[pos], "Field 3 (fragment_id) header", &bytes);
    pos = decode_byte(pos, bytes[pos], "fragment_id value", &bytes);

    pos = decode_byte(pos, bytes[pos], "Field 4 (per_exch_num_senders) header", &bytes);
    pos = decode_byte(pos, bytes[pos], "Map size", &bytes);

    println!("\n[{:04x}] Field 5 (desc_tbl) - TDescriptorTable", pos);
    pos = decode_byte(pos, bytes[pos], "Field header", &bytes);
    pos = decode_descriptor_table(pos, &bytes);

    println!("\n[{:04x}] Field 7 (destinations) - CRITICAL!", pos);
    if pos < bytes.len() {
        pos = decode_byte(pos, bytes[pos], "Field header", &bytes);
        pos = decode_byte(pos, bytes[pos], "List type/size", &bytes);
        pos = decode_byte(pos, bytes[pos], "List count", &bytes);
    }

    println!("\n[{:04x}] Remaining bytes", pos);
    pos = decode_rest(pos, &bytes);
}

fn decode_byte(pos: usize, byte: u8, desc: &str, bytes: &[u8]) -> usize {
    println!("[{:04x}] 0x{:02x} - {}", pos, byte, desc);
    pos + 1
}

fn decode_varint(mut pos: usize, desc: &str) -> usize {
    print!("[{:04x}] ", pos);
    let start = pos;
    // Simplified varint decode
    while pos < 200 {  // safety limit
        let b = std::fs::read("/tmp/rust_thrift_dump.bin").unwrap()[pos];
        print!("{:02x} ", b);
        pos += 1;
        if (b & 0x80) == 0 {
            break;
        }
    }
    println!("- {}", desc);
    pos
}

fn decode_descriptor_table(mut pos: usize, bytes: &[u8]) -> usize {
    // Field 1: tuple_descriptors (list)
    if pos < bytes.len() {
        pos = decode_byte(pos, bytes[pos], "  Field 1 (tuple_descriptors) header", bytes);
        pos = decode_byte(pos, bytes[pos], "  List size", bytes);
    }

    // Field 2: slot_descriptors (optional list) - might not be present
    // Field 3: table_descriptors (optional list) - might not be present

    // Look for field stop
    while pos < bytes.len() && bytes[pos] != 0x00 {
        println!("[{:04x}] 0x{:02x} - TDescriptorTable field", pos, bytes[pos]);
        pos += 1;
        // Skip some bytes (simplified)
        if bytes[pos-1] == 0x19 || bytes[pos-1] == 0x1c {
            // List or struct, complex
            break;
        }
    }

    if pos < bytes.len() && bytes[pos] == 0x00 {
        pos = decode_byte(pos, bytes[pos], "  Field stop", bytes);
    }

    pos
}

fn decode_rest(mut pos: usize, bytes: &[u8]) -> usize {
    while pos < bytes.len() {
        println!("[{:04x}] 0x{:02x}", pos, bytes[pos]);
        pos += 1;
        if pos > 200 {  // safety
            break;
        }
    }
    pos
}
