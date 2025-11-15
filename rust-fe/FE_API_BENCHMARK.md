# FE API E2E Latency & Resource Benchmark

Comprehensive benchmark for testing Frontend (FE) API performance with increasing concurrency loads.

## Overview

This benchmark measures FE performance across two key APIs:
- **MySQL Protocol Queries** - Standard SQL query execution
- **Stream Load API** - Bulk data loading via HTTP

## Metrics Measured

For each concurrency level, the benchmark tracks:

| Metric | Description |
|--------|-------------|
| **QPS/RPS** | Queries/Requests per second (throughput) |
| **Latency** | Average response time in milliseconds |
| **CPU Usage** | Average CPU utilization percentage (sampled every 100ms) |
| **Memory Usage** | Average memory utilization percentage (sampled every 100ms) |

## Visualization

**Two versions available:**

1. **Standard** (`benchmark_fe_api.sh`) - Tables only
2. **Enhanced** (`benchmark_fe_api_visual.sh`) - **RECOMMENDED** - Includes:
   - **Interactive SVG line charts** showing trends vs concurrency
   - **QPS/RPS charts** - Visualize throughput scaling
   - **Latency charts** - Spot performance degradation
   - **CPU charts** - Monitor resource consumption
   - **Memory charts** - Track memory pressure
   - **Theme toggle** - Light/dark mode support
   - **Zero dependencies** - Pure vanilla JavaScript + SVG

## Quick Start

### 1. Basic Usage

```bash
# Standard version (tables only)
./scripts/benchmark_fe_api.sh

# Enhanced version (with interactive charts) - RECOMMENDED
./scripts/benchmark_fe_api_visual.sh

# Test Java FE
./scripts/benchmark_fe_api_visual.sh --fe-host 127.0.0.1 --mysql-port 9030 --http-port 8030

# Test Rust FE
./scripts/benchmark_fe_api_visual.sh --fe-host 127.0.0.1 --mysql-port 9031 --http-port 8031
```

### 2. View Results

```bash
# Open HTML report (with interactive charts!)
open fe_api_results.html

# View JSON data
cat fe_api_results.json | jq
```

## Configuration Options

### Concurrency Levels

Test different concurrent load patterns:

```bash
# Light load (1-20 concurrent)
./scripts/benchmark_fe_api.sh --concurrency "1 5 10 20"

# Medium load (1-50 concurrent)
./scripts/benchmark_fe_api.sh --concurrency "1 10 25 50"

# Heavy load (1-200 concurrent)
./scripts/benchmark_fe_api.sh --concurrency "1 20 50 100 200"

# Custom levels
./scripts/benchmark_fe_api.sh --concurrency "5 15 30 60 120"
```

### Request Volume

Adjust requests per concurrency level:

```bash
# Quick test (100 requests per level)
./scripts/benchmark_fe_api.sh --requests 100

# Standard test (1000 requests - default)
./scripts/benchmark_fe_api.sh --requests 1000

# Thorough test (10000 requests)
./scripts/benchmark_fe_api.sh --requests 10000
```

### Warmup

Configure warmup requests before benchmarking:

```bash
# Minimal warmup
./scripts/benchmark_fe_api.sh --warmup 50

# Standard warmup (default)
./scripts/benchmark_fe_api.sh --warmup 100

# Extended warmup
./scripts/benchmark_fe_api.sh --warmup 500
```

## Use Cases

### 1. Compare Java FE vs Rust FE

```bash
# Benchmark Java FE
./scripts/benchmark_fe_api.sh \
    --fe-host 127.0.0.1 \
    --mysql-port 9030 \
    --http-port 8030 \
    --output-json java_fe_api.json \
    --output-html java_fe_api.html

# Benchmark Rust FE
./scripts/benchmark_fe_api.sh \
    --fe-host 127.0.0.1 \
    --mysql-port 9031 \
    --http-port 8031 \
    --output-json rust_fe_api.json \
    --output-html rust_fe_api.html

# Compare results
diff <(jq '.mysql_query' java_fe_api.json) <(jq '.mysql_query' rust_fe_api.json)
```

### 2. Identify Performance Bottlenecks

```bash
# Test with aggressive concurrency to find limits
./scripts/benchmark_fe_api.sh \
    --concurrency "1 10 50 100 200 500 1000" \
    --requests 5000

# Analyze where QPS plateaus or latency spikes
open fe_api_results.html
```

### 3. Resource Usage Analysis

```bash
# Monitor CPU/Memory under load
./scripts/benchmark_fe_api.sh \
    --concurrency "1 25 50 75 100" \
    --requests 2000

# Check results for resource trends
jq '.mysql_query | to_entries[] | {concurrency: .key, cpu: .value.avg_cpu_percent, mem: .value.avg_mem_percent}' fe_api_results.json
```

