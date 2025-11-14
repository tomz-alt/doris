# Benchmarking Guide: Validating Distributed Query Optimizations

This guide explains how to validate that optimizations are working and benchmark Rust FE vs Java FE performance.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Micro-Benchmarks (FE Only)](#micro-benchmarks-fe-only)
3. [End-to-End Benchmarks](#end-to-end-benchmarks)
4. [Validating Optimizations](#validating-optimizations)
5. [Comparing Rust FE vs Java FE](#comparing-rust-fe-vs-java-fe)
6. [Analyzing Results](#analyzing-results)

---

## Quick Start

### Prerequisites

```bash
# Install Rust benchmarking tools
cargo install criterion

# Install Python dependencies for analysis
pip3 install pandas matplotlib seaborn

# Ensure Doris cluster is running
# - At least 1 FE (either Rust or Java)
# - At least 3 BEs
# - TPC-H dataset loaded (1GB scale factor recommended)
```

### Run All Benchmarks

```bash
# 1. Run micro-benchmarks (FE layer only)
cargo bench --bench tpch_benchmark

# 2. Run end-to-end comparison
cd scripts
chmod +x benchmark_fe_comparison.sh
./benchmark_fe_comparison.sh

# 3. View results
open ../target/criterion/report/index.html  # Micro-benchmark report
cat ../benchmark_results/BENCHMARK_SUMMARY.md  # E2E comparison
```

---

## Micro-Benchmarks (FE Only)

Measures FE layer performance in isolation (no BE interaction).

### What is Measured

- **SQL Parsing**: Time to parse TPC-H queries with Doris parser
- **Logical Planning**: Time to create logical plans with DataFusion
- **Fragment Splitting**: Time to split plans into distributed fragments
- **Cost Model**: Time to make cost-based join decisions
- **Partition Pruning**: Time to prune partitions based on predicates
- **Runtime Filters**: Time to generate runtime filters

### Running Micro-Benchmarks

```bash
# Run all micro-benchmarks
cargo bench --bench tpch_benchmark

# Run specific benchmark group
cargo bench --bench tpch_benchmark -- sql_parsing
cargo bench --bench tpch_benchmark -- fragment_splitting
cargo bench --bench tpch_benchmark -- cost_model

# Save baseline for comparison
cargo bench --bench tpch_benchmark -- --save-baseline before_optimization

# Compare with baseline
cargo bench --bench tpch_benchmark -- --baseline before_optimization
```

### Example Output

```
sql_parsing/Q1         time:   [245.32 µs 247.18 µs 249.21 µs]
sql_parsing/Q3         time:   [412.55 µs 415.89 µs 419.44 µs]

fragment_splitting/Q1/baseline
                       time:   [1.2341 ms 1.2398 ms 1.2459 ms]

fragment_splitting/Q1/cost_based_join
                       time:   [1.3215 ms 1.3287 ms 1.3365 ms]
                       change: [+7.15% +7.45% +7.76%] (p = 0.00 < 0.05)

cost_model/broadcast_vs_shuffle_decision
                       time:   [1.8923 µs 1.9034 µs 1.9152 µs]

partition_pruning/prune_12_partitions
                       time:   [3.4521 µs 3.4698 µs 3.4891 µs]
```

### Interpreting Results

- **Time**: Mean execution time with confidence interval
- **Change**: Performance change vs baseline (+ is slower, - is faster)
- **Throughput**: Operations per second

**Key Insights**:
- Parsing should be < 500µs for simple queries
- Fragment splitting adds ~1-2ms overhead
- Cost model decisions should be < 5µs
- Partition pruning should be < 10µs per partition

---

## End-to-End Benchmarks

Measures complete query execution including BE interaction.

### Setup

```bash
# 1. Load TPC-H data
mysql -h localhost -P 9030 -u root < scripts/load_tpch.sql

# 2. Configure benchmark
export TPCH_SCALE=1           # 1GB scale factor
export NUM_ITERATIONS=10      # Run each query 10 times
export RUST_FE_HOST=localhost
export RUST_FE_PORT=9030
export JAVA_FE_HOST=localhost
export JAVA_FE_PORT=9031      # If running Java FE separately

# 3. Run benchmark
cd scripts
./benchmark_fe_comparison.sh
```

### What is Measured

For each query and configuration:
- **Total Time**: End-to-end query latency
- **Planning Time**: FE planning duration
- **Fragment Splitting Time**: Time to split into distributed fragments
- **BE Execution Time**: Time spent in BEs
- **Optimization Metrics**: Which optimizations were applied

### Benchmark Configurations

The script tests multiple feature flag configurations:

1. **baseline**: All optimizations disabled
2. **cost_based_join**: Only cost-based join selection
3. **partition_pruning**: Only partition pruning
4. **runtime_filters**: Only runtime filters
5. **all_optimizations**: All optimizations enabled

### Example Output

```
========================================
  Doris FE Benchmark: Rust vs Java
========================================

Configuration:
  - TPC-H Scale: 1GB
  - Iterations: 10
  - Rust FE: localhost:9030
  - Java FE: localhost:9031
  - Output: ./benchmark_results

Testing configuration: baseline
  Running Q1...
    Iteration 1: Rust=245ms, Java=312ms
    Iteration 2: Rust=238ms, Java=305ms
    ...

Testing configuration: all_optimizations
  Running Q1...
    Iteration 1: Rust=142ms, Java=298ms
    Iteration 2: Rust=138ms, Java=301ms
    ...

Benchmark complete! Results saved to: ./benchmark_results
```

---

## Validating Optimizations

### 1. Check BE Logs

Optimizations leave traces in BE logs:

```bash
# View BE logs
tail -f /path/to/be/log/be.INFO

# Look for optimization indicators:
grep "Runtime filter" be.INFO          # Runtime filters applied
grep "Partition pruned" be.INFO        # Partition pruning
grep "Bucket shuffle" be.INFO          # Bucket shuffle optimization
grep "Arrow IPC" be.INFO               # Arrow format transfer
```

### 2. Query Profiling

Use `SHOW QUERY PROFILE` to see optimization details:

```sql
-- Run query with optimizations
SELECT /*+ SET_VAR(enable_cost_based_join=true) */
    l_returnflag,
    SUM(l_quantity) as sum_qty
FROM lineitem
WHERE l_shipdate >= '1998-01-01'
GROUP BY l_returnflag;

-- View profile
SHOW QUERY PROFILE;
```

Look for:
- **Join Strategy**: "Broadcast" or "Shuffle"
- **Partitions Scanned**: Number of partitions (should be reduced with pruning)
- **Runtime Filters**: Filter generation and effectiveness
- **Exchange Type**: HashPartition, Broadcast, or Gather

### 3. Use Query Profiler API

The Rust FE includes a query profiler for detailed metrics:

```rust
use doris_rust_fe::benchmark::QueryProfiler;
use uuid::Uuid;

let mut profiler = QueryProfiler::new();

// Start profiling
let query_id = Uuid::new_v4();
let handle = profiler.start_query(
    query_id,
    "SELECT ...".to_string(),
    QueryFeatureFlags::all_enabled(),
);

// Record optimizations
profiler.record_join_strategy(
    query_id,
    join_node_id,
    "Broadcast",
    "lineitem".to_string(),
    "orders".to_string(),
    1_000_000,
    100_000,
    "Right side < 10MB threshold".to_string(),
);

profiler.record_partition_pruning(
    query_id,
    "lineitem".to_string(),
    12,  // total partitions
    6,   // pruned
    vec!["p7".to_string(), "p8".to_string(), ...],
);

// Complete profiling
profiler.complete_query(handle, 3, 5, 1000);

// Get profile
let profile = profiler.get_profile(&query_id).unwrap();
println!("Total time: {}ms", profile.execution_metrics.total_time_ms);
println!("Partitions pruned: {}%",
    profile.optimization_info.partition_pruning
        .as_ref().unwrap().pruning_ratio * 100.0);
```

### 4. Compare Profiles

```rust
// Compare baseline vs optimized
let comparison = profiler.compare_profiles(&baseline_id, &optimized_id).unwrap();

println!("Speedup: {:.2}x", comparison.speedup);
println!("I/O reduction: {:.1}%", comparison.io_reduction * 100.0);

// Export markdown report
let report = comparison.to_markdown();
std::fs::write("comparison_report.md", report).unwrap();
```

---

## Comparing Rust FE vs Java FE

### Architecture Differences

| Component | Rust FE | Java FE |
|-----------|---------|---------|
| **Parser** | Doris dialect (custom) | ANTLR-based |
| **Planner** | DataFusion | Custom planner |
| **Optimizer** | Cost-based with feature flags | Rule-based + CBO |
| **BE Protocol** | gRPC (tonic) | Thrift |
| **Concurrency** | Tokio async | Java threads |

### Expected Performance Characteristics

**Rust FE Advantages**:
- Lower memory usage (no GC)
- Faster startup time
- Better tail latency (no GC pauses)
- More efficient concurrency (async/await)

**Java FE Advantages**:
- More mature optimizer
- Better statistics integration
- More comprehensive rule set
- Proven production stability

### Running Direct Comparison

```bash
# Terminal 1: Start Rust FE
cd rust-fe
cargo run --release

# Terminal 2: Start Java FE
cd doris/fe
./bin/start_fe.sh --daemon

# Terminal 3: Run comparison
cd rust-fe/scripts
./benchmark_fe_comparison.sh
```

### Metrics to Compare

1. **Latency**:
   - P50, P95, P99 query latency
   - Planning time
   - Execution time

2. **Throughput**:
   - Queries per second (QPS)
   - Concurrent query handling

3. **Resource Usage**:
   - Memory footprint
   - CPU utilization
   - Network bandwidth

4. **Optimization Effectiveness**:
   - Join strategy correctness
   - Partition pruning ratio
   - Runtime filter hit rate

---

## Analyzing Results

### 1. View Criterion HTML Reports

```bash
# Open benchmark report in browser
open target/criterion/report/index.html

# Compare specific benchmarks
open target/criterion/fragment_splitting/report/index.html
```

The HTML reports show:
- Performance over time
- Throughput estimates
- Regression detection
- Distribution plots

### 2. Analyze CSV Results

The end-to-end benchmark produces CSV files:

```bash
cd benchmark_results

# View results
cat baseline_results.csv
cat all_optimizations_results.csv

# Calculate statistics with Python
python3 <<EOF
import pandas as pd

baseline = pd.read_csv('baseline_results.csv')
optimized = pd.read_csv('all_optimizations_results.csv')

# Group by query and calculate mean
baseline_avg = baseline.groupby('Query')['Total_Time_ms'].mean()
optimized_avg = optimized.groupby('Query')['Total_Time_ms'].mean()

# Calculate speedup
speedup = baseline_avg / optimized_avg

print("Query Speedup:")
print(speedup.sort_values(ascending=False))

# Overall statistics
print(f"\nAverage speedup: {speedup.mean():.2f}x")
print(f"Best query: {speedup.idxmax()} ({speedup.max():.2f}x)")
print(f"Worst query: {speedup.idxmin()} ({speedup.min():.2f}x)")
EOF
```

### 3. Generate Visualizations

```bash
# Create performance comparison charts
python3 scripts/visualize_results.py \
    --baseline benchmark_results/baseline_results.csv \
    --optimized benchmark_results/all_optimizations_results.csv \
    --output benchmark_results/charts/
```

This generates:
- Bar chart: Query latency comparison
- Line chart: Performance over iterations
- Heatmap: Optimization effectiveness per query
- Box plot: Latency distribution

### 4. Statistical Significance

```python
from scipy import stats

# T-test for statistical significance
rust_times = optimized['Total_Time_ms']
java_times = baseline['Total_Time_ms']

t_stat, p_value = stats.ttest_ind(rust_times, java_times)

if p_value < 0.05:
    print(f"Performance difference is statistically significant (p={p_value:.4f})")
else:
    print(f"Performance difference is NOT statistically significant (p={p_value:.4f})")
```

---

## Optimization Validation Checklist

### Cost-Based Join Selection

- [ ] Verify join strategy in query profile (`SHOW QUERY PROFILE`)
- [ ] Check BE logs for "Broadcast" or "Shuffle" exchange
- [ ] Confirm small tables are broadcast (<10MB)
- [ ] Confirm large tables use shuffle join
- [ ] Measure network traffic reduction

### Partition Pruning

- [ ] Check BE logs for "Partitions pruned"
- [ ] Verify only relevant partitions are scanned
- [ ] Measure I/O reduction (bytes scanned)
- [ ] Test with various date ranges
- [ ] Validate pruning ratio > 50% for selective queries

### Runtime Filters

- [ ] Check BE logs for "Runtime filter generated"
- [ ] Verify filter is applied to probe side
- [ ] Measure probe side I/O reduction
- [ ] Test with dimension-fact joins
- [ ] Validate filter effectiveness > 70%

### Bucket Shuffle

- [ ] Verify colocated tables detected
- [ ] Check for "Bucket shuffle: colocated" in logs
- [ ] Measure network traffic (should be ~0 for colocated joins)
- [ ] Test with bucketed tables only
- [ ] Validate no shuffle exchange in plan

### Arrow Result Parsing

- [ ] Check BE response format in logs
- [ ] Measure result transfer time
- [ ] Compare with baseline (should be 5-10x faster)
- [ ] Test with large result sets (>100K rows)
- [ ] Verify zero-copy deserialization

---

## Troubleshooting

### Benchmarks Running Slowly

- Reduce `NUM_ITERATIONS` in benchmark script
- Use smaller TPC-H scale factor (`TPCH_SCALE=0.1`)
- Run specific queries only (edit `TPCH_QUERIES` array)

### Optimizations Not Applied

- Check feature flags are enabled: `echo $DORIS_COST_BASED_JOIN`
- Verify FE logs show optimization decisions
- Ensure tables have statistics for cost model
- Check partition metadata is available

### Inconsistent Results

- Increase `NUM_ITERATIONS` for more stable averages
- Warm up cluster before benchmark (run queries 3x first)
- Isolate benchmark environment (no other workload)
- Use dedicated hardware (not shared VM)

### BE Logs Missing Information

- Increase BE log level: `SET log_level = 'INFO';`
- Enable query profiling: `SET enable_profile = true;`
- Check BE log file location: `/path/to/be/log/be.INFO`

---

## Best Practices

1. **Baseline First**: Always run baseline configuration before optimizations
2. **Isolation**: Run benchmarks on isolated cluster (no production load)
3. **Warm-up**: Execute queries 3x before measuring
4. **Multiple Iterations**: Use at least 10 iterations for stable results
5. **Documentation**: Record cluster configuration, data size, versions
6. **Version Control**: Tag commit hash used for benchmark
7. **Automation**: Use scripts for reproducibility
8. **Analysis**: Use statistical methods to validate significance

---

## References

- Criterion.rs documentation: https://bheisler.github.io/criterion.rs/book/
- TPC-H specification: http://www.tpc.org/tpch/
- Query profiling guide: `docs/DISTRIBUTED_QUERY_OPTIMIZATIONS.md`
- Feature flags reference: `src/planner/feature_flags.rs`
