#!/bin/bash
set -e

# FE API E2E Latency & Resource Benchmark
# Tests FE performance with increasing concurrency for:
# - MySQL protocol queries
# - Stream Load (HTTP API)
# Measures: latency, CPU, memory usage

BENCHMARK_NAME="FE API E2E"
FE_HOST=${FE_HOST:-127.0.0.1}
FE_MYSQL_PORT=${FE_MYSQL_PORT:-9030}
FE_HTTP_PORT=${FE_HTTP_PORT:-8030}

# Concurrency levels to test
CONCURRENCY_LEVELS=${CONCURRENCY_LEVELS:-"1 5 10 20 50 100"}
REQUESTS_PER_LEVEL=${REQUESTS_PER_LEVEL:-1000}
WARMUP_REQUESTS=${WARMUP_REQUESTS:-100}

# Test configuration
DATABASE=${DATABASE:-benchmark_db}
TABLE=${TABLE:-test_table}
OUTPUT_JSON="fe_api_results.json"
OUTPUT_HTML="fe_api_results.html"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --fe-host) FE_HOST="$2"; shift 2 ;;
        --mysql-port) FE_MYSQL_PORT="$2"; shift 2 ;;
        --http-port) FE_HTTP_PORT="$2"; shift 2 ;;
        --concurrency) CONCURRENCY_LEVELS="$2"; shift 2 ;;
        --requests) REQUESTS_PER_LEVEL="$2"; shift 2 ;;
        --warmup) WARMUP_REQUESTS="$2"; shift 2 ;;
        --output-json) OUTPUT_JSON="$2"; shift 2 ;;
        --output-html) OUTPUT_HTML="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  --fe-host HOST             FE host (default: 127.0.0.1)"
            echo "  --mysql-port PORT          MySQL port (default: 9030)"
            echo "  --http-port PORT           HTTP port (default: 8030)"
            echo "  --concurrency LEVELS       Concurrency levels (default: '1 5 10 20 50 100')"
            echo "  --requests NUM             Requests per level (default: 1000)"
            echo "  --warmup NUM               Warmup requests (default: 100)"
            echo "  --output-json FILE         Output JSON file"
            echo "  --output-html FILE         Output HTML file"
            echo "  -h, --help                 Show this help"
            exit 0
            ;;
        *) error "Unknown option: $1" ;;
    esac
done

# Check dependencies
command -v mysql >/dev/null 2>&1 || error "mysql client not found"
command -v curl >/dev/null 2>&1 || error "curl not found"
command -v bc >/dev/null 2>&1 || error "bc not found"
command -v jq >/dev/null 2>&1 || warn "jq not found (optional for better JSON formatting)"

log "Starting $BENCHMARK_NAME Benchmark"
log "FE Endpoint: ${FE_HOST}:${FE_MYSQL_PORT} (MySQL) / :${FE_HTTP_PORT} (HTTP)"
log "Concurrency Levels: ${CONCURRENCY_LEVELS}"
log "Requests per Level: ${REQUESTS_PER_LEVEL}"
echo

# Test connection
log "Testing FE connection..."
mysql -h "$FE_HOST" -P "$FE_MYSQL_PORT" -u root -e "SELECT 1" 2>/dev/null || error "Cannot connect to FE"
success "FE connection OK"
echo

# Setup test database and table
log "Setting up test database and table..."
mysql -h "$FE_HOST" -P "$FE_MYSQL_PORT" -u root <<EOF 2>/dev/null || true
CREATE DATABASE IF NOT EXISTS ${DATABASE};
USE ${DATABASE};
DROP TABLE IF EXISTS ${TABLE};
CREATE TABLE ${TABLE} (
    id INT,
    name VARCHAR(100),
    value DOUBLE,
    timestamp DATETIME
) DISTRIBUTED BY HASH(id) BUCKETS 10;
EOF
success "Database setup complete"
echo