### 4. API-Specific Testing

Focus on specific API performance:

**MySQL Query Performance Only:**
```bash
# After running benchmark, analyze MySQL results
jq '.mysql_query' fe_api_results.json
```

**Stream Load Performance Only:**
```bash
# After running benchmark, analyze Stream Load results
jq '.stream_load' fe_api_results.json
```

## Example Output

### Console Output

```
[INFO] Starting FE API E2E Benchmark
[INFO] FE Endpoint: 127.0.0.1:9030 (MySQL) / :8030 (HTTP)
[INFO] Concurrency Levels: 1 5 10 20 50 100
[INFO] Requests per Level: 1000

[INFO] Testing FE connection...
[SUCCESS] FE connection OK

[INFO] Setting up test database and table...
[SUCCESS] Database setup complete

[INFO] Running warmup (100 requests)...
[SUCCESS] Warmup complete

[INFO] === Testing Concurrency Level: 1 ===
[INFO] MySQL Query Test: concurrency=1, requests=1000
[SUCCESS] MySQL: QPS=245.67, Latency=4.07ms, CPU=12.5%, MEM=2.3%
[INFO] Stream Load Test: concurrency=1, requests=1000
[SUCCESS] Stream: RPS=178.23, Latency=5.61ms, CPU=15.2%, MEM=2.4%

[INFO] === Testing Concurrency Level: 10 ===
[INFO] MySQL Query Test: concurrency=10, requests=1000
[SUCCESS] MySQL: QPS=1823.45, Latency=5.48ms, CPU=45.7%, MEM=3.1%
[INFO] Stream Load Test: concurrency=10, requests=1000
[SUCCESS] Stream: RPS=1234.56, Latency=8.10ms, CPU=52.3%, MEM=3.5%

...

[SUCCESS] Benchmark complete!
[INFO] Results saved to:
  - JSON: fe_api_results.json
  - HTML: fe_api_results.html
```

### JSON Results Structure

```json
{
  "benchmark": "FE API E2E",
  "timestamp": "2024-11-15T08:00:00+00:00",
  "fe_endpoint": "127.0.0.1:9030",
  "concurrency_levels": [1, 5, 10, 20, 50, 100],
  "requests_per_level": 1000,
  "mysql_query": {
    "1": {
      "qps": 245.67,
      "avg_latency_ms": 4.07,
      "avg_cpu_percent": 12.5,
      "avg_mem_percent": 2.3
    },
    "10": {
      "qps": 1823.45,
      "avg_latency_ms": 5.48,
      "avg_cpu_percent": 45.7,
      "avg_mem_percent": 3.1
    }
  },
  "stream_load": {
    "1": {
      "rps": 178.23,
      "avg_latency_ms": 5.61,
      "avg_cpu_percent": 15.2,
      "avg_mem_percent": 2.4
    }
  }
}
```

### HTML Report

The HTML report provides:
- **Theme Toggle** - Light/dark mode
- **Metadata Summary** - Endpoint, request count, timestamp
- **MySQL Query Table** - QPS, latency, CPU, memory by concurrency
- **Stream Load Table** - RPS, latency, CPU, memory by concurrency
- **Color-coded Metrics** - Easy visual identification

## Expected Results

### Typical Performance Patterns

**Low Concurrency (1-10):**
- Linear QPS/RPS increase
- Stable latency
- Low CPU/memory usage

**Medium Concurrency (10-50):**
- QPS/RPS scaling continues
- Latency starts to increase slightly
- CPU usage rises proportionally

**High Concurrency (50-100+):**
- QPS/RPS plateau or decrease
- Latency increases significantly
- CPU approaches saturation

### Java FE vs Rust FE

Expected differences:

| Metric | Java FE | Rust FE | Rust Advantage |
|--------|---------|---------|----------------|
| **Peak QPS** | ~2000-3000 | ~5000-8000 | 2-3x higher |
| **Latency (p50)** | 5-10ms | 2-5ms | 2x lower |
| **CPU Efficiency** | High under load | Lower overall | 30-50% better |
| **Memory Usage** | Higher (JVM) | Lower (native) | 40-60% less |

## Troubleshooting

### Connection Failed

```bash
# Verify FE is running
docker compose ps

# Test connection manually
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT 1"
curl http://127.0.0.1:8030/api/health
```

### Resource Monitoring Disabled

If you see "resource monitoring disabled":
```bash
# Check if FE process is running
ps aux | grep -E "DorisFeMain|rust-fe"

# The benchmark will still work, just without CPU/memory metrics
```

### High Latency at Low Concurrency

