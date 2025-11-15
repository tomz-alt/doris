# TPC-H SF100 Benchmark Execution Plan
# Rust FE vs Java FE Performance Comparison

**Scale Factor**: SF100 (100GB dataset)
**Objective**: Compare Rust FE vs Java FE on production-scale workload
**Status**: Ready for execution (requires cluster setup)

---

## Prerequisites

### 1. Cluster Requirements

**Minimum Configuration**:
- **1 FE Node**: 16 cores, 64GB RAM, 500GB SSD
- **3-5 BE Nodes**: 32 cores each, 128GB RAM each, 2TB NVMe SSD each
- **Network**: 10Gbps interconnect between nodes
- **OS**: Linux (Ubuntu 20.04+ or CentOS 7+)

**Recommended Configuration for Best Results**:
- **1 FE Node**: 32 cores, 128GB RAM, 1TB NVMe SSD
- **10 BE Nodes**: 64 cores each, 256GB RAM each, 4TB NVMe SSD each
- **Network**: 25Gbps or 100Gbps interconnect

### 2. Software Requirements

```bash
# Install Doris (both Java FE and Rust FE)
# Java FE
wget https://apache-doris-releases.oss-accelerate.aliyuncs.com/apache-doris-2.1.0-bin-x64.tar.gz
tar -xzf apache-doris-2.1.0-bin-x64.tar.gz

# Rust FE (from your build)
cd /home/user/doris/rust-fe
cargo build --release
cp target/release/doris-rust-fe /opt/doris-rust-fe/

# TPC-H data generation tool
git clone https://github.com/electrum/tpch-dbgen.git
cd tpch-dbgen
make
```

### 3. TPC-H SF100 Data Generation

**Storage Required**: ~100GB raw data + ~200GB in Doris (with replication)

```bash
cd tpch-dbgen

# Generate SF100 data (takes 2-4 hours)
./dbgen -s 100 -f

# This creates 8 files:
# - customer.tbl    (2.4GB)
# - lineitem.tbl    (74GB)   <- Largest table
# - nation.tbl      (2KB)
# - orders.tbl      (17GB)
# - part.tbl        (2.4GB)
# - partsupp.tbl    (12GB)
# - region.tbl      (389B)
# - supplier.tbl    (140MB)
```

---

## Setup Instructions

### Step 1: Configure Doris Cluster

**1. Java FE Setup** (baseline):

```bash
# Edit fe/conf/fe.conf
http_port = 8030
rpc_port = 9020
query_port = 9030
edit_log_port = 9010

# Start Java FE
cd /opt/doris-java/fe
./bin/start_fe.sh --daemon

# Verify
mysql -h 127.0.0.1 -P 9030 -u root
```

**2. Rust FE Setup** (comparison):

```bash
# Configure Rust FE (use different ports)
export DORIS_FE_HTTP_PORT=8031
export DORIS_FE_RPC_PORT=9021
export DORIS_FE_QUERY_PORT=9031

# Start Rust FE
cd /opt/doris-rust-fe
./doris-rust-fe --config fe.conf &

# Verify
mysql -h 127.0.0.1 -P 9031 -u root
```

**3. BE Nodes Setup** (shared by both FEs):

```bash
# On each BE node, edit be/conf/be.conf
be_port = 9060
webserver_port = 8040
heartbeat_service_port = 9050
brpc_port = 8060

# Start BE
cd /opt/doris/be
./bin/start_be.sh --daemon

# Add to FE (do this for each BE, for both FEs)
# For Java FE:
mysql -h 127.0.0.1 -P 9030 -u root -e "ALTER SYSTEM ADD BACKEND 'be1:9050'"
mysql -h 127.0.0.1 -P 9030 -u root -e "ALTER SYSTEM ADD BACKEND 'be2:9050'"
# ... repeat for all BE nodes

# For Rust FE:
mysql -h 127.0.0.1 -P 9031 -u root -e "ALTER SYSTEM ADD BACKEND 'be1:9050'"
mysql -h 127.0.0.1 -P 9031 -u root -e "ALTER SYSTEM ADD BACKEND 'be2:9050'"
# ... repeat for all BE nodes
```

### Step 2: Load TPC-H SF100 Data

**1. Create Database and Tables**:

