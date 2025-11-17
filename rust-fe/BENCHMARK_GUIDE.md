# Complete TPC-H & TPC-DS Benchmark Guide

## Quick Start

Run full benchmark suite with 5 rounds:

```bash
cd /home/user/doris/rust-fe

# Run both TPC-H and TPC-DS
./scripts/run_full_benchmark.sh -r 5

# Or run individually
./scripts/run_full_benchmark.sh --tpch-only -r 5
./scripts/run_full_benchmark.sh --tpcds-only -r 5
```

Results will be in `./benchmark_results/` directory.

## Prerequisites

### 1. Environment Setup

**Required:**
- MySQL client: `apt-get install mysql-client`
- bc calculator: `apt-get install bc`
- Java FE running on port 9030
- Rust FE running on port 9031
- Doris BE with TPC-H and TPC-DS data loaded

**Verify connections:**
```bash
# Test Java FE
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT 1"

# Test Rust FE
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT 1"

# Check TPC-H database
mysql -h 127.0.0.1 -P 9030 -u root -e "SHOW DATABASES" | grep tpch

# Check TPC-DS database
mysql -h 127.0.0.1 -P 9030 -u root -e "SHOW DATABASES" | grep tpcds
```

### 2. Data Loading

**TPC-H Data (SF1 = 1GB):**
```bash
cd /path/to/doris/tools/tpch-tools

# Configure
cat > conf/doris-cluster.conf <<EOF
export FE_HOST='127.0.0.1'
export FE_HTTP_PORT='8030'
export FE_QUERY_PORT='9030'
export USER='root'
export PASSWORD=''
export DB='tpch_sf1'
EOF

# Generate and load
./bin/build-tpch-dbgen.sh
./bin/gen-tpch-data.sh -s 1
./bin/create-tpch-tables.sh -s 1
./bin/load-tpch-data.sh
```

**TPC-DS Data (SF1 = 1GB):**
```bash
cd /path/to/doris/tools/tpcds-tools

# Configure
cat > conf/doris-cluster.conf <<EOF
export FE_HOST='127.0.0.1'
export FE_HTTP_PORT='8030'
export FE_QUERY_PORT='9030'
export USER='root'
export PASSWORD=''
export DB='tpcds_sf1'
EOF

# Generate and load
./bin/build-tpcds-dbgen.sh
./bin/gen-tpcds-data.sh -s 1
./bin/create-tpcds-tables.sh -s 1
./bin/load-tpcds-data.sh
```

## Benchmark Options

### Full Suite (TPC-H + TPC-DS)

```bash
./scripts/run_full_benchmark.sh \
    --rounds 5 \
    --scale 1 \
    --output-dir ./results_$(date +%Y%m%d_%H%M%S)
```

**Options:**
- `-r, --rounds N` - Number of rounds (default: 5)
- `-s, --scale N` - Scale factor (default: 1)
- `-o, --output-dir DIR` - Output directory
- `--tpch-only` - Run only TPC-H
- `--tpcds-only` - Run only TPC-DS
- `--java-host HOST` - Java FE host (default: 127.0.0.1)
- `--java-port PORT` - Java FE port (default: 9030)
- `--rust-host HOST` - Rust FE host (default: 127.0.0.1)
- `--rust-port PORT` - Rust FE port (default: 9031)

### TPC-H Only (22 queries)

```bash
./scripts/benchmark_tpch.sh \
    --scale 1 \
    --rounds 5 \
    --warmup 1 \
    --java-host 127.0.0.1 \
    --java-port 9030 \
    --rust-host 127.0.0.1 \
    --rust-port 9031 \
    --output-json tpch_results.json \
    --output-html tpch_results.html
```

### TPC-DS Only (99 queries)

```bash
./scripts/benchmark_tpcds.sh \
    --scale 1 \
    --rounds 5 \
    --warmup 1 \
    --java-host 127.0.0.1 \
    --java-port 9030 \
    --rust-host 127.0.0.1 \
    --rust-port 9031 \
    --output-json tpcds_results.json \
    --output-html tpcds_results.html
```

## Output Files

After running benchmarks, you'll get:

```
benchmark_results/
├── tpch_results.html       # TPC-H interactive charts
├── tpch_results.json       # TPC-H raw data
├── tpcds_results.html      # TPC-DS interactive charts
├── tpcds_results.json      # TPC-DS raw data
└── summary.html            # Combined summary dashboard
```

