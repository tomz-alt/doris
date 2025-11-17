//! Decode Doris `TPipelineFragmentParamsList` payloads using the
//! official Apache Thrift Rust runtime and the generated `doris_thrift`
//! types.
//!
//! This is useful for differential testing between Java FE and Rust FE:
//! you can point it at a binary Thrift payload captured from either
//! implementation and inspect the typed Doris structs.
//!
//! Usage:
//!   cargo run --no-default-features --features real_be_proto \
//!     --example decode_compact_generated -- /path/to/thrift.bin

use std::env;
use std::fs::File;
use std::io::Read;

use thrift::protocol::{TCompactInputProtocol, TInputProtocol, TSerializable};
use thrift::transport::TBufferChannel;

use doris_thrift::palo_internal_service::TPipelineFragmentParamsList;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .expect("usage: decode_compact_generated <file>");

    let mut buf = Vec::new();
    File::open(&path)?.read_to_end(&mut buf)?;

    // Feed the captured payload into an in-memory Thrift transport.
    let mut channel = TBufferChannel::with_capacity(buf.len(), 0);
    channel.set_readable_bytes(&buf);

    let mut i_prot = TCompactInputProtocol::new(channel);
    let value = TPipelineFragmentParamsList::read_from_in_protocol(&mut i_prot)
        .map_err(|e| format!("failed to decode compact Thrift payload: {e}"))?;

    let num_params = value
        .params_list
        .as_ref()
        .map(|v| v.len())
        .unwrap_or(0);

    println!("Successfully decoded '{}'", path);
    println!("  params_list.len = {num_params}");

    if let Some(params_list) = value.params_list.as_ref() {
        for (idx, params) in params_list.iter().enumerate() {
            println!("  [{}].protocol_version = {:?}", idx, params.protocol_version);
            println!("  [{}].fragment_id      = {:?}", idx, params.fragment_id);
            println!("  [{}].table_name       = {:?}", idx, params.table_name);
            println!("  [{}].is_nereids       = {:?}", idx, params.is_nereids);

            if let Some(ref qg) = params.query_globals {
                println!("  [{}].query_globals.now_string = {}", idx, qg.now_string);
                println!("  [{}].query_globals.time_zone  = {:?}", idx, qg.time_zone);
            } else {
                println!("  [{}].query_globals = <none>", idx);
            }

            if let Some(ref qo) = params.query_options {
                println!(
                    "  [{}].query_options.batch_size / timeout / enable_pipeline_engine = {:?} / {:?} / {:?}",
                    idx, qo.batch_size, qo.query_timeout, qo.enable_pipeline_engine
                );
            } else {
                println!("  [{}].query_options = <none>", idx);
            }

            if idx == 0 {
                if let Some(ref desc) = params.desc_tbl {
                    let slots_len = desc.slot_descriptors.as_ref().map(|v| v.len()).unwrap_or(0);
                    let tuples_len = desc.tuple_descriptors.len();
                    println!("  [0].desc_tbl.slot_descriptors.len   = {slots_len}");
                    println!("  [0].desc_tbl.tuple_descriptors.len  = {tuples_len}");

                    if let Some(ref slots) = desc.slot_descriptors {
                        println!("  [0].desc_tbl.slot_descriptors (first up to 16):");
                        for (i, slot) in slots.iter().take(16).enumerate() {
                            println!(
                                "    slot[{}]: id={}, tuple_id={}, col_name={:?}, slot_idx={:?}",
                                i,
                                slot.id,
                                slot.parent,
                                slot.col_name,
                                slot.slot_idx,
                            );
                        }
                    }

                    println!("  [0].desc_tbl.tuple_descriptors:");
                    for t in &desc.tuple_descriptors {
                        println!(
                            "    tuple: id={}, table_id={:?}, num_null_bytes={:?}, num_null_slots={:?}",
                            t.id, t.table_id, t.num_null_bytes, t.num_null_slots
                        );
                    }
                } else {
                    println!("  [0].desc_tbl = <none>");
                }
            }

            if let Some(ref frag) = params.fragment {
                let nodes_len = frag
                    .plan
                    .as_ref()
                    .map(|p| p.nodes.len())
                    .unwrap_or(0);
                println!("  [{}].fragment.plan.nodes.len        = {nodes_len}", idx);

                if let Some(ref plan) = frag.plan {
                    if let Some(first_node) = plan.nodes.get(0) {
                        println!(
                            "  [{}].fragment.plan.nodes[0].output_tuple_id = {:?}",
                            idx, first_node.output_tuple_id
                        );
                        println!(
                            "  [{}].fragment.plan.nodes[0].output_slot_ids = {:?}",
                            idx, first_node.output_slot_ids
                        );
                    }
                }
            } else {
                println!("  [{}].fragment = <none>", idx);
            }

            let local_params_len = params
                .local_params
                .as_ref()
                .map(|v| v.len())
                .unwrap_or(0);
            println!("  [{}].local_params.len               = {}", idx, local_params_len);
            println!();
        }
    }

    Ok(())
}
