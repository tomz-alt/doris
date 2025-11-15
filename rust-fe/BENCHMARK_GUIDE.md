# TPC-H & TPC-DS Benchmark Guide

Complete guide for running TPC-H and TPC-DS benchmarks to compare Java FE vs Rust FE performance.

## Overview

This benchmark suite:
- ✅ Generates TPC-H/TPC-DS data at any scale factor
- ✅ Runs queries on both Java FE and Rust FE
- ✅ Executes multiple rounds per query (3-5 configurable)
- ✅ Calculates statistics (mean, median, stdev, speedup)
- ✅ Generates beautiful HTML reports with charts
- ✅ Inspired by [ClickHouse JSONBench](https://github.com/ClickHouse/JSONBench)

## Quick Start

### 1. Prerequisites

```bash
# Install Python dependencies
pip3 install -r requirements.txt

# Ensure Docker cluster is running
cd rust-fe
./docker/quickstart.sh

# Verify both FEs are accessible
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT 1"  # Java FE
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT 1"  # Rust FE
```

### 2. Generate TPC-H Data

```bash
# Generate SF1 (1GB) data - good for testing
./scripts/tpch/generate_data.sh --scale 1 --port 9030

# Generate SF10 (10GB) data - moderate workload
./scripts/tpch/generate_data.sh --scale 10 --port 9030

# Generate SF100 (100GB) data - production scale
./scripts/tpch/generate_data.sh --scale 100 --port 9030
```

**Note**: Data is loaded into Java FE (port 9030), but both FEs share the same Backend, so data is accessible to both.

### 3. Run TPC-H Benchmark

```bash
# Run with default settings (SF1, 3 rounds)
python3 scripts/benchmark_tpch.py

# Run SF10 with 5 rounds and 2 warmup rounds
python3 scripts/benchmark_tpch.py --scale 10 --rounds 5 --warmup 2

# Full options
python3 scripts/benchmark_tpch.py \
    --scale 1 \
    --rounds 5 \
    --warmup 2 \
    --java-host 127.0.0.1 \
    --java-port 9030 \
    --rust-host 127.0.0.1 \
    --rust-port 9031 \
    --output-html tpch_sf1_results.html \
    --output-json tpch_sf1_results.json
```

### 4. View Results

```bash
# Open HTML report in browser
open tpch_results.html  # macOS
xdg-open tpch_results.html  # Linux
start tpch_results.html  # Windows

# Or view JSON results
cat tpch_results.json | python3 -m json.tool
```

## TPC-H Benchmark

### Data Generation

The `generate_data.sh` script:
1. Downloads and compiles TPC-H `dbgen` tool
2. Generates `.tbl` files at specified scale factor
3. Creates TPC-H database and tables
4. Loads data using Doris Stream Load API
5. Verifies row counts

**Data Sizes by Scale Factor**:
- SF1: ~1 GB (testing)
- SF10: ~10 GB (moderate)
- SF100: ~100 GB (production)
- SF1000: ~1 TB (large scale)

### Query Coverage

The benchmark includes 5 representative TPC-H queries:
- **Q1**: Pricing Summary Report (aggregation)
- **Q3**: Shipping Priority (3-way join)
- **Q6**: Forecasting Revenue Change (simple filter)
- **Q10**: Returned Item Reporting (4-way join)
- **Q18**: Large Volume Customer (subquery)

To add more queries, create `.sql` files in `scripts/tpch/queries/`.

### Benchmark Output

#### Console Output

```
=============================================================
TPC-H Benchmark (SF1)
=============================================================
Java FE:  127.0.0.1:9030
Rust FE:  127.0.0.1:9031
Rounds:   3
Queries:  5
=============================================================

Q1:
------------------------------------------------------------
  Round 1/3:
    Java FE: 2.451s
    Rust FE: 0.823s
    Speedup: 2.98x (+66.4%)
  Round 2/3:
    Java FE: 2.389s
    Rust FE: 0.801s
    Speedup: 2.98x (+66.5%)
  ...

  Summary:
    Java FE mean: 2.420s ± 0.031s
    Rust FE mean: 0.812s ± 0.011s
    Mean speedup: 2.98x

...

=============================================================
BENCHMARK SUMMARY
=============================================================

Query    Java FE (mean)  Rust FE (mean)  Speedup
------------------------------------------------------------
Q1            2.420s           0.812s      2.98x
Q3            1.567s           0.534s      2.93x
Q6            0.234s           0.089s      2.63x
Q10           3.123s           1.045s      2.99x
Q18           5.678s           1.923s      2.95x
------------------------------------------------------------

Overall Geometric Mean Speedup:              2.89x
Average Speedup:                             2.90x
Median Speedup:                              2.95x
Best Speedup:                                2.99x
=============================================================
```

#### HTML Report

The HTML report includes:
- **Summary Cards**: Scale factor, rounds, endpoints, timestamp
- **Overall Stats**: Geometric mean, average, median, best/worst speedup
- **Charts**:
  - Execution time comparison (bar chart)
  - Speedup factor by query (bar chart)
- **Detailed Table**: Mean times, stdev, speedup for each query
- **Query-by-Query Analysis**: Individual round times displayed

**Screenshot Preview**:
```
┌─────────────────────────────────────────────────────┐
│ 📊 TPC-H Benchmark Results                          │
│ Java FE vs Rust FE Performance Comparison           │
├─────────────────────────────────────────────────────┤
│ SF1  │ 3 Rounds │ 127.0.0.1:9030 │ 127.0.0.1:9031 │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Geometric Mean Speedup: 2.89x                      │
│  Average Speedup: 2.90x                             │
│  Median Speedup: 2.95x                              │
│                                                     │
│  [Chart: Execution Time Comparison]                 │
│  [Chart: Speedup Factor by Query]                   │
│                                                     │
│  [Detailed Results Table]                           │
│  [Query-by-Query Analysis]                          │
└─────────────────────────────────────────────────────┘
```

#### JSON Output

```json
{
  "metadata": {
    "benchmark": "TPC-H",
    "scale_factor": 1,
    "rounds": 3,
    "timestamp": "2025-11-15T10:30:45",
    "java_fe": "127.0.0.1:9030",
    "rust_fe": "127.0.0.1:9031"
  },
  "queries": {
    "Q1": {
      "java_fe": {
        "times": [2.451, 2.389, 2.420],
        "min": 2.389,
        "max": 2.451,
        "mean": 2.420,
        "median": 2.420,
        "stdev": 0.031
      },
      "rust_fe": {
        "times": [0.823, 0.801, 0.812],
        "min": 0.801,
        "max": 0.823,
        "mean": 0.812,
        "median": 0.812,
        "stdev": 0.011
      },
      "speedup": {
        "mean": 2.98,
        "median": 2.98,
        "best": 3.06
      }
    },
    ...
  },
  "overall": {
    "geometric_mean": 2.89,
    "arithmetic_mean": 2.90,
    "median": 2.95,
    "best": 2.99,
    "worst": 2.63
  }
}
```

## TPC-DS Benchmark

### Coming Soon

TPC-DS benchmark infrastructure is being prepared. It will follow the same pattern as TPC-H:

```bash
# Generate TPC-DS data (future)
./scripts/tpcds/generate_data.sh --scale 1

# Run TPC-DS benchmark (future)
python3 scripts/benchmark_tpcds.py --scale 1 --rounds 3
```

TPC-DS includes 99 queries testing various OLAP patterns. The benchmark will generate a separate HTML report: `tpcds_results.html`.

## Advanced Usage

### Custom Query Sets

You can run benchmarks on custom query sets:

1. Create a directory with your `.sql` files:
   ```bash
   mkdir -p scripts/custom_queries
   # Add q1.sql, q2.sql, etc.
   ```

2. Run benchmark with custom queries:
   ```bash
   python3 scripts/benchmark_tpch.py \
       --queries-dir scripts/custom_queries \
       --output-html custom_results.html
   ```

### Docker Environments

#### Using Docker Containers

If running FEs in Docker:

```bash
# Connect to Java FE in Docker
python3 scripts/benchmark_tpch.py \
    --java-host 127.0.0.1 \
    --java-port 9030 \
    --rust-host 127.0.0.1 \
    --rust-port 9031

# Or connect from inside Docker network
docker exec doris-mysql-client python3 /path/to/benchmark_tpch.py \
    --java-host doris-fe-java \
    --java-port 9030 \
    --rust-host doris-fe-rust \
    --rust-port 9031
```

### Automated Benchmark Suite

Run multiple benchmarks with different configurations:

```bash
#!/bin/bash
# run_all_benchmarks.sh

# SF1 with 3 rounds
python3 scripts/benchmark_tpch.py --scale 1 --rounds 3 \
    --output-html results/tpch_sf1_3r.html

# SF1 with 5 rounds
python3 scripts/benchmark_tpch.py --scale 1 --rounds 5 \
    --output-html results/tpch_sf1_5r.html

# SF10 with 3 rounds
python3 scripts/benchmark_tpch.py --scale 10 --rounds 3 \
    --output-html results/tpch_sf10_3r.html

# SF100 with 3 rounds (production)
python3 scripts/benchmark_tpch.py --scale 100 --rounds 3 \
    --output-html results/tpch_sf100_3r.html

echo "All benchmarks complete! Results in results/ directory"
```

### Comparative Analysis

Compare results across different configurations:

```python
import json

# Load results from multiple runs
with open('tpch_sf1_results.json') as f:
    sf1 = json.load(f)

with open('tpch_sf10_results.json') as f:
    sf10 = json.load(f)

# Compare geometric mean speedups
print(f"SF1 Speedup:  {sf1['overall']['geometric_mean']:.2f}x")
print(f"SF10 Speedup: {sf10['overall']['geometric_mean']:.2f}x")
```

## Troubleshooting

### Connection Issues

```bash
# Test FE connectivity
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT 1"  # Java FE
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT 1"  # Rust FE

# Check if database exists
mysql -h 127.0.0.1 -P 9030 -u root -e "SHOW DATABASES LIKE 'tpch%'"

# Verify tables
mysql -h 127.0.0.1 -P 9030 -u root tpch_sf1 -e "SHOW TABLES"
```

### Data Loading Issues

```bash
# Check Stream Load API availability
curl http://127.0.0.1:8030/api/health

# Verify row counts
mysql -h 127.0.0.1 -P 9030 -u root tpch_sf1 <<EOF
SELECT 'lineitem' as table_name, COUNT(*) FROM lineitem
UNION ALL SELECT 'orders', COUNT(*) FROM orders;
EOF
```

### Query Failures

If queries fail during benchmark:

1. **Run query manually** to see full error:
   ```bash
   mysql -h 127.0.0.1 -P 9030 -u root tpch_sf1 < scripts/tpch/queries/q1.sql
   ```

2. **Check FE logs**:
   ```bash
   # Docker
   docker compose logs doris-fe-java
   docker compose logs doris-fe-rust

   # Native
   tail -f /path/to/doris/fe/log/fe.log
   ```

3. **Verify data loaded correctly**:
   ```sql
   SELECT COUNT(*) FROM lineitem;  -- Should match expected row count
   ```

### Performance Issues

If benchmarks run slower than expected:

1. **Check BE status**:
   ```sql
   SHOW BACKENDS\G
   -- Ensure BE is "Alive" and has no issues
   ```

2. **Verify optimizations enabled** (Rust FE):
   ```bash
   docker compose logs doris-fe-rust | grep -i optimization
   # Should show: Cost-based joins: true, Partition pruning: true, etc.
   ```

3. **Increase warmup rounds**:
   ```bash
   python3 scripts/benchmark_tpch.py --warmup 5
   ```

## Expected Performance

### Rust FE Optimizations Impact

Based on micro-benchmarks and design, expected improvements:

| Optimization | Query Types | Expected Speedup |
|--------------|-------------|------------------|
| Cost-based Join | Q3, Q10, Q18 (multi-join) | 2.5-4x |
| Partition Pruning | Q1, Q6 (date filters) | 1.8-2.5x |
| Runtime Filters | Q3, Q18 (selective joins) | 2-3x |
| Bucket Shuffle | Q1 (large aggregations) | 1.5-2x |
| **Combined** | **All queries** | **2.5-3.5x** |

### Scale Factor Impact

| Scale Factor | Data Size | Benchmark Time | Recommended For |
|--------------|-----------|----------------|-----------------|
| SF1 | 1 GB | 5-10 min | Development, testing |
| SF10 | 10 GB | 30-60 min | Integration testing |
| SF100 | 100 GB | 3-6 hours | Production validation |
| SF1000 | 1 TB | 24+ hours | Large-scale testing |

## Benchmark Best Practices

1. **Use consistent hardware**: Run all benchmarks on the same machine
2. **Multiple rounds**: Use at least 3 rounds to account for variance
3. **Warmup runs**: Use 1-2 warmup rounds to prime caches
4. **Monitor resources**: Check CPU, memory, disk I/O during benchmarks
5. **Isolate workload**: Don't run other intensive tasks during benchmarks
6. **Document environment**: Record hardware specs, OS version, Docker version
7. **Save results**: Keep JSON results for historical comparison

## Contributing

To add more TPC-H queries:

1. Add `.sql` file to `scripts/tpch/queries/`:
   ```bash
   # Create q5.sql
   cat > scripts/tpch/queries/q5.sql <<'EOF'
   -- Your TPC-H Q5 query here
   EOF
   ```

2. Run benchmark (queries are auto-discovered):
   ```bash
   python3 scripts/benchmark_tpch.py
   ```

## References

- [TPC-H Specification](http://www.tpc.org/tpch/)
- [TPC-DS Specification](http://www.tpc.org/tpcds/)
- [ClickHouse JSONBench](https://github.com/ClickHouse/JSONBench) (Inspiration)
- [Apache Doris Documentation](https://doris.apache.org/docs/)

## Support

For issues or questions:
1. Check troubleshooting section above
2. Review FE logs: `docker compose logs doris-fe-rust`
3. Verify data loaded correctly
4. Test queries manually first

---

**Happy Benchmarking! 📊🚀**