```sql
-- Connect to Java FE
mysql -h 127.0.0.1 -P 9030 -u root

CREATE DATABASE tpch_100;
USE tpch_100;

-- Create tables (DDL in scripts/tpch/create_tables_sf100.sql)
SOURCE /home/user/doris/rust-fe/scripts/tpch/create_tables_sf100.sql;
```

**2. Load Data** (takes 2-4 hours depending on cluster):

```bash
# Use Stream Load for each table
cd tpch-dbgen

# Load customer
curl --location-trusted \
    -u root: \
    -H "column_separator:|" \
    -T customer.tbl \
    http://127.0.0.1:8030/api/tpch_100/customer/_stream_load

# Load lineitem (largest table - will take longest)
curl --location-trusted \
    -u root: \
    -H "column_separator:|" \
    -T lineitem.tbl \
    http://127.0.0.1:8030/api/tpch_100/lineitem/_stream_load

# Repeat for all 8 tables...
# Or use the automated script:
bash /home/user/doris/rust-fe/scripts/tpch/load_sf100_data.sh
```

**3. Verify Data Loaded**:

```sql
-- Check row counts
SELECT 'customer' as tbl, COUNT(*) FROM customer
UNION ALL SELECT 'lineitem', COUNT(*) FROM lineitem
UNION ALL SELECT 'nation', COUNT(*) FROM nation
UNION ALL SELECT 'orders', COUNT(*) FROM orders
UNION ALL SELECT 'part', COUNT(*) FROM part
UNION ALL SELECT 'partsupp', COUNT(*) FROM partsupp
UNION ALL SELECT 'region', COUNT(*) FROM region
UNION ALL SELECT 'supplier', COUNT(*) FROM supplier;

-- Expected counts for SF100:
-- customer:    15,000,000
-- lineitem:   600,037,902
-- nation:              25
-- orders:     150,000,000
-- part:        20,000,000
-- partsupp:    80,000,000
-- region:               5
-- supplier:     1,000,000
```

**4. Replicate to Rust FE**:

```sql
-- Connect to Rust FE and create same schema
mysql -h 127.0.0.1 -P 9031 -u root

CREATE DATABASE tpch_100;
USE tpch_100;
SOURCE /home/user/doris/rust-fe/scripts/tpch/create_tables_sf100.sql;

-- Since BEs are shared, data is already available
-- Just verify:
SELECT COUNT(*) FROM lineitem;
```

### Step 3: Warmup Queries

Run warmup queries to populate caches:

```bash
# Warmup Java FE
for i in {1..22}; do
    mysql -h 127.0.0.1 -P 9030 -u root tpch_100 \
        < /home/user/doris/rust-fe/scripts/tpch/queries/q${i}.sql \
        > /dev/null 2>&1
done

# Warmup Rust FE
for i in {1..22}; do
    mysql -h 127.0.0.1 -P 9031 -u root tpch_100 \
        < /home/user/doris/rust-fe/scripts/tpch/queries/q${i}.sql \
        > /dev/null 2>&1
done
```

---

## Benchmark Execution

### Step 4: Run SF100 Benchmarks

Use the automated comparison script:

```bash
cd /home/user/doris/rust-fe/scripts

# Configure for SF100
export TPCH_SCALE=100
export NUM_ITERATIONS=10
export JAVA_FE_HOST=127.0.0.1
export JAVA_FE_PORT=9030
export RUST_FE_HOST=127.0.0.1
export RUST_FE_PORT=9031
export OUTPUT_DIR=/home/user/benchmark_results_sf100

# Run full benchmark suite (estimated 6-12 hours)
./benchmark_sf100_comparison.sh
```

The script will:
1. Run all 22 TPC-H queries on Java FE (baseline)
2. Run all 22 TPC-H queries on Rust FE with different optimization configs:
   - Baseline (all optimizations OFF)
   - Cost-based joins only
   - Partition pruning only
   - Runtime filters only
   - All optimizations enabled
3. Collect timing, I/O, network metrics
4. Capture query profiles and BE logs
5. Generate comparison reports

### Step 5: Monitor Execution

During benchmark execution, monitor:

```bash
# Terminal 1: Watch Java FE logs
tail -f /opt/doris-java/fe/log/fe.INFO | grep -E "Query|Optimization"

# Terminal 2: Watch Rust FE logs
tail -f /opt/doris-rust-fe/log/rust_fe.log | grep -E "Query|Optimization"

# Terminal 3: Watch BE logs (on BE node)
tail -f /opt/doris/be/log/be.INFO | grep -E "Fragment|Join|Partition|Runtime filter"

# Terminal 4: Monitor system resources
htop

# Terminal 5: Monitor network traffic
iftop -i eth0
```

