#!/bin/bash
set -e

# Full TPC-H and TPC-DS Benchmark Suite
# Runs both benchmarks with configurable rounds and generates comparison HTML

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROUNDS=${ROUNDS:-5}
SCALE=${SCALE:-1}
OUTPUT_DIR="${OUTPUT_DIR:-./benchmark_results}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log() { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }
section() { echo -e "\n${CYAN}$*${NC}\n"; }

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Full TPC-H and TPC-DS benchmark suite comparing Java FE vs Rust FE

Options:
  -r, --rounds ROUNDS        Number of benchmark rounds (default: 5)
  -s, --scale SCALE          Scale factor (default: 1)
  -o, --output-dir DIR       Output directory (default: ./benchmark_results)
  --tpch-only                Run only TPC-H benchmark
  --tpcds-only               Run only TPC-DS benchmark
  --java-host HOST           Java FE host (default: 127.0.0.1)
  --java-port PORT           Java FE MySQL port (default: 9030)
  --rust-host HOST           Rust FE host (default: 127.0.0.1)
  --rust-port PORT           Rust FE MySQL port (default: 9031)
  -h, --help                 Show this help

Examples:
  # Run full benchmark with 5 rounds
  $0 -r 5

  # Run only TPC-H with 3 rounds
  $0 --tpch-only -r 3

  # Custom output directory
  $0 -r 5 -o /tmp/benchmark_$(date +%Y%m%d)

Environment Variables:
  ROUNDS       - Number of benchmark rounds (default: 5)
  SCALE        - TPC-H/TPC-DS scale factor (default: 1)
  OUTPUT_DIR   - Output directory for results

Output:
  - {output_dir}/tpch_results.html    - TPC-H comparison chart
  - {output_dir}/tpcds_results.html   - TPC-DS comparison chart
  - {output_dir}/summary.html         - Combined summary
  - {output_dir}/tpch_results.json    - TPC-H raw data
  - {output_dir}/tpcds_results.json   - TPC-DS raw data

EOF
    exit 0
}

# Parse arguments
TPCH_ONLY=false
TPCDS_ONLY=false
JAVA_HOST="127.0.0.1"
JAVA_PORT="9030"
RUST_HOST="127.0.0.1"
RUST_PORT="9031"

while [[ $# -gt 0 ]]; do
    case $1 in
        -r|--rounds) ROUNDS="$2"; shift 2 ;;
        -s|--scale) SCALE="$2"; shift 2 ;;
        -o|--output-dir) OUTPUT_DIR="$2"; shift 2 ;;
        --tpch-only) TPCH_ONLY=true; shift ;;
        --tpcds-only) TPCDS_ONLY=true; shift ;;
        --java-host) JAVA_HOST="$2"; shift 2 ;;
        --java-port) JAVA_PORT="$2"; shift 2 ;;
        --rust-host) RUST_HOST="$2"; shift 2 ;;
        --rust-port) RUST_PORT="$2"; shift 2 ;;
        -h|--help) usage ;;
        *) error "Unknown option: $1" ;;
    esac
done

# Create output directory
mkdir -p "$OUTPUT_DIR"

section "===================================================="
section "TPC-H & TPC-DS Full Benchmark Suite"
section "Java FE vs Rust FE Comparison"
section "===================================================="

log "Configuration:"
log "  Rounds: $ROUNDS"
log "  Scale Factor: SF$SCALE"
log "  Output Directory: $OUTPUT_DIR"
log "  Java FE: ${JAVA_HOST}:${JAVA_PORT}"
log "  Rust FE: ${RUST_HOST}:${RUST_PORT}"
echo

# Check dependencies
log "Checking dependencies..."
command -v mysql >/dev/null 2>&1 || error "mysql client not found. Install: apt-get install mysql-client"
command -v bc >/dev/null 2>&1 || error "bc not found. Install: apt-get install bc"
success "Dependencies OK"
echo

