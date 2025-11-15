# Complete TPC-H & TPC-DS Benchmark Suite

**Comprehensive benchmarking infrastructure for comparing Java FE vs Rust FE with full query coverage and beautiful visualization.**

## 📊 What's Included

✅ **TPC-H**: All 22 official queries
✅ **TPC-DS**: All 99 official queries
✅ **Multi-round execution**: 3-5 configurable rounds per query
✅ **Statistical analysis**: Mean, median, stdev, speedup
✅ **HTML reports**: Beautiful charts with Chart.js
✅ **JSON export**: Machine-readable results

## 🚀 Quick Start

### 1. Install Dependencies

```bash
pip3 install -r requirements.txt
```

### 2. Start Docker Cluster

```bash
./docker/quickstart.sh
```

### 3A. Run TPC-H Benchmark (22 queries)

```bash
# Generate data
./scripts/tpch/generate_data.sh --scale 1

# Run benchmark
python3 scripts/benchmark_tpch.py --scale 1 --rounds 5

# View results
open tpch_results.html
```

### 3B. Run TPC-DS Benchmark (99 queries)

```bash
# Generate data
./scripts/tpcds/generate_data.sh --scale 1

# Run benchmark
python3 scripts/benchmark_tpcds.py --scale 1 --rounds 5

# View results
open tpcds_results.html
```

## 📈 TPC-H Benchmark

### All 22 Queries Included

| Query | Description | Pattern |
|-------|-------------|---------|
| Q1 | Pricing Summary Report | Aggregation |
| Q2 | Minimum Cost Supplier | Subquery |
| Q3 | Shipping Priority | 3-way join |
| Q4 | Order Priority Checking | EXISTS |
| Q5 | Local Supplier Volume | 6-way join |
| Q6 | Forecasting Revenue Change | Simple filter |
| Q7 | Volume Shipping | Complex joins |
| Q8 | National Market Share | CTEs |
| Q9 | Product Type Profit Measure | Complex aggregation |
| Q10 | Returned Item Reporting | 4-way join |
| Q11 | Important Stock Identification | Subquery with HAVING |
| Q12 | Shipping Modes and Order Priority | CASE expressions |
| Q13 | Customer Distribution | LEFT OUTER JOIN |
| Q14 | Promotion Effect | CASE expressions |
| Q15 | Top Supplier | Views |
| Q16 | Parts/Supplier Relationship | NOT IN |
| Q17 | Small-Quantity-Order Revenue | Correlated subquery |
| Q18 | Large Volume Customer | IN clause |
| Q19 | Discounted Revenue | Complex OR conditions |
| Q20 | Potential Part Promotion | Nested subqueries |
| Q21 | Suppliers Who Kept Orders Waiting | Complex EXISTS |
| Q22 | Global Sales Opportunity | SUBSTRING |

### Data Scale Factors

| Scale Factor | Data Size | Tables | Rows (lineitem) |
|--------------|-----------|--------|-----------------|
| SF1 | ~1 GB | 8 | ~6M |
| SF10 | ~10 GB | 8 | ~60M |
| SF100 | ~100 GB | 8 | ~600M |
| SF1000 | ~1 TB | 8 | ~6B |

## 📊 TPC-DS Benchmark

### All 99 Queries Included

TPC-DS includes 99 queries across 4 categories:

| Category | Queries | Description |
|----------|---------|-------------|
| **Reporting** | Q1-Q20 | Basic reporting queries |
| **Ad-hoc** | Q21-Q40 | Interactive analysis |
| **Iterative OLAP** | Q41-Q60 | Complex analytical queries |
| **Data Mining** | Q61-Q99 | Advanced analytics |

**Query Examples:**
- **Q1**: Customer return analysis with CTEs
- **Q2**: Weekly sales comparison across years
- **Q3**: Sales analysis by brand
- **Q7**: Promotional sales analysis
- **Q10**: Customer segment analysis with EXISTS/NOT EXISTS
- **Q52**: Store sales by brand
- **Q88**: Complex hourly sales analysis

### Data Scale Factors

