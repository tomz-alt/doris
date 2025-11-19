use fe_planner::thrift_plan::TDescriptorTable;
use fe_planner::write_descriptor_table;
use thrift::protocol::TCompactOutputProtocol;
use thrift::transport::TBufferChannel;

fn main() {
    println!("=== TPC-H Lineitem Descriptor Table Test ===\n");

    // Create complete descriptor table for lineitem table
    let desc_tbl = TDescriptorTable::for_lineitem_table(0, 10001);

    println!("Created TDescriptorTable for lineitem:");
    println!("  Tuple ID: 0");
    println!("  Table ID: 10001");
    println!("  Number of columns: {}", desc_tbl.slot_descriptors.as_ref().unwrap().len());
    println!();

    // Print all slot descriptors
    if let Some(ref slots) = desc_tbl.slot_descriptors {
        println!("Slot Descriptors:");
        for (idx, slot) in slots.iter().enumerate() {
            println!(
                "  [{}] {} (id={}, type={:?}, is_key={:?})",
                idx,
                slot.col_name,
                slot.id,
                slot.primitive_type,
                slot.is_key
            );
        }
        println!();
    }

    // Serialize the descriptor table
    let mut transport = TBufferChannel::with_capacity(8192, 8192);
    {
        let mut protocol = TCompactOutputProtocol::new(&mut transport);
        match write_descriptor_table(&mut protocol, &desc_tbl) {
            Ok(_) => println!("✅ Descriptor table serialized successfully"),
            Err(e) => {
                eprintln!("❌ Serialization error: {:?}", e);
                return;
            }
        }
    }

    let bytes = transport.write_bytes();
    println!("Serialized size: {} bytes", bytes.len());
    println!();

    // Print hex dump
    println!("Hex dump of serialized descriptor table:");
    for (i, chunk) in bytes.chunks(16).enumerate() {
        print!("{:08x}  ", i * 16);
        for (j, byte) in chunk.iter().enumerate() {
            print!("{:02x} ", byte);
            if j == 7 {
                print!(" ");
            }
        }
        if chunk.len() < 16 {
            for _ in 0..(16 - chunk.len()) {
                print!("   ");
            }
        }
        print!(" |");
        for byte in chunk {
            if byte.is_ascii_graphic() || *byte == b' ' {
                print!("{}", *byte as char);
            } else {
                print!(".");
            }
        }
        println!("|");
    }
    println!();

    // Save to file
    let output_path = "/tmp/lineitem_descriptor.bin";
    std::fs::write(output_path, bytes).expect("Failed to write output file");
    println!("✅ Saved to: {}", output_path);
}