# Get FE process PID for resource monitoring
FE_PID=$(ps aux | grep -E "DorisFeMain|rust-fe" | grep -v grep | head -1 | awk '{print $2}')
if [ -z "$FE_PID" ]; then
    warn "Cannot find FE process PID, resource monitoring disabled"
    MONITOR_RESOURCES=false
else
    log "Found FE process PID: $FE_PID"
    MONITOR_RESOURCES=true
fi

# Function to get CPU and memory usage
get_resource_usage() {
    local pid=$1
    if [ "$MONITOR_RESOURCES" = true ]; then
        # Get CPU% and MEM% from ps
        ps -p "$pid" -o %cpu,%mem --no-headers 2>/dev/null || echo "0.0 0.0"
    else
        echo "0.0 0.0"
    fi
}

# Function to run MySQL query benchmark with concurrency
benchmark_mysql_query() {
    local concurrency=$1
    local requests=$2
    local query="SELECT id, name, value FROM ${DATABASE}.${TABLE} WHERE id < 1000 LIMIT 10"

    log "MySQL Query Test: concurrency=$concurrency, requests=$requests"

    # Create query file
    local query_file="/tmp/bench_query_$$.sql"
    echo "$query" > "$query_file"

    # Run concurrent requests and measure time
    local total_start=$(date +%s%N)
    local pids=()

    # Calculate requests per worker
    local per_worker=$((requests / concurrency))

    # Start concurrent workers
    for ((i=1; i<=concurrency; i++)); do
        (
            for ((j=1; j<=per_worker; j++)); do
                mysql -h "$FE_HOST" -P "$FE_MYSQL_PORT" -u root -D "$DATABASE" < "$query_file" >/dev/null 2>&1
            done
        ) &
        pids+=($!)
    done

    # Sample resource usage during test
    local cpu_samples=()
    local mem_samples=()
    local sample_count=0

    while [ $(jobs -r | wc -l) -gt 0 ]; do
        read cpu mem <<< $(get_resource_usage "$FE_PID")
        cpu_samples+=("$cpu")
        mem_samples+=("$mem")
        sample_count=$((sample_count + 1))
        sleep 0.1
    done

    # Wait for all workers to finish
    for pid in "${pids[@]}"; do
        wait "$pid" 2>/dev/null || true
    done

    local total_end=$(date +%s%N)
    local total_duration_ns=$((total_end - total_start))
    local total_duration_sec=$(echo "scale=3; $total_duration_ns / 1000000000" | bc)

    # Calculate average resource usage
    local avg_cpu=0
    local avg_mem=0
    if [ $sample_count -gt 0 ]; then
        for cpu in "${cpu_samples[@]}"; do
            avg_cpu=$(echo "$avg_cpu + $cpu" | bc)
        done
        avg_cpu=$(echo "scale=2; $avg_cpu / $sample_count" | bc)

        for mem in "${mem_samples[@]}"; do
            avg_mem=$(echo "$avg_mem + $mem" | bc)
        done
        avg_mem=$(echo "scale=2; $avg_mem / $sample_count" | bc)
    fi

    # Calculate metrics
    local qps=$(echo "scale=2; $requests / $total_duration_sec" | bc)
    local avg_latency_ms=$(echo "scale=2; $total_duration_sec * 1000 / $requests" | bc)

    rm -f "$query_file"

    echo "$qps|$avg_latency_ms|$avg_cpu|$avg_mem"
}

