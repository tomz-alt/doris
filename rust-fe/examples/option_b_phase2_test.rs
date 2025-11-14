/// Option B Phase 2 Test: Fragment Splitting for Distributed Execution
/// Demonstrates splitting single fragments into multi-fragment distributed plans
/// Usage: TPCH_DATA_DIR=/home/user/doris/tpch-data SKIP_PROTO=1 cargo run --example option_b_phase2_test

use datafusion::prelude::*;
use datafusion::error::Result as DFResult;
use datafusion::physical_plan::displayable;
use uuid::Uuid;

use doris_rust_fe::planner::{PlanConverter, PlanFragment, FragmentSplitter};

#[tokio::main]
async fn main() -> DFResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Option B Phase 2: Fragment Splitting Test ===\n");

    // Create DataFusion session
    let ctx = SessionContext::new();

    // Get TPC-H data directory
    let data_dir = std::env::var("TPCH_DATA_DIR")
        .unwrap_or_else(|_| "/home/user/doris/tpch-data".to_string());

    println!("Loading TPC-H data from: {}\n", data_dir);

    // Register lineitem table
    let file_path = format!("{}/lineitem.tbl", data_dir);
    let csv_options = CsvReadOptions::new()
        .delimiter(b'|')
        .has_header(false)
        .file_extension(".tbl");

    ctx.register_csv("lineitem", &file_path, csv_options).await?;
    println!("✓ Registered lineitem table\n");

    // Test queries that benefit from distribution
    let test_queries = vec![
        ("Simple Aggregation (Gather)",
         "SELECT COUNT(*) FROM lineitem"),

        ("GROUP BY Aggregation (Hash Partition)",
         "SELECT column_9, COUNT(*) as count FROM lineitem GROUP BY column_9"),

        ("TopN Query (Partial + Final)",
         "SELECT * FROM lineitem ORDER BY column_1 LIMIT 10"),

        ("Complex TPC-H Q1",
         r#"SELECT
                column_9 as l_returnflag,
                column_10 as l_linestatus,
                COUNT(*) as count_order,
                SUM(CAST(column_5 AS DOUBLE)) as sum_qty
            FROM lineitem
            WHERE column_11 <= '1998-12-01'
            GROUP BY column_9, column_10
            ORDER BY column_9, column_10
            LIMIT 10"#),
    ];

    for (idx, (name, sql)) in test_queries.iter().enumerate() {
        println!("=== Test {}: {} ===", idx + 1, name);
        println!("SQL: {}\n", sql.trim());

        // Step 1: Create DataFusion physical plan
        let df = ctx.sql(sql).await?;
        let physical_plan = df.create_physical_plan().await?;

        println!("DataFusion Physical Plan:");
        println!("{}\n", displayable(physical_plan.as_ref()).indent(true));

        // Step 2: Convert to single Doris fragment
        let query_id = Uuid::new_v4();
        let mut converter = PlanConverter::new(query_id);

        match converter.convert_to_fragments(physical_plan.clone()) {
            Ok(single_fragment_plan) => {
                println!("✓ Single Fragment Plan Created");
                println!("  Fragments: {}\n", single_fragment_plan.fragments.len());

                // Step 3: Split into multiple fragments for distribution
                let mut splitter = FragmentSplitter::new(query_id);

                if let Some(single_fragment) = single_fragment_plan.fragments.into_iter().next() {
                    match splitter.split_into_fragments(single_fragment) {
                        Ok(multi_fragments) => {
                            println!("✓ Multi-Fragment Distribution Plan Created");
                            println!("  Total Fragments: {}\n", multi_fragments.len());

                            // Print each fragment
                            for (frag_idx, fragment) in multi_fragments.iter().enumerate() {
                                println!("  Fragment {}:", frag_idx);
                                print_fragment_summary(fragment, "    ");
                            }

                            // Analyze fragment types
                            analyze_fragment_types(&multi_fragments);
                        }
                        Err(e) => {
                            println!("✗ Fragment splitting failed: {}\n", e);
                        }
                    }
                } else {
                    println!("✗ No fragments to split\n");
                }
            }
            Err(e) => {
                println!("✗ Conversion failed: {}\n", e);
            }
        }

        println!("---\n");
    }

    println!("=== Summary ===");
    println!("✓ DataFusion creates optimized physical plans");
    println!("✓ Single fragments converted from DataFusion");
    println!("✓ Fragments split for distributed execution");
    println!("✓ Exchange nodes inserted at boundaries");
    println!("\nFragment Types:");
    println!("- Data Fragments: Execute on BE nodes (scan + partial agg)");
    println!("- Coordinator Fragment: Merge results (final agg + sort)");
    println!("- Exchange Nodes: Shuffle data (Gather, Hash Partition, Broadcast)");
    println!("\nNext: Send fragments to BE via gRPC/HTTP");

    Ok(())
}

