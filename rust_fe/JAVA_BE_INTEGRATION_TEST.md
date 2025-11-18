# Java BE Integration Test Plan

## Objective
Verify Rust FE produces **100% identical behavior** to Java FE when querying real data on Java BE.

## Test Setup

### 1. Start Java BE with TPC-H Data
```bash
# Start Java BE
cd /home/user/doris
./be/bin/start_be.sh --daemon

# Wait for BE to be ready
sleep 5
```

### 2. Load TPC-H Data via Java FE
```bash
# Start Java FE
./fe/bin/start_fe.sh --daemon

# Connect with MySQL CLI
mysql -h 127.0.0.1 -P 9030 -u root

# Create TPC-H schema and load data
CREATE DATABASE tpch;
USE tpch;

CREATE TABLE lineitem (
    l_orderkey INT,
    l_partkey INT,
    l_suppkey INT,
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
DUPLICATE KEY(l_orderkey, l_partkey, l_suppkey, l_linenumber)
DISTRIBUTED BY HASH(l_orderkey) BUCKETS 10;

# Load sample data (use STREAM LOAD or INSERT)
```

### 3. Execute TPC-H Q1 via Java FE (Baseline)
```sql
SELECT
    l_returnflag,
    l_linestatus,
    sum(l_quantity) as sum_qty,
    sum(l_extendedprice) as sum_base_price,
    sum(l_extendedprice * (1 - l_discount)) as sum_disc_price,
    sum(l_extendedprice * (1 - l_discount) * (1 + l_tax)) as sum_charge,
    avg(l_quantity) as avg_qty,
    avg(l_extendedprice) as avg_price,
    avg(l_discount) as avg_disc,
    count(*) as count_order
FROM lineitem
WHERE l_shipdate <= date '1998-12-01' - interval '90' day
GROUP BY l_returnflag, l_linestatus
ORDER BY l_returnflag, l_linestatus;
```

**Save results to**: `/tmp/java_fe_q1.csv`

### 4. Stop Java FE, Start Rust FE
```bash
# Stop Java FE
./fe/bin/stop_fe.sh

# Start Rust FE on same port
cd rust_fe
cargo run --bin doris-fe
```

### 5. Execute TPC-H Q1 via Rust FE
```bash
mysql -h 127.0.0.1 -P 9030 -u root

USE tpch;

# Same query as step 3
SELECT ...
```

**Save results to**: `/tmp/rust_fe_q1.csv`

### 6. Compare Results
```bash
diff /tmp/java_fe_q1.csv /tmp/rust_fe_q1.csv
# Should output: no differences
```

## Required Implementation

To make this work, Rust FE needs:

### A. Thrift RPC Client to BE
```rust
// fe-backend-client crate
pub struct BackendClient {
    client: TBackendServiceClient,
}

impl BackendClient {
    pub async fn exec_plan_fragment(&self, fragment: TPlanFragment) -> Result<QueryResult> {
        // Send Thrift RPC to BE
        // Receive results
        // Convert to ResultSet
    }
}
```

### B. Update QueryExecutor
```rust
fn execute_select(&self, stmt: &SelectStatement) -> Result<QueryResult> {
    // 1. Parse query
    // 2. Create plan via QueryPlanner
    // 3. Send to BE via BackendClient
    // 4. Return results
}
```

### C. Metadata Sync from BE
- Read tablet locations from BE
- Update catalog with BE metadata
- Handle partition info

## Success Criteria

✅ **Pass**: `diff` shows 0 differences between Java FE and Rust FE results
✅ **Pass**: Row count matches
✅ **Pass**: Column values match (within floating point precision)
✅ **Pass**: Column order matches
✅ **Pass**: Query execution time within 10% of Java FE

❌ **Fail**: Any difference in results
❌ **Fail**: Rust FE crashes or errors
❌ **Fail**: Different row count