# Function to run Stream Load benchmark with concurrency
benchmark_stream_load() {
    local concurrency=$1
    local requests=$2

    log "Stream Load Test: concurrency=$concurrency, requests=$requests"

    # Create test data file (small CSV)
    local data_file="/tmp/bench_data_$$.csv"
    cat > "$data_file" <<EOF
1,test1,100.5,2024-01-01 00:00:00
2,test2,200.5,2024-01-01 00:00:01
3,test3,300.5,2024-01-01 00:00:02
EOF

    local total_start=$(date +%s%N)
    local pids=()

    # Calculate requests per worker
    local per_worker=$((requests / concurrency))

    # Start concurrent workers
    for ((i=1; i<=concurrency; i++)); do
        (
            for ((j=1; j<=per_worker; j++)); do
                curl -s --location-trusted \
                    -u root: \
                    -H "label:bench_label_${i}_${j}" \
                    -H "column_separator:," \
                    -T "$data_file" \
                    "http://${FE_HOST}:${FE_HTTP_PORT}/api/${DATABASE}/${TABLE}/_stream_load" \
                    >/dev/null 2>&1 || true
            done
        ) &
        pids+=($!)
    done

    # Sample resource usage during test
    local cpu_samples=()
    local mem_samples=()
    local sample_count=0

    while [ $(jobs -r | wc -l) -gt 0 ]; do
        read cpu mem <<< $(get_resource_usage "$FE_PID")
        cpu_samples+=("$cpu")
        mem_samples+=("$mem")
        sample_count=$((sample_count + 1))
        sleep 0.1
    done

    # Wait for all workers to finish
    for pid in "${pids[@]}"; do
        wait "$pid" 2>/dev/null || true
    done

    local total_end=$(date +%s%N)
    local total_duration_ns=$((total_end - total_start))
    local total_duration_sec=$(echo "scale=3; $total_duration_ns / 1000000000" | bc)

    # Calculate average resource usage
    local avg_cpu=0
    local avg_mem=0
    if [ $sample_count -gt 0 ]; then
        for cpu in "${cpu_samples[@]}"; do
            avg_cpu=$(echo "$avg_cpu + $cpu" | bc)
        done
        avg_cpu=$(echo "scale=2; $avg_cpu / $sample_count" | bc)

        for mem in "${mem_samples[@]}"; do
            avg_mem=$(echo "$avg_mem + $mem" | bc)
        done
        avg_mem=$(echo "scale=2; $avg_mem / $sample_count" | bc)
    fi

    # Calculate metrics
    local rps=$(echo "scale=2; $requests / $total_duration_sec" | bc)
    local avg_latency_ms=$(echo "scale=2; $total_duration_sec * 1000 / $requests" | bc)

    rm -f "$data_file"

    echo "$rps|$avg_latency_ms|$avg_cpu|$avg_mem"
}

# Run warmup
log "Running warmup ($WARMUP_REQUESTS requests)..."
benchmark_mysql_query 1 "$WARMUP_REQUESTS" >/dev/null
success "Warmup complete"
echo

# Initialize result arrays
declare -A mysql_results
declare -A stream_results

# Run benchmarks for each concurrency level
for concurrency in $CONCURRENCY_LEVELS; do
    log "=== Testing Concurrency Level: $concurrency ==="

    # MySQL Query Test
    result=$(benchmark_mysql_query "$concurrency" "$REQUESTS_PER_LEVEL")
    IFS='|' read -r qps latency cpu mem <<< "$result"
    mysql_results["$concurrency"]="$qps|$latency|$cpu|$mem"
    success "MySQL: QPS=$qps, Latency=${latency}ms, CPU=${cpu}%, MEM=${mem}%"

    # Stream Load Test
    result=$(benchmark_stream_load "$concurrency" "$REQUESTS_PER_LEVEL")
    IFS='|' read -r rps latency cpu mem <<< "$result"
    stream_results["$concurrency"]="$rps|$latency|$cpu|$mem"
    success "Stream: RPS=$rps, Latency=${latency}ms, CPU=${cpu}%, MEM=${mem}%"

    echo
    sleep 2  # Cool down between levels
done

# Generate JSON results
log "Generating results..."