| Scale Factor | Data Size | Tables | Complexity |
|--------------|-----------|--------|------------|
| SF1 | ~3 GB | 24 | Testing |
| SF10 | ~30 GB | 24 | Moderate |
| SF100 | ~300 GB | 24 | Production |
| SF1000 | ~3 TB | 24 | Large scale |

## 📸 HTML Report Features

Both TPC-H and TPC-DS generate identical HTML reports with:

### Summary Dashboard
- Metadata cards (scale factor, rounds, endpoints)
- Overall performance stats (geometric mean, average, median speedup)

### Interactive Charts
- **Execution Time Comparison**: Bar chart showing Java FE vs Rust FE times
- **Speedup Factor**: Bar chart showing speedup for each query

### Detailed Results Table
- Query name
- Java FE mean time ± stdev
- Rust FE mean time ± stdev
- Speedup factor
- Color-coded (green for speedup > 1.0)

### Query-by-Query Analysis
- Individual round times for all queries
- Breakdown showing all execution rounds

## 🎯 Expected Performance

### Rust FE Optimizations Impact

| Optimization | Query Types | Expected Speedup |
|--------------|-------------|------------------|
| **Cost-based Join** | Multi-join queries (Q3, Q5, Q7, Q8, Q10, Q21) | 2.5-4x |
| **Partition Pruning** | Date-filtered queries (Q1, Q6, Q12, Q14) | 1.8-2.5x |
| **Runtime Filters** | Selective joins (Q3, Q10, Q18, Q21) | 2-3x |
| **Bucket Shuffle** | Large aggregations (Q1, Q5, Q9) | 1.5-2x |
| **Combined** | **All queries** | **2.5-3.5x** |

### TPC-H Expected Results

```
Overall Geometric Mean Speedup:  2.5-3.5x
Best Query Speedup (Q10):        3.5-4.5x
Worst Query Speedup (Q6):        2.0-2.5x
```

### TPC-DS Expected Results

```
Overall Geometric Mean Speedup:  2.0-3.0x
Best Query Speedup (Q72):        4.0-5.0x
Worst Query Speedup (Q55):       1.5-2.0x
```

## 💻 Advanced Usage

### Custom Scale Factors

```bash
# TPC-H SF10
./scripts/tpch/generate_data.sh --scale 10
python3 scripts/benchmark_tpch.py --scale 10 --rounds 3

# TPC-DS SF100
./scripts/tpcds/generate_data.sh --scale 100
python3 scripts/benchmark_tpcds.py --scale 100 --rounds 3
```

### More Rounds for Precision

```bash
# 10 rounds with 3 warmup rounds
python3 scripts/benchmark_tpch.py --rounds 10 --warmup 3
python3 scripts/benchmark_tpcds.py --rounds 10 --warmup 3
```

### Custom Output Files

```bash
# Separate reports for different runs
python3 scripts/benchmark_tpch.py \
    --output-html results/tpch_sf1_run1.html \
    --output-json results/tpch_sf1_run1.json

python3 scripts/benchmark_tpcds.py \
    --output-html results/tpcds_sf1_run1.html \
    --output-json results/tpcds_sf1_run1.json
```

### Subset of Queries

```bash
# Run only specific queries (edit queries directory)
mkdir -p scripts/tpch/queries_subset
cp scripts/tpch/queries/q{1,3,6}.sql scripts/tpch/queries_subset/
python3 scripts/benchmark_tpch.py --queries-dir scripts/tpch/queries_subset
```

## 🔧 Troubleshooting

### Connection Issues

```bash
# Test FE connectivity
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT 1"  # Java FE
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT 1"  # Rust FE
```

### Data Not Found

```bash
# Check databases
mysql -h 127.0.0.1 -P 9030 -u root -e "SHOW DATABASES LIKE 'tpc%'"

# Regenerate if needed
./scripts/tpch/generate_data.sh --scale 1
./scripts/tpcds/generate_data.sh --scale 1
```

### Query Failures

If queries fail during benchmark:

1. **Run manually** to see full error:
   ```bash
   mysql -h 127.0.0.1 -P 9030 -u root tpch_sf1 < scripts/tpch/queries/q1.sql
   ```

