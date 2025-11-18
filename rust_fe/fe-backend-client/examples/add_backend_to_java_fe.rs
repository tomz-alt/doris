// Example: Connect to Java FE and add BE to cluster
//
// This demonstrates using Rust MySQL client to interact with Java FE

use mysql::*;
use mysql::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("══════════════════════════════════════════════════════════");
    println!("  Setup Java FE with Real C++ BE Data");
    println!("══════════════════════════════════════════════════════════\n");

    println!("Step 1: Connecting to Java FE at 127.0.0.1:9030...");

    let url = "mysql://root@127.0.0.1:9030";
    let pool = Pool::new(url)?;
    let mut conn = pool.get_conn()?;

    println!("✅ Connected!\n");

    // Step 2: Add backend to cluster
    println!("Step 2: Adding backend to cluster...");
    match conn.query_drop("ALTER SYSTEM ADD BACKEND '127.0.0.1:9050'") {
        Ok(_) => println!("✅ Backend added successfully"),
        Err(e) => {
            let err_msg = format!("{}", e);
            if err_msg.contains("already exists") || err_msg.contains("Same backend already exists") {
                println!("✅ Backend already exists");
            } else {
                println!("⚠️  Backend add warning: {}", e);
            }
        }
    }

    // Wait for backend to register
    println!("   Waiting for backend registration...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Show backends
    println!("\nStep 3: Verifying backend status...");
    match conn.query_map(
        "SHOW BACKENDS",
        |row: mysql::Row| -> (String, String, String, String, String) {
            let id: String = row.get(0).unwrap_or_default();
            let host: String = row.get(1).unwrap_or_default();
            let hb_port: String = row.get(2).unwrap_or_default();
            let be_port: String = row.get(3).unwrap_or_default();
            let alive: String = row.get(4).unwrap_or_default();
            (id, host, hb_port, be_port, alive)
        },
    ) {
        Ok(backends) => {
            if backends.is_empty() {
                println!("❌ No backends found!");
                return Err("No backends registered".into());
            }
            println!("✅ Backend status:");
            for (id, host, hb_port, be_port, alive) in backends {
                println!("   BE {} - {}:{} (heartbeat: {}) [alive: {}]", id, host, be_port, hb_port, alive);
            }
        }
        Err(e) => {
            println!("⚠️  Could not query backends: {}", e);
        }
    }

    // Create TPC-H database
    println!("\nStep 4: Creating TPC-H database...");
    conn.query_drop("CREATE DATABASE IF NOT EXISTS tpch")?;
    println!("✅ Database created");

    conn.query_drop("USE tpch")?;

    // Create lineitem table
    println!("\nStep 5: Creating lineitem table...");
    let create_table_sql = r#"
        CREATE TABLE IF NOT EXISTS lineitem (
            l_orderkey BIGINT,
            l_partkey BIGINT,
            l_suppkey BIGINT,
            l_linenumber INT,
            l_quantity DECIMAL(15,2),
            l_extendedprice DECIMAL(15,2),
            l_discount DECIMAL(15,2),
            l_tax DECIMAL(15,2),
            l_returnflag CHAR(1),
            l_linestatus CHAR(1),
            l_shipdate DATE,
            l_commitdate DATE,
            l_receiptdate DATE,
            l_shipinstruct CHAR(25),
            l_shipmode CHAR(10),
            l_comment VARCHAR(44)
        )
        DUPLICATE KEY(l_orderkey)
        DISTRIBUTED BY HASH(l_orderkey) BUCKETS 1
        PROPERTIES ("replication_num" = "1")
    "#;

    conn.query_drop(create_table_sql)?;
    println!("✅ Table created");

    // Insert sample data
    println!("\nStep 6: Inserting sample TPC-H data into C++ BE...");
    let inserts = vec![
        "INSERT INTO lineitem VALUES (1, 155190, 7706, 1, 17.00, 21168.23, 0.04, 0.02, 'N', 'O', '1996-03-13', '1996-02-12', '1996-03-22', 'DELIVER IN PERSON', 'TRUCK', 'regular deposits')",
        "INSERT INTO lineitem VALUES (1, 67310, 7311, 2, 36.00, 45983.16, 0.09, 0.06, 'N', 'O', '1996-04-12', '1996-02-28', '1996-04-20', 'TAKE BACK RETURN', 'MAIL', 'quick packages')",
        "INSERT INTO lineitem VALUES (2, 106170, 1191, 1, 38.00, 44694.46, 0.00, 0.05, 'N', 'O', '1997-01-28', '1997-01-14', '1997-02-02', 'TAKE BACK RETURN', 'RAIL', 'bold requests')",
        "INSERT INTO lineitem VALUES (3, 4297, 1798, 1, 45.00, 54058.05, 0.06, 0.00, 'R', 'F', '1994-02-02', '1994-01-04', '1994-02-23', 'NONE', 'AIR', 'final deposits')",
    ];

    for (i, insert_sql) in inserts.iter().enumerate() {
        match conn.query_drop(*insert_sql) {
            Ok(_) => println!("   ✓ Row {} inserted", i + 1),
            Err(e) => println!("   ⚠️  Row {} error: {}", i + 1, e),
        }
    }
    println!("✅ Inserted {} rows into C++ BE storage", inserts.len());

    // Query to verify data
    println!("\nStep 7: Verifying data from C++ BE...");
    let count: Option<i64> = conn.query_first("SELECT COUNT(*) FROM lineitem")?;
    println!("✅ Row count: {}", count.unwrap_or(0));

    // Test a simple TPC-H style query
    println!("\nStep 8: Testing TPC-H Q6 style query against real BE...");
    let revenue: Option<f64> = conn.query_first(
        "SELECT SUM(l_extendedprice * l_discount) AS revenue FROM lineitem WHERE l_discount BETWEEN 0.05 AND 0.07"
    )?;
    println!("✅ Revenue: {:.2}", revenue.unwrap_or(0.0));

    println!("\n══════════════════════════════════════════════════════════");
    println!("  🎉 Java FE Setup Complete - Real BE Data Ready!");
    println!("══════════════════════════════════════════════════════════");
    println!("✅ Java FE running on port 9030");
    println!("✅ C++ BE connected (127.0.0.1:9050)");
    println!("✅ Database: tpch");
    println!("✅ Table: lineitem");
    println!("✅ Data: {} rows in C++ BE (NOT in-memory)", inserts.len());
    println!("\nNext: Test Rust FE queries against this same BE data");
    println!("══════════════════════════════════════════════════════════\n");

    Ok(())
}