cat > "$OUTPUT_JSON" <<EOF
{
  "benchmark": "$BENCHMARK_NAME",
  "timestamp": "$(date -Iseconds)",
  "fe_endpoint": "${FE_HOST}:${FE_MYSQL_PORT}",
  "concurrency_levels": [$(echo $CONCURRENCY_LEVELS | sed 's/ /, /g')],
  "requests_per_level": $REQUESTS_PER_LEVEL,
  "mysql_query": {
EOF

first=true
for concurrency in $CONCURRENCY_LEVELS; do
    IFS='|' read -r qps latency cpu mem <<< "${mysql_results[$concurrency]}"
    [ "$first" = false ] && echo "," >> "$OUTPUT_JSON"
    first=false
    cat >> "$OUTPUT_JSON" <<EOF
    "$concurrency": {
      "qps": $qps,
      "avg_latency_ms": $latency,
      "avg_cpu_percent": $cpu,
      "avg_mem_percent": $mem
    }
EOF
done

cat >> "$OUTPUT_JSON" <<EOF

  },
  "stream_load": {
EOF

first=true
for concurrency in $CONCURRENCY_LEVELS; do
    IFS='|' read -r rps latency cpu mem <<< "${stream_results[$concurrency]}"
    [ "$first" = false ] && echo "," >> "$OUTPUT_JSON"
    first=false
    cat >> "$OUTPUT_JSON" <<EOF
    "$concurrency": {
      "rps": $rps,
      "avg_latency_ms": $latency,
      "avg_cpu_percent": $cpu,
      "avg_mem_percent": $mem
    }
EOF
done

cat >> "$OUTPUT_JSON" <<EOF

  }
}
EOF

# Generate HTML report
cat > "$OUTPUT_HTML" <<'HTMLEOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FE API E2E Benchmark Results</title>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&display=swap" rel="stylesheet">
    <style>
        :root {
            --color: #000;
            --background-color: #fff;
            --table-header: #f5f5f5;
            --table-border: #e0e0e0;
            --metric-qps: #4ecdc4;
            --metric-latency: #ff6b6b;
            --metric-cpu: #ffa500;
            --metric-mem: #9b59b6;
        }

        [data-theme="dark"] {
            --color: #ccc;
            --background-color: #04293A;
            --table-header: #053b52;
            --table-border: #064663;
        }

        * { box-sizing: border-box; }

        body {
            font-family: 'Inter', sans-serif;
            background: var(--background-color);
            color: var(--color);
            margin: 0;
            padding: 20px;
            transition: background-color 0.3s, color 0.3s;
        }

        .container { max-width: 1400px; margin: 0 auto; }

        h1 { font-size: 2rem; margin-bottom: 0.5rem; font-weight: 700; }
        h2 { font-size: 1.5rem; margin-top: 2rem; }

        .theme-toggle {
            position: fixed;
            top: 20px;
            right: 20px;
            background: var(--table-header);
            border: 1px solid var(--table-border);
            padding: 8px 16px;
            border-radius: 4px;
            cursor: pointer;
            font-size: 1.2rem;
        }

        .metadata {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin: 20px 0;
        }

        .metadata-card {
            background: var(--table-header);
            padding: 15px;
            border-radius: 4px;
            border: 1px solid var(--table-border);
        }

        .metadata-card h3 {
            margin: 0 0 5px 0;
            font-size: 0.85rem;
            text-transform: uppercase;
            opacity: 0.7;
        }

        .metadata-card p {
            margin: 0;
            font-size: 1.2rem;
            font-weight: 600;
        }

        table {
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
            font-variant-numeric: tabular-nums;
        }

        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid var(--table-border);
        }

        th {
            background: var(--table-header);
            font-weight: 600;
            position: sticky;
            top: 0;
        }

        tr:hover { background: var(--table-header); }

        .metric { font-weight: 600; }
        .metric.qps { color: var(--metric-qps); }
        .metric.latency { color: var(--metric-latency); }
        .metric.cpu { color: var(--metric-cpu); }
        .metric.mem { color: var(--metric-mem); }

        .chart {
            width: 100%;
            height: 300px;
            margin: 20px 0;
            position: relative;
        }
    </style>