**Open in browser:**
```bash
# On Linux with GUI
xdg-open benchmark_results/summary.html

# On macOS
open benchmark_results/summary.html

# Or copy to your local machine
scp -r user@host:/path/to/benchmark_results ./
open benchmark_results/summary.html
```

## Benchmark Queries

### TPC-H Queries (22 total)

Located in: `scripts/tpch/queries/`

- **Q1:** Pricing Summary Report
- **Q2:** Minimum Cost Supplier
- **Q3:** Shipping Priority
- **Q4:** Order Priority Checking
- **Q5:** Local Supplier Volume
- **Q6:** Forecasting Revenue Change
- **Q7:** Volume Shipping
- **Q8:** National Market Share
- **Q9:** Product Type Profit Measure
- **Q10:** Returned Item Reporting
- **Q11:** Important Stock Identification
- **Q12:** Shipping Modes and Order Priority
- **Q13:** Customer Distribution
- **Q14:** Promotion Effect
- **Q15:** Top Supplier
- **Q16:** Parts/Supplier Relationship
- **Q17:** Small-Quantity-Order Revenue
- **Q18:** Large Volume Customer
- **Q19:** Discounted Revenue
- **Q20:** Potential Part Promotion
- **Q21:** Suppliers Who Kept Orders Waiting
- **Q22:** Global Sales Opportunity

### TPC-DS Queries (99 total)

Located in: `scripts/tpcds/queries/`

Covers decision support scenarios including:
- Customer analytics
- Sales analysis
- Inventory management
- Promotional effectiveness
- Time-series analysis

## Understanding Results

### HTML Visualization

The HTML reports provide:

