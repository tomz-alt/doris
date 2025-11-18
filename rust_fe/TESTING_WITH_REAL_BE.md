# Testing Rust FE with Real C++ BE Data

## Prerequisites

Before you can test Rust FE queries against real data in C++ BE, you need:

1. ✅ Rust FE compiled (done)
2. ⚠️ **protoc installed** (BLOCKED - see PROTOC_INSTALLATION.md)
3. ⚠️ **Protobuf bindings generated** (needs protoc)
4. ❌ C++ BE running
5. ❌ Data loaded into BE

## Architecture Understanding

**CRITICAL**: You CANNOT load data into BE without an FE running first.

```
┌─────────────────────────────────────────────────────────┐
│  Doris Architecture                                     │
│                                                         │
│  FE (Frontend)                                          │
│  - Stores metadata (tables, schemas, tablet locations) │
│  - Coordinates queries                                  │
│  - Tells BE where data is stored                       │
│                                                         │
│  BE (Backend)                                           │
│  - Stores actual data                                   │
│  - Doesn't know what tables exist without FE           │
│  - Needs FE to tell it about schemas and partitions    │
└─────────────────────────────────────────────────────────┘
```

**Conclusion**: You must use either Java FE or Rust FE to create tables before BE can store data.

## Setup Approaches

### Approach 1: Java FE for Setup, Rust FE for Queries (Recommended)

This is the standard integration test approach:

#### Step 1: Start Backend
```bash
cd /path/to/doris/be
./bin/start_be.sh --daemon

# Verify BE is running
curl http://localhost:8040/api/health
```

#### Step 2: Start Java FE and Create Schema
```bash
cd /path/to/doris/fe
./bin/start_fe.sh --daemon

# Wait for FE to start
sleep 30

# Connect and create schema
mysql -h 127.0.0.1 -P 9030 -u root <<EOF
-- Create database
CREATE DATABASE tpch;
USE tpch;

-- Create lineitem table
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
) DUPLICATE KEY(l_orderkey)
DISTRIBUTED BY HASH(l_orderkey) BUCKETS 10;

-- Verify table created
SHOW CREATE TABLE lineitem;
EOF
```

#### Step 3: Load Data
```bash
# Option A: Insert sample data
mysql -h 127.0.0.1 -P 9030 -u root tpch <<EOF
INSERT INTO lineitem VALUES
(1, 155190, 7706, 1, 17.00, 21168.23, 0.04, 0.02, 'N', 'O', '1996-03-13', '1996-02-12', '1996-03-22', 'DELIVER IN PERSON', 'TRUCK', 'blithely regular ideas caj'),
(1, 67310, 7311, 2, 36.00, 45983.16, 0.09, 0.06, 'N', 'O', '1996-04-12', '1996-02-28', '1996-04-20', 'TAKE BACK RETURN', 'MAIL', 'slyly bold pinto beans'),
(2, 106170, 1191, 1, 38.00, 44694.46, 0.00, 0.05, 'N', 'O', '1997-01-28', '1997-01-14', '1997-02-02', 'TAKE BACK RETURN', 'RAIL', 'ven requests. deposits breach'),
(3, 4297, 1798, 1, 45.00, 54058.05, 0.10, 0.02, 'R', 'F', '1994-02-02', '1994-01-04', '1994-02-23', 'NONE', 'AIR', 'ongside of the furiously');

-- Verify data loaded
SELECT COUNT(*) FROM lineitem;
SELECT * FROM lineitem LIMIT 3;
EOF

# Option B: Stream Load (for large files)
# Prepare CSV file first
cat > /tmp/lineitem.csv <<EOF
5,108570,8571,1,15.00,22824.15,0.02,0.04,N,O,1997-05-02,1997-05-01,1997-05-28,NONE,TRUCK,pending requests sleep
6,139636,2150,1,32.00,49620.16,0.07,0.02,A,F,1992-02-21,1992-04-27,1992-03-30,DELIVER IN PERSON,MAIL,arefully
EOF

# Load via Stream Load API
curl --location-trusted -u root: \
  -H "label:lineitem_load_$(date +%s)" \
  -H "column_separator:," \
  -T /tmp/lineitem.csv \
  http://localhost:8040/api/tpch/lineitem/_stream_load

# Check load results
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT COUNT(*) FROM tpch.lineitem;"
```

#### Step 4: Export Metadata (FUTURE - Not Yet Implemented)
```bash
# This would export Java FE metadata for Rust FE
# NOT YET IMPLEMENTED in Rust FE

# Workaround: Recreate tables in Rust FE
mysql -h 127.0.0.1 -P 9030 -u root -e "SHOW CREATE TABLE tpch.lineitem" > /tmp/schema.sql
```

#### Step 5: Stop Java FE, Start Rust FE
```bash
# Stop Java FE
cd /path/to/doris/fe
./bin/stop_fe.sh

# Start Rust FE (needs protobuf bindings first!)
cd /home/user/doris/rust_fe
export PATH="$HOME/.local/bin:$PATH"  # If protoc installed there
cargo build --package fe-backend-client  # Generates protobuf bindings
cargo run --bin doris-fe

# In another terminal, connect to Rust FE
mysql -h 127.0.0.1 -P 9030 -u root
```

#### Step 6: Query from Rust FE
```sql
-- Connect to Rust FE
mysql -h 127.0.0.1 -P 9030 -u root

-- This should work once Rust FE has:
-- 1. Metadata about tpch.lineitem table
-- 2. gRPC client to BE (needs protobuf bindings)

USE tpch;
SELECT * FROM lineitem LIMIT 5;

-- TPC-H Q1
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

### Approach 2: Rust FE for Everything (Future Goal)

Once Rust FE is fully implemented:

```bash
# Start BE
cd /path/to/doris/be
./bin/start_be.sh --daemon

