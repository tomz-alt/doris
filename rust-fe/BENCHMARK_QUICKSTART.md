# Benchmark Quick Start

## TL;DR - Run Everything

```bash
cd /home/user/doris/rust-fe

# Install dependencies (if needed)
apt-get install -y mysql-client bc

# Run full benchmark suite (5 rounds)
./scripts/run_full_benchmark.sh -r 5

# Results in: ./benchmark_results/
# - summary.html (open this first!)
# - tpch_results.html
# - tpcds_results.html
```

## One-Liners

**TPC-H only (22 queries, 5 rounds):**
```bash
./scripts/benchmark_tpch.sh -r 5
# Output: tpch_results.html
```

**TPC-DS only (99 queries, 5 rounds):**
```bash
./scripts/benchmark_tpcds.sh -r 5
# Output: tpcds_results.html
```

**Quick 3-round test:**
```bash
./scripts/run_full_benchmark.sh -r 3 --tpch-only
```

## Prerequisites Checklist

- [ ] MySQL client installed: `which mysql`
- [ ] bc calculator installed: `which bc`
- [ ] Java FE running on port 9030
- [ ] Rust FE running on port 9031
- [ ] TPC-H data loaded in `tpch_sf1` database
- [ ] TPC-DS data loaded in `tpcds_sf1` database

**Test connections:**
```bash
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT 1"  # Java FE
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT 1"  # Rust FE
```

## What You Get

```
benchmark_results/
├── summary.html          ← Open this first! Combined dashboard
├── tpch_results.html     ← TPC-H detailed charts (22 queries)
├── tpcds_results.html    ← TPC-DS detailed charts (99 queries)
├── tpch_results.json     ← Raw data for TPC-H
└── tpcds_results.json    ← Raw data for TPC-DS
```

## Common Options

```bash
# 5 rounds (recommended)
./scripts/run_full_benchmark.sh -r 5

# 3 rounds (quick test)
./scripts/run_full_benchmark.sh -r 3

# Custom output directory
./scripts/run_full_benchmark.sh -r 5 -o /tmp/bench_$(date +%Y%m%d)

# Scale factor 10 (10GB data)
./scripts/run_full_benchmark.sh -r 5 --scale 10

# TPC-H only
./scripts/run_full_benchmark.sh -r 5 --tpch-only

# TPC-DS only
./scripts/run_full_benchmark.sh -r 5 --tpcds-only
```

## Viewing Results

**Option 1: Local browser**
```bash
open benchmark_results/summary.html
```

**Option 2: Copy to local machine**
```bash
# On remote server
tar czf benchmark_results.tar.gz benchmark_results/

# On local machine
scp user@remote:/path/to/benchmark_results.tar.gz ./
tar xzf benchmark_results.tar.gz
open benchmark_results/summary.html
```

**Option 3: HTTP server**
```bash
cd benchmark_results
python3 -m http.server 8080
# Visit: http://localhost:8080/summary.html
```

## Expected Performance

**TPC-H (SF1, 22 queries):**
- Java FE: ~1.2-1.5 seconds geomean
- Rust FE: ~1.0-1.2 seconds geomean
- **Expected Speedup: 1.1-1.3x**

**TPC-DS (SF1, 99 queries):**
- Java FE: ~2.0-2.5 seconds geomean
- Rust FE: ~1.8-2.2 seconds geomean
- **Expected Speedup: 1.0-1.2x**

## Troubleshooting

**Connection failed:**
```bash
# Check Java FE
docker ps | grep doris-fe-java
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT 1"

# Check Rust FE
ps aux | grep doris-rust-fe
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT 1"
```

**Missing mysql command:**
```bash
apt-get update && apt-get install -y mysql-client
```

**Missing bc command:**
```bash
apt-get install -y bc
```

**Data not loaded:**
```bash
# Check TPC-H
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT COUNT(*) FROM tpch_sf1.lineitem"

# Check TPC-DS
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT COUNT(*) FROM tpcds_sf1.store_sales"
```

## What Gets Measured

For each query:
- **Warmup round:** 1 execution (discarded)
- **Measured rounds:** 5 executions (used for stats)
- **Metrics collected:**
  - Execution time (milliseconds)
  - Query result size
  - Success/failure status

**Summary statistics:**
- Geometric mean (main metric)
- Min/max times
- Standard deviation
- Speedup ratio (Java / Rust)

## Full Documentation

For complete details, see:
- **BENCHMARK_GUIDE.md** - Comprehensive guide
- **tools.md** - Quick reference commands
- **README_BENCHMARK.md** - Main overview

---

**Questions?** Check the full guide: `BENCHMARK_GUIDE.md`