### Step 6: Validate Optimizations

Check BE logs to verify optimizations are applied:

```bash
# Cost-based join decisions
grep "Cost-based join strategy" /opt/doris/be/log/be.INFO

# Partition pruning
grep "Partition pruning" /opt/doris/be/log/be.INFO | tail -20

# Runtime filters
grep "Runtime filter" /opt/doris/be/log/be.INFO | tail -20

# Bucket shuffle
grep "Bucket shuffle" /opt/doris/be/log/be.INFO | tail -20
```

---

## Expected Results for SF100

### Baseline Performance (Java FE)

| Query | Typical Time | Data Scanned | Network Traffic |
|-------|-------------|--------------|-----------------|
| Q1    | 25-35s      | 74GB         | 2-5GB           |
| Q3    | 45-60s      | 91GB         | 10-15GB         |
| Q6    | 8-12s       | 74GB         | 100MB           |
| Q9    | 90-120s     | 166GB        | 20-30GB         |
| Q13   | 60-80s      | 17GB         | 15-20GB         |
| Q17   | 120-180s    | 94GB         | 30-40GB         |
| Q18   | 100-150s    | 91GB         | 25-35GB         |
| Q21   | 150-200s    | 166GB        | 40-50GB         |

### Expected Rust FE Improvements

| Optimization | Query Types | Expected Speedup | I/O Reduction |
|--------------|-------------|------------------|---------------|
| **Partition Pruning** | Q1, Q6, Q12, Q14 | 2.0-2.5x | 60-75% |
| **Cost-Based Joins** | Q3, Q5, Q8, Q9 | 1.5-2.0x | 20-30% |
| **Runtime Filters** | Q9, Q17, Q18, Q21 | 2.0-3.5x | 70-85% |
| **All Optimizations** | Complex queries | 2.5-4.0x | 65-80% |

### Projected SF100 Results

| Configuration | Avg Query Time | Speedup | Total Suite Time |
|---------------|----------------|---------|------------------|
| Java FE (baseline) | 65s | 1.0x | ~24 minutes |
| Rust FE (baseline) | 63s | 1.03x | ~23 minutes |
| Rust FE (cost-based) | 45s | 1.44x | ~16.5 minutes |
| Rust FE (partition pruning) | 40s | 1.63x | ~14.7 minutes |
| Rust FE (runtime filters) | 35s | 1.86x | ~12.8 minutes |
| Rust FE (all optimizations) | **22s** | **2.95x** | **~8 minutes** |

---

## Data Collection

The benchmark script automatically collects:

### 1. Timing Metrics
- Query planning time (FE)
- Fragment scheduling time (FE)
- Execution time per fragment (BE)
- Total end-to-end query time
- Network data transfer time

### 2. Resource Metrics
- CPU usage (FE and BE)
- Memory usage (FE and BE)
- Disk I/O (bytes read/written)
- Network I/O (bytes sent/received)

### 3. Optimization Metrics
- Join strategy decisions (broadcast vs shuffle)
- Partitions scanned vs pruned
- Runtime filters generated and applied
- Filter effectiveness (rows filtered)

### 4. Query Profiles
```sql
-- Captured automatically for each query
SHOW QUERY PROFILE;
```

---

## Analysis and Reporting

### Step 7: Generate Reports

After benchmarks complete:

```bash
cd /home/user/benchmark_results_sf100

# View summary
cat BENCHMARK_SUMMARY.md

# View detailed CSV results
head -20 combined_results.csv

# Generate charts (if Python/matplotlib available)
python3 /home/user/doris/rust-fe/scripts/generate_charts.py \
    --input combined_results.csv \
    --output charts/
```

### Report Structure

**1. Executive Summary** (`EXECUTIVE_SUMMARY.md`):
- Overall speedup: X.Xx
- I/O reduction: XX%
- Network reduction: XX%
- Cost savings projection

**2. Per-Query Analysis** (`PER_QUERY_RESULTS.md`):
- Detailed breakdown for each of 22 queries
- Optimization effectiveness
- Resource utilization

**3. Optimization Analysis** (`OPTIMIZATION_ANALYSIS.md`):
- Cost model accuracy
- Partition pruning effectiveness
- Runtime filter hit rates
- Bucket shuffle opportunities