# Test connections
log "Testing database connections..."
mysql -h "$JAVA_HOST" -P "$JAVA_PORT" -u root -e "SELECT 1" 2>/dev/null || \
    error "Cannot connect to Java FE at ${JAVA_HOST}:${JAVA_PORT}"
mysql -h "$RUST_HOST" -P "$RUST_PORT" -u root -e "SELECT 1" 2>/dev/null || \
    error "Cannot connect to Rust FE at ${RUST_HOST}:${RUST_PORT}"
success "Database connections OK"
echo

# Run TPC-H benchmark
if [ "$TPCDS_ONLY" = false ]; then
    section "Running TPC-H Benchmark (22 queries, $ROUNDS rounds)..."

    "$SCRIPT_DIR/benchmark_tpch.sh" \
        --scale "$SCALE" \
        --rounds "$ROUNDS" \
        --warmup 1 \
        --java-host "$JAVA_HOST" \
        --java-port "$JAVA_PORT" \
        --rust-host "$RUST_HOST" \
        --rust-port "$RUST_PORT" \
        --output-json "$OUTPUT_DIR/tpch_results.json" \
        --output-html "$OUTPUT_DIR/tpch_results.html"

    success "TPC-H benchmark completed!"
    success "Results: $OUTPUT_DIR/tpch_results.html"
    echo
fi

# Run TPC-DS benchmark
if [ "$TPCH_ONLY" = false ]; then
    section "Running TPC-DS Benchmark (99 queries, $ROUNDS rounds)..."

    "$SCRIPT_DIR/benchmark_tpcds.sh" \
        --scale "$SCALE" \
        --rounds "$ROUNDS" \
        --warmup 1 \
        --java-host "$JAVA_HOST" \
        --java-port "$JAVA_PORT" \
        --rust-host "$RUST_HOST" \
        --rust-port "$RUST_PORT" \
        --output-json "$OUTPUT_DIR/tpcds_results.json" \
        --output-html "$OUTPUT_DIR/tpcds_results.html"

    success "TPC-DS benchmark completed!"
    success "Results: $OUTPUT_DIR/tpcds_results.html"
    echo
fi

# Generate combined summary
if [ "$TPCH_ONLY" = false ] && [ "$TPCDS_ONLY" = false ]; then
    section "Generating combined summary..."

    "$SCRIPT_DIR/generate_summary.sh" \
        --tpch-json "$OUTPUT_DIR/tpch_results.json" \
        --tpcds-json "$OUTPUT_DIR/tpcds_results.json" \
        --output "$OUTPUT_DIR/summary.html"

    success "Summary generated: $OUTPUT_DIR/summary.html"
    echo
fi

section "===================================================="
section "Benchmark Complete!"
section "===================================================="

echo
log "Results Location: $OUTPUT_DIR"
echo
log "Generated Files:"
if [ "$TPCDS_ONLY" = false ]; then
    log "  - tpch_results.html    (TPC-H comparison chart)"
    log "  - tpch_results.json    (TPC-H raw data)"
fi
if [ "$TPCH_ONLY" = false ]; then
    log "  - tpcds_results.html   (TPC-DS comparison chart)"
    log "  - tpcds_results.json   (TPC-DS raw data)"
fi
if [ "$TPCH_ONLY" = false ] && [ "$TPCDS_ONLY" = false ]; then
    log "  - summary.html         (Combined summary)"
fi
echo

log "Open results in browser:"
if [ "$TPCDS_ONLY" = false ]; then
    echo "  file://$(realpath "$OUTPUT_DIR/tpch_results.html")"
fi
if [ "$TPCH_ONLY" = false ]; then
    echo "  file://$(realpath "$OUTPUT_DIR/tpcds_results.html")"
fi
if [ "$TPCH_ONLY" = false ] && [ "$TPCDS_ONLY" = false ]; then
    echo "  file://$(realpath "$OUTPUT_DIR/summary.html")"
fi
echo

success "All benchmarks completed successfully!"