</head>
<body>
    <button class="theme-toggle" onclick="toggleTheme()">🌓</button>

    <div class="container">
        <h1>FE API E2E Benchmark Results</h1>
        <p>Frontend Performance Testing with Increasing Concurrency</p>

        <div class="metadata">
            <div class="metadata-card">
                <h3>FE Endpoint</h3>
                <p>FE_ENDPOINT</p>
            </div>
            <div class="metadata-card">
                <h3>Requests/Level</h3>
                <p>REQUESTS_PER_LEVEL</p>
            </div>
            <div class="metadata-card">
                <h3>Concurrency Levels</h3>
                <p>CONCURRENCY_COUNT levels</p>
            </div>
            <div class="metadata-card">
                <h3>Timestamp</h3>
                <p>TIMESTAMP</p>
            </div>
        </div>

        <h2>MySQL Query Performance</h2>
        <table id="mysql-results">
            <thead>
                <tr>
                    <th>Concurrency</th>
                    <th>QPS</th>
                    <th>Avg Latency (ms)</th>
                    <th>Avg CPU (%)</th>
                    <th>Avg Memory (%)</th>
                </tr>
            </thead>
            <tbody></tbody>
        </table>

        <h2>Stream Load Performance</h2>
        <table id="stream-results">
            <thead>
                <tr>
                    <th>Concurrency</th>
                    <th>RPS</th>
                    <th>Avg Latency (ms)</th>
                    <th>Avg CPU (%)</th>
                    <th>Avg Memory (%)</th>
                </tr>
            </thead>
            <tbody></tbody>
        </table>
    </div>

    <script>
        function toggleTheme() {
            const html = document.documentElement;
            const currentTheme = html.getAttribute('data-theme');
            const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
            html.setAttribute('data-theme', newTheme);
            localStorage.setItem('theme', newTheme);
        }

        const savedTheme = localStorage.getItem('theme') || 'light';
        document.documentElement.setAttribute('data-theme', savedTheme);

        const results = RESULTS_JSON;

        // Render MySQL results
        const mysqlTbody = document.querySelector('#mysql-results tbody');
        Object.entries(results.mysql_query).forEach(([concurrency, data]) => {
            mysqlTbody.innerHTML += `
                <tr>
                    <td><strong>${concurrency}</strong></td>
                    <td class="metric qps">${data.qps}</td>
                    <td class="metric latency">${data.avg_latency_ms}</td>
                    <td class="metric cpu">${data.avg_cpu_percent}</td>
                    <td class="metric mem">${data.avg_mem_percent}</td>
                </tr>
            `;
        });

        // Render Stream Load results
        const streamTbody = document.querySelector('#stream-results tbody');
        Object.entries(results.stream_load).forEach(([concurrency, data]) => {
            streamTbody.innerHTML += `
                <tr>
                    <td><strong>${concurrency}</strong></td>
                    <td class="metric qps">${data.rps}</td>
                    <td class="metric latency">${data.avg_latency_ms}</td>
                    <td class="metric cpu">${data.avg_cpu_percent}</td>
                    <td class="metric mem">${data.avg_mem_percent}</td>
                </tr>
            `;
        });
    </script>
</body>
</html>
HTMLEOF

# Replace placeholders
results_json=$(cat "$OUTPUT_JSON")
sed -i "s|FE_ENDPOINT|${FE_HOST}:${FE_MYSQL_PORT}|g" "$OUTPUT_HTML"
sed -i "s|REQUESTS_PER_LEVEL|${REQUESTS_PER_LEVEL}|g" "$OUTPUT_HTML"
sed -i "s|CONCURRENCY_COUNT|$(echo $CONCURRENCY_LEVELS | wc -w)|g" "$OUTPUT_HTML"
sed -i "s|TIMESTAMP|$(date '+%Y-%m-%d %H:%M:%S')|g" "$OUTPUT_HTML"
sed -i "s|RESULTS_JSON|$results_json|g" "$OUTPUT_HTML"

success "Benchmark complete!"
echo
log "Results saved to:"
echo "  - JSON: $OUTPUT_JSON"
echo "  - HTML: $OUTPUT_HTML"
echo

# Cleanup
mysql -h "$FE_HOST" -P "$FE_MYSQL_PORT" -u root -e "DROP DATABASE IF EXISTS ${DATABASE}" 2>/dev/null || true
