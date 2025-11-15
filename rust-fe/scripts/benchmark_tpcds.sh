#!/bin/bash
set -e

# TPC-DS Benchmark Runner (Pure Bash - ClickBench Style)
# Inspired by https://github.com/ClickHouse/ClickBench

BENCHMARK_NAME="TPC-DS"
SCALE=${SCALE:-1}
ROUNDS=${ROUNDS:-3}
WARMUP=${WARMUP:-1}

JAVA_HOST=${JAVA_HOST:-127.0.0.1}
JAVA_PORT=${JAVA_PORT:-9030}
RUST_HOST=${RUST_HOST:-127.0.0.1}
RUST_PORT=${RUST_PORT:-9031}

DATABASE="tpcds_sf${SCALE}"
QUERIES_DIR="$(dirname "$0")/tpcds/queries"
OUTPUT_JSON="tpcds_results.json"
OUTPUT_HTML="tpcds_results.html"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

# Parse arguments (check for help first before dependency checks)
while [[ $# -gt 0 ]]; do
    case $1 in
        -s|--scale) SCALE="$2"; shift 2 ;;
        -r|--rounds) ROUNDS="$2"; shift 2 ;;
        -w|--warmup) WARMUP="$2"; shift 2 ;;
        --java-host) JAVA_HOST="$2"; shift 2 ;;
        --java-port) JAVA_PORT="$2"; shift 2 ;;
        --rust-host) RUST_HOST="$2"; shift 2 ;;
        --rust-port) RUST_PORT="$2"; shift 2 ;;
        --output-json) OUTPUT_JSON="$2"; shift 2 ;;
        --output-html) OUTPUT_HTML="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  -s, --scale SCALE          Scale factor (default: 1)"
            echo "  -r, --rounds ROUNDS        Number of benchmark rounds (default: 3)"
            echo "  -w, --warmup WARMUP        Number of warmup rounds (default: 1)"
            echo "  --java-host HOST           Java FE host (default: 127.0.0.1)"
            echo "  --java-port PORT           Java FE port (default: 9030)"
            echo "  --rust-host HOST           Rust FE host (default: 127.0.0.1)"
            echo "  --rust-port PORT           Rust FE port (default: 9031)"
            echo "  --output-json FILE         Output JSON file (default: tpcds_results.json)"
            echo "  --output-html FILE         Output HTML file (default: tpcds_results.html)"
            echo "  -h, --help                 Show this help"
            exit 0
            ;;
        *) error "Unknown option: $1" ;;
    esac
done

# Check dependencies
command -v mysql >/dev/null 2>&1 || error "mysql client not found. Install with: apt-get install mysql-client"
command -v bc >/dev/null 2>&1 || error "bc not found. Install with: apt-get install bc"

log "Starting $BENCHMARK_NAME Benchmark"
log "Scale Factor: SF${SCALE}"
log "Rounds: ${ROUNDS} (Warmup: ${WARMUP})"
log "Java FE: ${JAVA_HOST}:${JAVA_PORT}"
log "Rust FE: ${RUST_HOST}:${RUST_PORT}"
echo

# Test connections
log "Testing database connections..."
mysql -h "$JAVA_HOST" -P "$JAVA_PORT" -u root -e "SELECT 1" 2>/dev/null || error "Cannot connect to Java FE"
mysql -h "$RUST_HOST" -P "$RUST_PORT" -u root -e "SELECT 1" 2>/dev/null || error "Cannot connect to Rust FE"
success "Database connections OK"
echo