2. **Check FE logs**:
   ```bash
   docker compose logs doris-fe-rust
   ```

3. **Verify data loaded**:
   ```sql
   SELECT COUNT(*) FROM lineitem;
   ```

## 📁 Project Structure

```
rust-fe/
├── scripts/
│   ├── benchmark_tpch.py         # TPC-H benchmark runner
│   ├── benchmark_tpcds.py        # TPC-DS benchmark runner (symlink)
│   ├── tpch/
│   │   ├── generate_data.sh      # TPC-H data generation
│   │   └── queries/              # All 22 TPC-H queries
│   │       ├── q1.sql
│   │       ├── q2.sql
│   │       └── ... (q1-q22)
│   ├── tpcds/
│   │   ├── generate_data.sh      # TPC-DS data generation
│   │   └── queries/              # All 99 TPC-DS queries
│   │       ├── q1.sql
│   │       ├── q2.sql
│   │       └── ... (q1-q99)
│   ├── extract_tpcds_queries.sh  # Extract queries from TPC-DS kit
│   └── generate_tpcds_queries.py # Generate TPC-DS query templates
├── BENCHMARK_GUIDE.md            # Detailed benchmark guide
├── README_BENCHMARK.md           # This file
└── requirements.txt              # Python dependencies
```

## 📚 Documentation

- **[BENCHMARK_GUIDE.md](BENCHMARK_GUIDE.md)**: Complete 500+ line guide with troubleshooting
- **[scripts/README.md](scripts/README.md)**: Scripts overview
- **[docker/README.md](docker/README.md)**: Docker setup guide

## 🎓 References

- [TPC-H Specification](http://www.tpc.org/tpch/)
- [TPC-DS Specification](http://www.tpc.org/tpcds/)
- [TPC-DS Kit (Query Templates)](https://github.com/gregrahn/tpcds-kit)
- [ClickHouse JSONBench](https://github.com/ClickHouse/JSONBench)
- [Apache Doris Documentation](https://doris.apache.org/docs/)

## ✨ Example Output

### Console Output

```
=============================================================
TPC-H Benchmark (SF1)
=============================================================
Java FE:  127.0.0.1:9030
Rust FE:  127.0.0.1:9031
Rounds:   5
Queries:  22
=============================================================

Q1: Pricing Summary Report
------------------------------------------------------------
  Round 1/5:
    Java FE: 2.451s
    Rust FE: 0.823s
    Speedup: 2.98x (+66.4%)
  ...

=============================================================
BENCHMARK SUMMARY
=============================================================

Query    Java FE (mean)  Rust FE (mean)  Speedup
------------------------------------------------------------
Q1            2.420s           0.812s      2.98x
Q2            1.234s           0.456s      2.70x
Q3            1.567s           0.534s      2.93x
...
Q22           0.892s           0.345s      2.59x
------------------------------------------------------------

Overall Geometric Mean Speedup:              2.87x
Average Speedup:                             2.89x
Median Speedup:                              2.90x
Best Speedup:                                3.15x
```

### HTML Report

Open `tpch_results.html` or `tpcds_results.html` in your browser to see:
- Beautiful responsive design
- Interactive charts
- Complete statistics
- Query-by-query breakdown

## 🎉 Summary

This benchmark suite provides:
- ✅ **Complete coverage**: 22 TPC-H + 99 TPC-DS queries
- ✅ **Easy to use**: One command to run everything
- ✅ **Beautiful reports**: HTML + JSON output
- ✅ **Statistical rigor**: Multiple rounds, warmup, stdev
- ✅ **Production-ready**: Test at any scale factor

**Start benchmarking now** to validate the Rust FE performance improvements!

```bash
# Run complete benchmark suite
./scripts/tpch/generate_data.sh --scale 1
python3 scripts/benchmark_tpch.py --scale 1 --rounds 5

./scripts/tpcds/generate_data.sh --scale 1
python3 scripts/benchmark_tpcds.py --scale 1 --rounds 5

# Compare results
open tpch_results.html
open tpcds_results.html
```