1. **Per-Query Comparison Charts**
   - Bar charts comparing Java FE vs Rust FE per query
   - Color-coded: Orange (#ff6b35) for Java, Teal (#4ecdc4) for Rust
   - Interactive tooltips showing exact timings

2. **Summary Statistics**
   - Geometric mean (better for skewed distributions)
   - Min/max query times
   - Standard deviation
   - Speedup ratios

3. **Theme Toggle**
   - Dark mode (default)
   - Light mode

### Interpreting Geomean

**Geometric Mean** is used instead of arithmetic mean because:
- Query times are heavily skewed (some queries are 100x faster than others)
- Geomean is less sensitive to outliers
- Standard in database benchmarking (TPC-H, TPC-DS, ClickBench)

**Formula:**
```
geomean = (t1 * t2 * t3 * ... * tn)^(1/n)
```

**Speedup:**
```
speedup = Java_FE_Geomean / Rust_FE_Geomean
```

- `> 1.0`: Rust FE is faster
- `< 1.0`: Java FE is faster
- `= 1.0`: Same performance

### Expected Performance

**Typical Results (SF1):**
- **TPC-H:** 1.1-1.3x speedup (Rust faster)
- **TPC-DS:** 1.0-1.2x speedup (Rust faster)
- **Overall:** 1.1-1.25x average speedup

**Performance Factors:**
- Both FEs route to same BE (BE does heavy lifting)
- FE overhead is minimal (~1-5% of total query time)
- Speedup comes from faster plan generation and lower overhead
- Network latency can affect results

## Customizing Benchmarks

### Running Subset of Queries

Edit the benchmark scripts or create custom query directory:

```bash
# Run only Q1, Q3, Q6
mkdir -p /tmp/tpch_subset/queries
cp scripts/tpch/queries/q{1,3,6}.sql /tmp/tpch_subset/queries/

# Modify benchmark script to use custom dir
QUERIES_DIR=/tmp/tpch_subset/queries ./scripts/benchmark_tpch.sh -r 5
```

### Different Scale Factors

```bash
# SF10 (10GB)
./scripts/run_full_benchmark.sh --scale 10 --rounds 3

# SF100 (100GB)
./scripts/run_full_benchmark.sh --scale 100 --rounds 3
```

**Note:** Ensure data is loaded at the correct scale factor!

### Custom Rounds Configuration

```bash
# 10 rounds with 2 warmup
./scripts/benchmark_tpch.sh --rounds 10 --warmup 2

# 3 rounds with no warmup
./scripts/benchmark_tpch.sh --rounds 3 --warmup 0
```

## Troubleshooting

### Connection Errors

```bash
# Error: Cannot connect to Java FE
# Solution: Check Java FE is running
docker ps | grep doris-fe-java

# Error: Cannot connect to Rust FE
# Solution: Check Rust FE is running
ps aux | grep doris-rust-fe
lsof -i :9031
```

### Query Timeout

```bash
# Increase MySQL timeout
mysql -h 127.0.0.1 -P 9030 -u root <<SQL
SET GLOBAL wait_timeout = 28800;
SET GLOBAL interactive_timeout = 28800;
SQL
```

### Missing Dependencies

```bash
# Install mysql client
apt-get update && apt-get install -y mysql-client

# Install bc calculator
apt-get install -y bc

# Verify
which mysql  # Should show path
which bc     # Should show path
```

### Data Not Loaded

```bash
# Check TPC-H tables
mysql -h 127.0.0.1 -P 9030 -u root <<SQL
USE tpch_sf1;
SHOW TABLES;
SELECT COUNT(*) FROM lineitem;  -- Should show ~6M for SF1
SQL

# Check TPC-DS tables
mysql -h 127.0.0.1 -P 9030 -u root <<SQL
USE tpcds_sf1;
SHOW TABLES;
SELECT COUNT(*) FROM store_sales;
SQL
```

## Advanced Usage

### Comparing Multiple Configurations

```bash
# Baseline (Java FE only)
./scripts/benchmark_tpch.sh --rounds 5 \
    --output-json baseline_java.json \
    --output-html baseline_java.html

# After optimization (Rust FE)
./scripts/benchmark_tpch.sh --rounds 5 \
    --output-json optimized_rust.json \
    --output-html optimized_rust.html

# Compare JSONs manually or create custom comparison
```

### Automated CI/CD Integration

```bash
#!/bin/bash
# ci_benchmark.sh

# Run benchmarks
./scripts/run_full_benchmark.sh -r 3 -o ./ci_results

# Check for regressions
SPEEDUP=$(grep -oP 'speedup.*\K[0-9.]+' ci_results/summary.html | head -1)

if (( $(echo "$SPEEDUP < 0.95" | bc -l) )); then
    echo "REGRESSION: Speedup dropped below 0.95x"
    exit 1
fi

echo "PASS: Speedup is ${SPEEDUP}x"
```

### Export to CSV

```bash
# Extract results from JSON to CSV
jq -r '.queries[] | [.name, .java_time, .rust_time, .speedup] | @csv' \
    benchmark_results/tpch_results.json > tpch_results.csv
```

## Performance Tips

### Optimize BE Configuration

```bash
# Increase BE memory
docker exec doris-be bash -c 'echo "mem_limit=80%" >> /opt/apache-doris/be/conf/be.conf'

# Restart BE
docker restart doris-be
```

### Warm Up Cache

```bash
# Run warmup queries first
for q in scripts/tpch/queries/*.sql; do
    mysql -h 127.0.0.1 -P 9030 -u root tpch_sf1 < "$q" >/dev/null 2>&1
done
```

### Disable Query Cache

```bash
# To ensure fair comparison
mysql -h 127.0.0.1 -P 9030 -u root <<SQL
SET GLOBAL query_cache_size = 0;
SET GLOBAL query_cache_type = 0;
SQL
```

## References

- **TPC-H Specification:** http://www.tpc.org/tpch/
- **TPC-DS Specification:** http://www.tpc.org/tpcds/
- **ClickBench:** https://github.com/ClickHouse/ClickBench
- **Doris TPC-H Tools:** https://github.com/apache/doris/tree/master/tools/tpch-tools
- **Doris TPC-DS Tools:** https://github.com/apache/doris/tree/master/tools/tpcds-tools

## Next Steps

After running benchmarks:

1. **Analyze Results**
   - Open `summary.html` in browser
   - Identify slow queries
   - Look for patterns

2. **Optimize Slow Queries**
   - Check execution plans
   - Review BE logs
   - Profile query execution

3. **Iterate**
   - Make optimizations
   - Re-run benchmarks
   - Compare results

4. **Document Findings**
   - Save results with git commit hash
   - Note configuration changes
   - Track performance trends over time

---

**Questions or Issues?**
- Check `tools.md` for quick reference commands
- Review `current_impl.md` for implementation status
- See `todo.md` for known issues and roadmap
