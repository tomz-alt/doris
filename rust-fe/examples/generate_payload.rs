/// Standalone payload generator that creates a Thrift payload for TPCH lineitem
/// without connecting to BE. This is useful for comparing payloads.
///
/// Usage:
///   cargo run --features real_be_proto --example generate_payload

#[cfg(feature = "real_be_proto")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use uuid::Uuid;
    use doris_rust_fe::planner::plan_fragment::{PlanFragment, PlanNode, QueryPlan};
    use doris_rust_fe::be::thrift_pipeline::PipelineFragmentParamsList;

    println!("=== Generating Thrift payload for TPCH lineitem scan ===\n");

    // Build a simple QueryPlan with a single fragment rooted at an
    // OLAP scan of tpch.lineitem
    let query_id = Uuid::new_v4();
    let scan = PlanNode::OlapScan {
        table_name: "lineitem".to_string(),
        columns: vec![],
        predicates: vec![],
        tablet_ids: vec![],
    };
    let fragment = PlanFragment::new(0, scan);
    let mut plan = QueryPlan::new(query_id);
    plan.add_fragment(fragment);

    // Convert to Thrift
    let params_list = PipelineFragmentParamsList::from_query_plan(&plan)
        .ok_or("Failed to create pipeline params from query plan")?;

    let bytes = params_list.to_thrift_bytes();

    // Write to file
    let output_path = "/tmp/rust-fe-thrift.bin";
    std::fs::write(output_path, &bytes)?;

    println!("✓ Generated Thrift payload: {} bytes", bytes.len());
    println!("✓ Saved to: {}", output_path);
    println!("\nYou can now decode it with:");
    println!("  cargo run --example decode_thrift -- {}", output_path);

    Ok(())
}

#[cfg(not(feature = "real_be_proto"))]
fn main() {
    eprintln!("This example requires --features real_be_proto");
    eprintln!("Usage: cargo run --features real_be_proto --example generate_payload");
}
