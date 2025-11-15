# Benchmark Scripts

Comprehensive TPC-H and TPC-DS benchmarking suite for comparing Java FE vs Rust FE performance.

## Quick Start

```bash
# 1. Install dependencies
pip3 install -r requirements.txt

# 2. Start Docker cluster
./docker/quickstart.sh

# 3. Generate TPC-H SF1 data
./scripts/tpch/generate_data.sh --scale 1

# 4. Run benchmark (3-5 rounds)
python3 scripts/benchmark_tpch.py --scale 1 --rounds 5

# 5. Open HTML report
open tpch_results.html
```

## Directory Structure

```
scripts/
├── README.md                     # This file
├── benchmark_tpch.py             # TPC-H benchmark runner
├── benchmark_tpcds.py            # TPC-DS benchmark runner (future)
├── tpch/
│   ├── generate_data.sh          # TPC-H data generation
│   └── queries/                  # TPC-H SQL queries
│       ├── q1.sql                # Query 1
│       ├── q3.sql                # Query 3
│       ├── q6.sql                # Query 6
│       ├── q10.sql               # Query 10
│       └── q18.sql               # Query 18
└── tpcds/
    ├── README.md                 # TPC-DS coming soon
    └── queries/                  # TPC-DS queries (future)
```

## Available Benchmarks

### ✅ TPC-H (Ready)
- 5 representative queries (Q1, Q3, Q6, Q10, Q18)
- Scale factors: SF1, SF10, SF100, SF1000
- Multi-round execution (configurable)
- HTML report with charts
- JSON results export

### 🚧 TPC-DS (Coming Soon)
- 99 queries planned
- Follow same pattern as TPC-H
- See `tpcds/README.md` for status

## Benchmark Features

✅ **Multi-round execution**: Run 3-5 rounds per query
✅ **Warmup support**: Prime caches before benchmarking
✅ **Statistical analysis**: Mean, median, stdev, min, max
✅ **Speedup calculation**: Automatic Java FE vs Rust FE comparison
✅ **Beautiful HTML reports**: Charts and tables
✅ **JSON export**: Machine-readable results
✅ **JSONBench-inspired**: Following ClickHouse best practices

## Example Output

```
=============================================================
TPC-H Benchmark (SF1)
=============================================================
Java FE:  127.0.0.1:9030
Rust FE:  127.0.0.1:9031
Rounds:   5
Queries:  5
=============================================================

Q1: Pricing Summary Report
------------------------------------------------------------
  Round 1/5:
    Java FE: 2.451s
    Rust FE: 0.823s
    Speedup: 2.98x (+66.4%)
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
```

## Documentation

- **[BENCHMARK_GUIDE.md](../BENCHMARK_GUIDE.md)**: Complete benchmark guide
- **[tpch/queries/](tpch/queries/)**: TPC-H query SQL files
- **[tpcds/README.md](tpcds/README.md)**: TPC-DS status and roadmap

## Usage Examples

### Basic Usage

```bash
# Run SF1 with 3 rounds
python3 scripts/benchmark_tpch.py --scale 1 --rounds 3
```

### Advanced Usage

```bash
# SF10 with 5 rounds and 2 warmup rounds
python3 scripts/benchmark_tpch.py \
    --scale 10 \
    --rounds 5 \
    --warmup 2 \
    --output-html tpch_sf10.html \
    --output-json tpch_sf10.json
```

### Custom FE Endpoints

```bash
# Use different ports
python3 scripts/benchmark_tpch.py \
    --java-host 192.168.1.10 \
    --java-port 9030 \
    --rust-host 192.168.1.11 \
    --rust-port 9031
```

## Requirements

- Python 3.7+
- mysql-connector-python
- Running Doris cluster (Java FE + Rust FE)
- TPC-H data loaded

See [BENCHMARK_GUIDE.md](../BENCHMARK_GUIDE.md) for detailed setup instructions.

## Contributing

To add more TPC-H queries:

1. Create `.sql` file in `scripts/tpch/queries/`
2. Re-run benchmark (auto-discovers queries)

To implement TPC-DS:

1. Follow pattern in `scripts/tpch/generate_data.sh`
2. Create table DDL and queries
3. Use `benchmark_tpch.py` as template

---

**For complete documentation, see [BENCHMARK_GUIDE.md](../BENCHMARK_GUIDE.md)**