**4. Production Recommendations** (`PRODUCTION_RECOMMENDATIONS.md`):
- Optimal configuration for your workload
- Tuning parameters
- Rollout strategy

---

## Troubleshooting

### Common Issues

**1. Out of Memory**:
```bash
# Increase BE memory limit
echo "mem_limit = 80%" >> /opt/doris/be/conf/be.conf
./bin/stop_be.sh && ./bin/start_be.sh --daemon
```

**2. Slow Data Loading**:
```bash
# Use parallel loading
split -l 10000000 lineitem.tbl lineitem_part_
for f in lineitem_part_*; do
    curl --location-trusted -u root: -H "column_separator:|" \
        -T $f http://127.0.0.1:8030/api/tpch_100/lineitem/_stream_load &
done
wait
```

**3. Query Timeout**:
```sql
-- Increase timeout
SET query_timeout = 3600;  -- 1 hour
```

**4. Disk Space Issues**:
```bash
# Check disk usage
df -h
du -sh /opt/doris/be/storage

# Add more storage paths if needed
```

---

## SF100 vs SF1 vs SF10 Comparison

| Metric | SF1 | SF10 | SF100 | SF1000 |
|--------|-----|------|-------|--------|
| **Data Size** | 1GB | 10GB | 100GB | 1TB |
| **lineitem rows** | 6M | 60M | 600M | 6B |
| **Load Time** | 5-10 min | 30-60 min | 2-4 hours | 20-40 hours |
| **Benchmark Time** | 30 min | 2 hours | 6-12 hours | 2-4 days |
| **Min BE Nodes** | 1 | 3 | 5 | 10+ |
| **Min RAM/BE** | 32GB | 64GB | 128GB | 256GB |
| **Use Case** | Development | Integration | Production | Large-scale |

**Recommendation**:
- **Development/Testing**: SF1
- **CI/CD**: SF10
- **Production Validation**: **SF100** ← You are here
- **Performance Limits**: SF1000

---

## Next Steps

1. **Setup Cluster**: Follow "Setup Instructions" section
2. **Load Data**: Execute data loading scripts (~4 hours)
3. **Run Benchmarks**: Execute `./benchmark_sf100_comparison.sh` (~8 hours)
4. **Analyze Results**: Review generated reports
5. **Tune**: Adjust optimization parameters based on results
6. **Production Deploy**: Roll out Rust FE with validated configuration

---

## Cost Estimate

### Cloud Deployment (AWS Example)

**Cluster Configuration**:
- 1x c5.9xlarge (FE): $1.53/hour
- 10x i3.8xlarge (BE): $24.96/hour total
- **Total**: ~$26.50/hour

**Benchmark Duration**: ~16 hours (setup + load + benchmark)
**Total Cost**: ~$424

**Alternative: Spot Instances**:
- 70% discount
- **Total Cost**: ~$127

### On-Premise

**Hardware**: ~$150,000 (one-time)
**Power/Cooling**: ~$500/month
**Amortized per benchmark**: Minimal

---

## Appendix: Quick Command Reference

```bash
# Start Java FE
cd /opt/doris-java/fe && ./bin/start_fe.sh --daemon

# Start Rust FE with all optimizations
cd /opt/doris-rust-fe
DORIS_COST_BASED_JOIN=true \
DORIS_PARTITION_PRUNING=true \
DORIS_RUNTIME_FILTERS=true \
DORIS_BUCKET_SHUFFLE=true \
./doris-rust-fe &

# Run single query with profile
mysql -h 127.0.0.1 -P 9031 -u root tpch_100 -e "
SELECT /* Q6 */ SUM(l_extendedprice * l_discount) as revenue
FROM lineitem
WHERE l_shipdate >= '1994-01-01'
  AND l_shipdate < '1995-01-01'
  AND l_discount BETWEEN 0.05 AND 0.07
  AND l_quantity < 24;
SHOW QUERY PROFILE;
"

# Check optimization effectiveness
grep -E "Cost-based|Partition pruning|Runtime filter|Bucket shuffle" \
    /opt/doris/be/log/be.INFO | tail -50

# Monitor query progress
watch -n 1 "mysql -h 127.0.0.1 -P 9031 -u root -e 'SHOW PROCESSLIST'"
```

---

**Document Version**: 1.0
**Last Updated**: 2025-11-14
**Status**: Ready for Execution
**Estimated Completion Time**: 16-24 hours