# Start Rust FE
cd /home/user/doris/rust_fe
cargo run --bin doris-fe

# Create tables via Rust FE
mysql -h 127.0.0.1 -P 9030 -u root <<EOF
CREATE DATABASE tpch;
USE tpch;
CREATE TABLE lineitem (...);
INSERT INTO lineitem VALUES (...);
SELECT * FROM lineitem LIMIT 5;
EOF
```

**Current blockers**:
- ⚠️ Metadata persistence (journal/checkpoint) not implemented
- ⚠️ BE communication requires protobuf bindings

## Current State: What Works Now

### Without C++ BE (Schema-only execution)
```bash
# Start Rust FE
cd /home/user/doris/rust_fe
cargo run --bin doris-fe

# Connect
mysql -h 127.0.0.1 -P 9030 -u root

# These work (no data, just schema):
CREATE DATABASE tpch;
USE tpch;
CREATE TABLE lineitem (...);

# Parses and plans, returns empty result set:
SELECT * FROM lineitem WHERE l_quantity > 10;

# Returns 10 columns (no data):
SELECT l_returnflag, sum(l_quantity) FROM lineitem GROUP BY l_returnflag;
```

## Missing Pieces for Real BE Integration

### 1. Protobuf Bindings (BLOCKED)
```bash
# Need this to work:
cargo build --package fe-backend-client
# Currently fails: "Could not find protoc"

# See PROTOC_INSTALLATION.md for workarounds
```

### 2. Implement Real BE Client
Once protobuf bindings are generated:

```rust
// fe-backend-client/src/lib.rs
impl BackendClient {
    pub async fn exec_plan_fragment(
        &mut self,
        fragment: &TPlanFragment,
        query_id: [u8; 16],
    ) -> Result<[u8; 16]> {
        // Convert TPlanFragment to PExecPlanFragmentRequest
        let request = PExecPlanFragmentRequest {
            request: fragment.serialize_to_bytes()?,
            compact: Some(false),
            version: Some(PFragmentRequestVersion::VERSION_1),
        };

        // Send gRPC request to BE
        let response = self.client.exec_plan_fragment(request).await?;

        // Extract fragment instance ID
        let finst_id = response.fragment_instance_id;
        Ok(finst_id)
    }

    pub async fn fetch_data(
        &mut self,
        finst_id: [u8; 16],
    ) -> Result<Vec<Row>> {
        // Send gRPC request to fetch results
        let request = PFetchDataRequest {
            finst_id: finst_id.to_vec(),
        };

        let response = self.client.fetch_data(request).await?;

        // Convert protobuf rows to Rust Row structs
        let rows = self.decode_rows(&response)?;
        Ok(rows)
    }
}
```

### 3. Metadata Synchronization
Need to implement:
- Journal replay (read Java FE's edit log)
- Or: Direct metadata export/import
- Or: FE cluster mode (Rust FE as follower)

## Testing Checklist

Once protobuf bindings are available:

- [ ] Compile Rust FE with BE client
- [ ] Start C++ BE on localhost:9060
- [ ] Start Java FE, create tables, load data
- [ ] Stop Java FE
- [ ] Start Rust FE
- [ ] Rust FE connects to BE successfully
- [ ] Execute simple SELECT: `SELECT * FROM lineitem LIMIT 5`
- [ ] Compare results with Java FE execution
- [ ] Execute TPC-H Q1
- [ ] Compare results (should be 100% identical)
- [ ] Performance comparison
- [ ] Error handling (non-existent table, syntax errors)

## Quick Start Script (Once protoc Available)

```bash
#!/bin/bash
# test-rust-fe-with-be.sh

set -e

echo "=== Starting Doris BE ==="
cd /path/to/doris/be
./bin/start_be.sh --daemon
sleep 10

echo "=== Starting Java FE and loading data ==="
cd /path/to/doris/fe
./bin/start_fe.sh --daemon
sleep 30

mysql -h 127.0.0.1 -P 9030 -u root <<EOF
CREATE DATABASE IF NOT EXISTS tpch;
USE tpch;
CREATE TABLE IF NOT EXISTS lineitem (...);
INSERT INTO lineitem VALUES (sample data...);
EOF

echo "=== Saving baseline results from Java FE ==="
mysql -h 127.0.0.1 -P 9030 -u root tpch -e "SELECT * FROM lineitem LIMIT 10" > /tmp/java_fe_results.txt

echo "=== Stopping Java FE ==="
./bin/stop_fe.sh

echo "=== Building Rust FE with BE client ==="
cd /home/user/doris/rust_fe
cargo build --release --package fe-backend-client
cargo build --release --bin doris-fe

echo "=== Starting Rust FE ==="
cargo run --release --bin doris-fe &
RUST_FE_PID=$!
sleep 10

echo "=== Testing Rust FE queries ==="
mysql -h 127.0.0.1 -P 9030 -u root tpch -e "SELECT * FROM lineitem LIMIT 10" > /tmp/rust_fe_results.txt

echo "=== Comparing results ==="
diff /tmp/java_fe_results.txt /tmp/rust_fe_results.txt && echo "✅ Results match!" || echo "❌ Results differ!"

echo "=== Cleanup ==="
kill $RUST_FE_PID
```

## Next Steps

1. **Get protoc installed** (see PROTOC_INSTALLATION.md for options):
   - Pre-generate bindings on dev machine
   - Manual protoc upload
   - Docker with protoc

2. **Generate protobuf bindings**:
   ```bash
   cargo build --package fe-backend-client
   ls -la fe-backend-client/src/generated/  # Should contain generated Rust files
   ```

3. **Implement real BE client** (replace MockBackend)

4. **Test with real BE** using scripts above

5. **Verify 100% identical results** vs Java FE
