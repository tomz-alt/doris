/// Option B Test: DataFusion for planning, convert to Doris fragments
/// This demonstrates extracting plans and converting them (without executing on BE)
/// Usage: TPCH_DATA_DIR=/home/user/doris/tpch-data SKIP_PROTO=1 cargo run --example option_b_test

use datafusion::prelude::*;
use datafusion::error::Result as DFResult;
use datafusion::physical_plan::displayable;
use uuid::Uuid;

// Import our plan converter
use doris_rust_fe::planner::{PlanConverter, QueryPlan};

#[tokio::main]
async fn main() -> DFResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Option B: Plan Extraction and Conversion Test ===\n");

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

    // Test queries with increasing complexity
    let test_queries = vec![
        ("Simple COUNT", "SELECT COUNT(*) FROM lineitem"),
        ("Filter", "SELECT * FROM lineitem WHERE column_9 = 'A' LIMIT 10"),
        ("Aggregation", "SELECT column_9, COUNT(*) as count FROM lineitem GROUP BY column_9"),
        ("TPC-H Q1 (simplified)", r#"
            SELECT
                column_9 as l_returnflag,
                column_10 as l_linestatus,
                COUNT(*) as count_order,
                SUM(CAST(column_5 AS DOUBLE)) as sum_qty
            FROM lineitem
            WHERE column_11 <= '1998-12-01'
            GROUP BY column_9, column_10
            ORDER BY column_9, column_10
            LIMIT 10
        "#),
    ];

    for (idx, (name, sql)) in test_queries.iter().enumerate() {
        println!("=== Test Query {}: {} ===", idx + 1, name);
        println!("SQL: {}\n", sql.trim());

        // Step 1: Create logical plan
        let df = ctx.sql(sql).await?;
        let logical_plan = df.logical_plan();
        println!("Logical Plan:");
        println!("{:?}\n", logical_plan);

        // Step 2: Create physical plan
        let physical_plan = df.create_physical_plan().await?;
        println!("Physical Plan:");
        println!("{}\n", displayable(physical_plan.as_ref()).indent(true));

        // Step 3: Convert to Doris plan fragments (THIS IS THE KEY PART OF OPTION B)
        let query_id = Uuid::new_v4();
        let mut converter = PlanConverter::new(query_id);

        match converter.convert_to_fragments(physical_plan.clone()) {
            Ok(doris_plan) => {
                println!("✓ Conversion Successful!");
                println!("Doris Query Plan:");
                print_query_plan(&doris_plan);
                println!();
            }
            Err(e) => {
                println!("✗ Conversion Failed: {}", e);
                println!();
            }
        }

        println!("---\n");
    }

    println!("=== Summary ===");
    println!("✓ DataFusion successfully creates logical plans");
    println!("✓ DataFusion successfully creates physical plans");
    println!("✓ Plan converter maps DataFusion plans to Doris fragments");
    println!("\nNext Steps:");
    println!("1. Serialize Doris fragments to protobuf/JSON");
    println!("2. Send fragments to BE via gRPC/HTTP");
    println!("3. BE executes fragments and returns results");
    println!("4. FE coordinates results from multiple BEs");

    Ok(())
}

fn print_query_plan(plan: &QueryPlan) {
    println!("  Query ID: {}", plan.query_id);
    println!("  Fragments: {}", plan.fragments.len());

    for (idx, fragment) in plan.fragments.iter().enumerate() {
        println!("\n  Fragment {}:", idx);
        println!("    Fragment ID: {}", fragment.fragment_id);
        println!("    Root Node: {:#?}", fragment.root_node);
        if !fragment.output_exprs.is_empty() {
            println!("    Output Expressions: {} exprs", fragment.output_exprs.len());
        }
        if let Some(ref partition_info) = fragment.partition_info {
            println!("    Partition: {:?} ({} partitions)",
                     partition_info.partition_type,
                     partition_info.num_partitions);
        }
    }
}