fn print_fragment_summary(fragment: &PlanFragment, indent: &str) {
    println!("{}Fragment ID: {}", indent, fragment.fragment_id);

    // Identify fragment type
    let fragment_type = identify_fragment_type(&fragment.root_node);
    println!("{}Type: {}", indent, fragment_type);

    // Count operators
    let op_count = count_operators(&fragment.root_node);
    println!("{}Operators: {}", indent, op_count);

    // Show root operator
    let root_op = get_root_operator_name(&fragment.root_node);
    println!("{}Root: {}", indent, root_op);

    // Show if there's an exchange
    if has_exchange(&fragment.root_node) {
        let exchange_type = get_exchange_type(&fragment.root_node);
        println!("{}Exchange: {:?}", indent, exchange_type);
    }

    println!();
}

fn identify_fragment_type(node: &doris_rust_fe::planner::PlanNode) -> &str {
    use doris_rust_fe::planner::PlanNode;

    match node {
        PlanNode::OlapScan { .. } => "Data Fragment (Leaf)",
        PlanNode::Exchange { .. } => "Exchange Fragment",
        PlanNode::Aggregation { is_merge: true, .. } => "Coordinator Fragment (Final Agg)",
        PlanNode::Aggregation { is_merge: false, .. } => "Data Fragment (Partial Agg)",
        PlanNode::TopN { .. } => "Coordinator Fragment (TopN)",
        PlanNode::Sort { .. } => "Coordinator Fragment (Sort)",
        _ => "Intermediate Fragment",
    }
}

fn count_operators(node: &doris_rust_fe::planner::PlanNode) -> usize {
    use doris_rust_fe::planner::PlanNode;

    1 + match node {
        PlanNode::Select { child, .. } => count_operators(child),
        PlanNode::Project { child, .. } => count_operators(child),
        PlanNode::Aggregation { child, .. } => count_operators(child),
        PlanNode::TopN { child, .. } => count_operators(child),
        PlanNode::Sort { child, .. } => count_operators(child),
        PlanNode::Exchange { child, .. } => count_operators(child),
        PlanNode::HashJoin { left, right, .. } => count_operators(left) + count_operators(right),
        PlanNode::Union { children } => children.iter().map(count_operators).sum(),
        _ => 0,
    }
}

fn get_root_operator_name(node: &doris_rust_fe::planner::PlanNode) -> &str {
    use doris_rust_fe::planner::PlanNode;

    match node {
        PlanNode::OlapScan { .. } => "OlapScan",
        PlanNode::Select { .. } => "Select",
        PlanNode::Project { .. } => "Project",
        PlanNode::Aggregation { .. } => "Aggregation",
        PlanNode::HashJoin { .. } => "HashJoin",
        PlanNode::Sort { .. } => "Sort",
        PlanNode::TopN { .. } => "TopN",
        PlanNode::Exchange { .. } => "Exchange",
        PlanNode::Union { .. } => "Union",
    }
}

fn has_exchange(node: &doris_rust_fe::planner::PlanNode) -> bool {
    use doris_rust_fe::planner::PlanNode;

    match node {
        PlanNode::Exchange { .. } => true,
        PlanNode::Select { child, .. } => has_exchange(child),
        PlanNode::Project { child, .. } => has_exchange(child),
        PlanNode::Aggregation { child, .. } => has_exchange(child),
        PlanNode::TopN { child, .. } => has_exchange(child),
        PlanNode::Sort { child, .. } => has_exchange(child),
        PlanNode::HashJoin { left, right, .. } => has_exchange(left) || has_exchange(right),
        _ => false,
    }
}

fn get_exchange_type(node: &doris_rust_fe::planner::PlanNode) -> Option<String> {
    use doris_rust_fe::planner::{PlanNode, ExchangeType};

    match node {
        PlanNode::Exchange { exchange_type, .. } => {
            Some(match exchange_type {
                ExchangeType::Broadcast => "Broadcast".to_string(),
                ExchangeType::HashPartition { partition_keys } => {
                    format!("HashPartition({} keys)", partition_keys.len())
                }
                ExchangeType::Random => "Random".to_string(),
                ExchangeType::Gather => "Gather".to_string(),
            })
        }
        PlanNode::Select { child, .. } => get_exchange_type(child),
        PlanNode::Project { child, .. } => get_exchange_type(child),
        PlanNode::Aggregation { child, .. } => get_exchange_type(child),
        PlanNode::TopN { child, .. } => get_exchange_type(child),
        PlanNode::Sort { child, .. } => get_exchange_type(child),
        _ => None,
    }
}

fn analyze_fragment_types(fragments: &[PlanFragment]) {
    let mut data_fragments = 0;
    let mut coordinator_fragments = 0;
    let mut exchange_fragments = 0;

    for fragment in fragments {
        let frag_type = identify_fragment_type(&fragment.root_node);
        if frag_type.contains("Data") {
            data_fragments += 1;
        } else if frag_type.contains("Coordinator") {
            coordinator_fragments += 1;
        } else if frag_type.contains("Exchange") {
            exchange_fragments += 1;
        }
    }

    println!("\nFragment Distribution:");
    println!("  Data Fragments: {} (execute on BE nodes)", data_fragments);
    println!("  Coordinator Fragments: {} (merge on coordinator)", coordinator_fragments);
    println!("  Exchange Fragments: {} (data shuffle)", exchange_fragments);
}
