# Instructions: Starting Local Doris Cluster for TPC-H Testing

## Goal
Run "mysql java jdbc → rust fe → C++ BE" end-to-end for TPC-H queries.

## Prerequisites

### 1. Build Doris (if not already built)

```bash
cd /home/user/doris

# Build C++ BE (takes ~30-60 minutes)
sh build.sh --be

# Build Java FE (takes ~10-20 minutes)
sh build.sh --fe
```

This creates:
- `/home/user/doris/output/be/` - C++ backend binary
- `/home/user/doris/output/fe/` - Java frontend JAR

### 2. Start Backend (BE)

```bash
cd /home/user/doris
./bin/start_be.sh --daemon

# Verify BE is running
ps aux | grep doris_be

# Check BE logs
tail -f be/log/be.INFO
```

BE will listen on:
- Port 9060: Thrift RPC (for FE → BE communication)
- Port 8060: HTTP (for monitoring)

### 3. Start Frontend (FE)

```bash
cd /home/user/doris
./bin/start_fe.sh --daemon

# Verify FE is running
ps aux | grep PaloFe

# Check FE logs
tail -f fe/log/fe.log
```

FE will listen on:
- Port 9030: MySQL protocol (for client connections)
- Port 9010: RPC (for FE-FE communication)
- Port 8030: HTTP (for monitoring)

### 4. Register BE with FE

```bash
# Connect to FE via MySQL protocol
mysql -h127.0.0.1 -P9030 -uroot

# Add BE to cluster
ALTER SYSTEM ADD BACKEND '127.0.0.1:9050';

# Verify BE is registered
SHOW BACKENDS\G
```

**OR** use our minimal_mysql_client:

```bash
cd /home/user/doris/rust_fe
cargo run --example minimal_mysql_client
```

This will:
- Register the BE
- Create `tpch` database
- Create `lineitem` table
- Insert 3 sample rows

### 5. Load TPC-H Data (Optional)

For real TPC-H testing, load actual data:

```bash
# Generate TPC-H data (scale factor 0.01 = ~60K lineitem rows)
docker run -v /tmp/tpch-data:/data \
  tpch/datagen:latest \
  -s 0.01 -f /data

# Load into Doris via STREAM LOAD
curl --location-trusted -u root: \
  -H "column_separator:|" \
  -T /tmp/tpch-data/lineitem.tbl \
  http://127.0.0.1:8030/api/tpch/lineitem/_stream_load
```

## Tablet Metadata Location

Once FE/BE are running, tablet metadata is available via:

### Method 1: MySQL Protocol (Simple)

```sql
-- Connect to Java FE
mysql -h127.0.0.1 -P9030 -uroot

-- Query tablet information
SELECT
  TABLE_NAME,
  TABLET_ID,
  BACKEND_ID,
  VERSION
FROM information_schema.tablets
WHERE TABLE_NAME = 'lineitem'
LIMIT 10;

-- Query backend information
SELECT
  BACKEND_ID,
  HOST,
  HEARTBEAT_PORT,
  BE_PORT
FROM information_schema.backends;
```

### Method 2: FE HTTP API (Detailed)

```bash
# Get table metadata (includes tablets)
curl http://127.0.0.1:8030/api/_get_table?db_name=tpch&table_name=lineitem

# Get backend list
curl http://127.0.0.1:8030/api/_get_backends
```

Response format:
```json
{
  "tablets": [
    {
      "tablet_id": 10003,
      "replicas": [
        {
          "backend_id": 10001,
          "host": "127.0.0.1",
          "be_port": 9060,
          "version": 2
        }
      ]
    }
  ]
}
```

### Method 3: FE Internal RPC (Production)

For production Rust FE, implement FE-to-FE RPC to query metadata directly:

```rust
// Connect to Java FE's internal RPC port (9010)
let fe_client = FeMetadataClient::connect("127.0.0.1:9010").await?;

// Query tablet locations
let tablets = fe_client.get_tablets("tpch", "lineitem").await?;

for tablet in tablets {
    println!("Tablet {}: backends {:?}",
        tablet.id, tablet.backend_locations);
}
```

## What Rust FE Needs

To execute queries, Rust FE needs:

1. **Table Schema** ✅ (already in catalog via `load_tpch_schema()`)
2. **Tablet IDs** ❌ (from Java FE metadata)
3. **Backend Addresses** ❌ (from Java FE metadata)
4. **Partition Info** ✅ (already in catalog)

## Current Workaround

Until FE/BE are running, use hardcoded metadata:

```rust
// Hardcoded for testing (assumes minimal_mysql_client ran)
let scan_ranges = vec![
    TScanRangeLocations {
        tablet_id: 10003, // First tablet from lineitem
        backend_ip: "127.0.0.1",
        backend_port: 9060,
        version: 2,
    }
];
```

## Verification

Once running, verify end-to-end:

```bash
# 1. Java FE is serving
mysql -h127.0.0.1 -P9030 -uroot -e "SELECT COUNT(*) FROM tpch.lineitem"

# 2. BE is accessible
curl http://127.0.0.1:8060/api/health

# 3. Rust FE can connect to BE
cd /home/user/doris/rust_fe
cargo run --example test_e2e_real_be
```

## Next Steps for Rust FE

1. Implement metadata fetcher (query Java FE via MySQL or HTTP)
2. Build TScanRangeLocations from real tablet data
3. Send query to BE
4. Parse PBlock results (already working!)
5. Return rows to MySQL client