Could indicate:
- Network issues
- FE not warmed up (increase warmup requests)
- Slow backend connections

## Advanced Usage

### Custom Test Scenarios

```bash
# Stress test with extreme concurrency
./scripts/benchmark_fe_api.sh \
    --concurrency "100 200 500 1000" \
    --requests 10000 \
    --warmup 1000

# Micro-benchmark with precise measurements
./scripts/benchmark_fe_api.sh \
    --concurrency "1 2 4 8 16" \
    --requests 10000 \
    --warmup 500

# Production simulation (moderate sustained load)
./scripts/benchmark_fe_api.sh \
    --concurrency "20 40 60" \
    --requests 50000 \
    --warmup 2000
```

### Automated Comparison

```bash
#!/bin/bash
# Compare Java vs Rust FE automatically

# Benchmark Java FE
./scripts/benchmark_fe_api.sh \
    --mysql-port 9030 --http-port 8030 \
    --output-json java.json --output-html java.html

# Benchmark Rust FE
./scripts/benchmark_fe_api.sh \
    --mysql-port 9031 --http-port 8031 \
    --output-json rust.json --output-html rust.html

# Generate comparison report
echo "=== QPS Comparison (MySQL) ==="
paste \
    <(jq -r '.mysql_query | to_entries[] | "\(.key): \(.value.qps)"' java.json) \
    <(jq -r '.mysql_query | to_entries[] | "\(.value.qps)"' rust.json) \
    | awk '{printf "%s Java=%s Rust=%s Speedup=%.2fx\n", $1, $2, $3, $3/$2}'
```

## Technical Details

### How It Works

1. **Setup Phase**
   - Creates test database and table
   - Identifies FE process PID for monitoring

2. **Warmup Phase**
   - Executes warmup requests
   - Ensures JIT compilation (Java) or cache warming

3. **Benchmark Phase**
   - For each concurrency level:
     - Spawns N concurrent workers
     - Each worker executes requests
     - Samples CPU/memory every 100ms
     - Calculates throughput and latency

4. **Reporting Phase**
   - Aggregates results across all levels
   - Generates JSON and HTML reports

### Limitations

- **CPU/Memory Accuracy**: Sampled every 100ms, may miss spikes
- **Network Overhead**: Not isolated from network latency
- **Shared Resources**: FE performance affected by other processes
- **Small Data**: Uses small test datasets (not representative of large workloads)

### Dependencies

Required tools:
- `mysql` - MySQL client for queries
- `curl` - HTTP client for Stream Load
- `bc` - Calculator for statistics
- `jq` - (Optional) JSON formatting

## Integration with Other Benchmarks

Combine with TPC-H/TPC-DS for comprehensive testing:

```bash
# 1. Test FE API performance (this benchmark)
./scripts/benchmark_fe_api.sh

# 2. Test query optimization (TPC-H)
./scripts/benchmark_tpch.sh --scale 1 --rounds 3

# 3. Test complex queries (TPC-DS)
./scripts/benchmark_tpcds.sh --scale 1 --rounds 3
```

This provides:
- **API Benchmark**: Raw FE throughput and resource usage
- **TPC-H**: Query optimizer performance
- **TPC-DS**: Complex query handling

## Summary

**FE API E2E Benchmark:**
- ✅ Tests MySQL protocol and Stream Load APIs
- ✅ Measures QPS/RPS, latency, CPU, memory (100ms sampling)
- ✅ Supports configurable concurrency levels (1-100+)
- ✅ Pure bash (no dependencies beyond mysql/curl/bc)
- ✅ **Interactive SVG charts** visualizing performance trends
- ✅ Generates beautiful HTML reports with theme toggle
- ✅ Perfect for Java FE vs Rust FE comparison

## How Metrics Are Measured

**CPU & Memory Sampling:**
```bash
# Finds FE process
FE_PID=$(ps aux | grep -E "DorisFeMain|rust-fe" | grep -v grep | awk '{print $2}')

# Samples every 100ms during benchmark
while benchmark_running; do
    ps -p "$FE_PID" -o %cpu,%mem --no-headers
    sleep 0.1
done

# Averages all samples
avg_cpu = sum(cpu_samples) / sample_count
```

**Throughput (QPS/RPS):**
```bash
qps = total_requests / total_duration_seconds
```

**Latency:**
```bash
avg_latency_ms = (total_duration_seconds * 1000) / total_requests
```

**Visualization:**
- Pure JavaScript SVG rendering (no Chart.js/libraries)
- Responsive line charts showing trends
- Automatic scaling based on data ranges
- Theme-aware colors (light/dark mode)

**Use it to validate Rust FE's superior throughput, lower latency, and better resource efficiency!** 🚀