# Get query files
QUERIES=($(ls "$QUERIES_DIR"/*.sql | sort -V))
QUERY_COUNT=${#QUERIES[@]}

if [ $QUERY_COUNT -eq 0 ]; then
    error "No queries found in $QUERIES_DIR"
fi

log "Found $QUERY_COUNT queries"
echo

# Run query and measure time
run_query() {
    local host=$1
    local port=$2
    local db=$3
    local query_file=$4

    local start_ns=$(date +%s%N)
    mysql -h "$host" -P "$port" -u root -D "$db" < "$query_file" >/dev/null 2>&1
    local exit_code=$?
    local end_ns=$(date +%s%N)

    if [ $exit_code -ne 0 ]; then
        echo "ERROR"
        return 1
    fi

    # Calculate duration in seconds with millisecond precision
    local duration_ns=$((end_ns - start_ns))
    local duration_sec=$(echo "scale=3; $duration_ns / 1000000000" | bc)
    echo "$duration_sec"
}

# Statistics functions
calculate_mean() {
    local sum=0
    local count=0
    for val in "$@"; do
        sum=$(echo "$sum + $val" | bc)
        count=$((count + 1))
    done
    echo "scale=3; $sum / $count" | bc
}

calculate_geomean() {
    local product=1
    local count=0
    for val in "$@"; do
        product=$(echo "$product * $val" | bc)
        count=$((count + 1))
    done
    echo "scale=3; e(l($product) / $count)" | bc -l
}

# Initialize results
declare -A JAVA_RESULTS
declare -A RUST_RESULTS

# Run benchmarks
for query_file in "${QUERIES[@]}"; do
    query_name=$(basename "$query_file" .sql)
    log "Running $query_name..."

    java_times=()
    rust_times=()

    # Warmup rounds
    for ((i=1; i<=WARMUP; i++)); do
        echo -n "  Warmup $i/$WARMUP (Java)... "
        run_query "$JAVA_HOST" "$JAVA_PORT" "$DATABASE" "$query_file" >/dev/null
        echo "done"

        echo -n "  Warmup $i/$WARMUP (Rust)... "
        run_query "$RUST_HOST" "$RUST_PORT" "$DATABASE" "$query_file" >/dev/null
        echo "done"
    done

    # Benchmark rounds
    for ((i=1; i<=ROUNDS; i++)); do
        echo -n "  Round $i/$ROUNDS (Java)... "
        java_time=$(run_query "$JAVA_HOST" "$JAVA_PORT" "$DATABASE" "$query_file")
        if [ "$java_time" = "ERROR" ]; then
            warn "Java FE query failed"
            java_time="0"
        else
            echo "${java_time}s"
        fi
        java_times+=("$java_time")

        echo -n "  Round $i/$ROUNDS (Rust)... "
        rust_time=$(run_query "$RUST_HOST" "$RUST_PORT" "$DATABASE" "$query_file")
        if [ "$rust_time" = "ERROR" ]; then
            warn "Rust FE query failed"
            rust_time="0"
        else
            echo "${rust_time}s"
        fi
        rust_times+=("$rust_time")
    done

    # Calculate statistics
    java_mean=$(calculate_mean "${java_times[@]}")
    rust_mean=$(calculate_mean "${rust_times[@]}")

    JAVA_RESULTS[$query_name]=$java_mean
    RUST_RESULTS[$query_name]=$rust_mean

    if [ "$(echo "$rust_mean > 0" | bc)" -eq 1 ]; then
        speedup=$(echo "scale=2; $java_mean / $rust_mean" | bc)
        success "$query_name: Java=${java_mean}s, Rust=${rust_mean}s, Speedup=${speedup}x"
    else
        warn "$query_name: Rust query failed"
    fi

    echo
done

# Calculate overall statistics
java_geomean=$(calculate_geomean "${JAVA_RESULTS[@]}")
rust_geomean=$(calculate_geomean "${RUST_RESULTS[@]}")
overall_speedup=$(echo "scale=2; $java_geomean / $rust_geomean" | bc)

log "Generating results..."

# Generate JSON results
cat > "$OUTPUT_JSON" <<EOF
{
  "benchmark": "$BENCHMARK_NAME",
  "scale_factor": $SCALE,
  "rounds": $ROUNDS,
  "warmup": $WARMUP,
  "timestamp": "$(date -Iseconds)",
  "java_fe": "${JAVA_HOST}:${JAVA_PORT}",
  "rust_fe": "${RUST_HOST}:${RUST_PORT}",
  "queries": {
EOF

first=true
for query_file in "${QUERIES[@]}"; do
    query_name=$(basename "$query_file" .sql)
    java_time=${JAVA_RESULTS[$query_name]}
    rust_time=${RUST_RESULTS[$query_name]}
    speedup=$(echo "scale=2; $java_time / $rust_time" | bc)

    [ "$first" = false ] && echo "," >> "$OUTPUT_JSON"
    first=false

    cat >> "$OUTPUT_JSON" <<EOF
    "$query_name": {
      "java_mean": $java_time,
      "rust_mean": $rust_time,
      "speedup": $speedup
    }
EOF
done

cat >> "$OUTPUT_JSON" <<EOF

  },
  "overall": {
    "java_geomean": $java_geomean,
    "rust_geomean": $rust_geomean,
    "speedup": $overall_speedup
  }
}
EOF

# Generate HTML report (ClickBench style)
cat > "$OUTPUT_HTML" <<'HTMLEOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>TPC-DS Benchmark Results</title>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&display=swap" rel="stylesheet">
    <style>
        :root {
            --color: #000;
            --background-color: #fff;
            --table-header: #f5f5f5;
            --table-border: #e0e0e0;
            --bar-java: #ff6b35;
            --bar-rust: #4ecdc4;
            --speedup-good: #4caf50;
            --speedup-bad: #f44336;
        }

        [data-theme="dark"] {
            --color: #ccc;
            --background-color: #04293A;
            --table-header: #053b52;
            --table-border: #064663;
            --bar-java: #ff8c61;
            --bar-rust: #6eddcc;
        }

        * { box-sizing: border-box; }

        body {
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
            background: var(--background-color);
            color: var(--color);
            margin: 0;
            padding: 20px;
            transition: background-color 0.3s, color 0.3s;
        }

        .container { max-width: 1200px; margin: 0 auto; }

        h1 {
            font-size: 2rem;
            margin-bottom: 0.5rem;
            font-weight: 700;
        }

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

        .bar-container {
            display: flex;
            align-items: center;
            gap: 10px;
        }

        .bar {
            height: 20px;
            border-radius: 2px;
            min-width: 2px;
        }

        .bar-java { background: var(--bar-java); }
        .bar-rust { background: var(--bar-rust); }

        .speedup {
            font-weight: 600;
            font-variant-numeric: tabular-nums;
        }

        .speedup.good { color: var(--speedup-good); }
        .speedup.bad { color: var(--speedup-bad); }

        .summary {
            background: var(--table-header);
            padding: 20px;
            border-radius: 4px;
            margin: 20px 0;
            border: 2px solid var(--bar-rust);
        }

        .summary h2 {
            margin-top: 0;
            font-size: 1.5rem;
        }

        .summary .speedup {
            font-size: 2rem;
        }
    </style>
</head>
<body>
    <button class="theme-toggle" onclick="toggleTheme()">🌓</button>

    <div class="container">
        <h1>BENCHMARK_TITLE</h1>
        <p>Java FE vs Rust FE Performance Comparison</p>

        <div class="metadata">
            <div class="metadata-card">
                <h3>Benchmark</h3>
                <p>BENCHMARK_NAME</p>
            </div>
            <div class="metadata-card">
                <h3>Scale Factor</h3>
                <p>SF SCALE_FACTOR</p>
            </div>
            <div class="metadata-card">
                <h3>Rounds</h3>
                <p>ROUNDS_COUNT</p>
            </div>
            <div class="metadata-card">
                <h3>Overall Speedup</h3>
                <p class="speedup good">OVERALL_SPEEDUPx</p>
            </div>
        </div>

        <div class="summary">
            <h2>Summary</h2>
            <p><strong>Geometric Mean Speedup:</strong> <span class="speedup good">OVERALL_SPEEDUPx</span></p>
            <p><strong>Rust FE is OVERALL_SPEEDUP times faster than Java FE</strong></p>
        </div>

        <h2>Query Execution Times (Lower is Better)</h2>
        <table>
            <thead>
                <tr>
                    <th>Query</th>
                    <th>Java FE (s)</th>
                    <th>Rust FE (s)</th>
                    <th>Speedup</th>
                </tr>
            </thead>
            <tbody id="results">
                <!-- Results will be inserted here -->
            </tbody>
        </table>
    </div>

    <script>
        // Theme toggle
        function toggleTheme() {
            const html = document.documentElement;
            const currentTheme = html.getAttribute('data-theme');
            const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
            html.setAttribute('data-theme', newTheme);
            localStorage.setItem('theme', newTheme);
        }

        // Load saved theme
        const savedTheme = localStorage.getItem('theme') || 'light';
        document.documentElement.setAttribute('data-theme', savedTheme);

        // Results data
        const results = RESULTS_JSON;

        // Find max time for bar scaling
        const maxTime = Math.max(
            ...Object.values(results).map(r => Math.max(r.java_mean, r.rust_mean))
        );

        // Render results
        const tbody = document.getElementById('results');
        Object.entries(results).forEach(([query, data]) => {
            const javaWidth = (data.java_mean / maxTime * 300) + 'px';
            const rustWidth = (data.rust_mean / maxTime * 300) + 'px';
            const speedupClass = data.speedup >= 1 ? 'good' : 'bad';

            tbody.innerHTML += `
                <tr>
                    <td><strong>${query.toUpperCase()}</strong></td>
                    <td>
                        <div class="bar-container">
                            <div class="bar bar-java" style="width: ${javaWidth}"></div>
                            <span>${data.java_mean.toFixed(3)}</span>
                        </div>
                    </td>
                    <td>
                        <div class="bar-container">
                            <div class="bar bar-rust" style="width: ${rustWidth}"></div>
                            <span>${data.rust_mean.toFixed(3)}</span>
                        </div>
                    </td>
                    <td><span class="speedup ${speedupClass}">${data.speedup.toFixed(2)}x</span></td>
                </tr>
            `;
        });
    </script>
</body>
</html>
HTMLEOF

# Replace placeholders in HTML
sed -i "s/BENCHMARK_TITLE/$BENCHMARK_NAME Benchmark Results/g" "$OUTPUT_HTML"
sed -i "s/BENCHMARK_NAME/$BENCHMARK_NAME/g" "$OUTPUT_HTML"
sed -i "s/SCALE_FACTOR/$SCALE/g" "$OUTPUT_HTML"
sed -i "s/ROUNDS_COUNT/$ROUNDS/g" "$OUTPUT_HTML"
sed -i "s/OVERALL_SPEEDUP/$overall_speedup/g" "$OUTPUT_HTML"

# Insert results JSON
results_json=$(cat "$OUTPUT_JSON" | grep -A 1000 '"queries"' | grep -B 1000 '  },' | grep -v '"queries"' | grep -v '  },' | sed 's/^  //')
sed -i "s|RESULTS_JSON|$results_json|g" "$OUTPUT_HTML"

success "Benchmark complete!"
echo
log "Results saved to:"
echo "  - JSON: $OUTPUT_JSON"
echo "  - HTML: $OUTPUT_HTML"
echo
success "Overall Geometric Mean Speedup: ${overall_speedup}x"
