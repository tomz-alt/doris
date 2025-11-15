#!/bin/bash
#
# TPC-H SF100 Benchmark: Rust FE vs Java FE Comparison
#
# This script runs comprehensive benchmarks comparing Rust FE optimizations
# against Java FE baseline on TPC-H Scale Factor 100 (100GB dataset).
#
# Requirements:
# - Doris cluster with Java FE and Rust FE
# - TPC-H SF100 data loaded in both
# - MySQL client installed
# - Python 3 with pandas for analysis
#
# Usage:
#   ./benchmark_sf100_comparison.sh
#
# Environment Variables:
#   TPCH_SCALE          - TPC-H scale factor (default: 100)
#   NUM_ITERATIONS      - Number of iterations per query (default: 5)
#   JAVA_FE_HOST        - Java FE host (default: 127.0.0.1)
#   JAVA_FE_PORT        - Java FE port (default: 9030)
#   RUST_FE_HOST        - Rust FE host (default: 127.0.0.1)
#   RUST_FE_PORT        - Rust FE port (default: 9031)
#   OUTPUT_DIR          - Results directory (default: ./benchmark_results_sf100)
#   SKIP_WARMUP         - Skip warmup phase (default: false)
#   QUICK_MODE          - Run subset of queries for quick validation (default: false)

set -e

# Configuration
TPCH_SCALE=${TPCH_SCALE:-100}
NUM_ITERATIONS=${NUM_ITERATIONS:-5}
JAVA_FE_HOST=${JAVA_FE_HOST:-127.0.0.1}
JAVA_FE_PORT=${JAVA_FE_PORT:-9030}
RUST_FE_HOST=${RUST_FE_HOST:-127.0.0.1}
RUST_FE_PORT=${RUST_FE_PORT:-9031}
OUTPUT_DIR=${OUTPUT_DIR:-./benchmark_results_sf100}
SKIP_WARMUP=${SKIP_WARMUP:-false}
QUICK_MODE=${QUICK_MODE:-false}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# TPC-H Queries (all 22)
TPCH_QUERIES=(
    "Q1:Pricing Summary Report"
    "Q2:Minimum Cost Supplier"
    "Q3:Shipping Priority"
    "Q4:Order Priority Checking"
    "Q5:Local Supplier Volume"
    "Q6:Forecasting Revenue Change"
    "Q7:Volume Shipping"
    "Q8:National Market Share"
    "Q9:Product Type Profit Measure"
    "Q10:Returned Item Reporting"
    "Q11:Important Stock Identification"
    "Q12:Shipping Modes and Order Priority"
    "Q13:Customer Distribution"
    "Q14:Promotion Effect"
    "Q15:Top Supplier"
    "Q16:Parts/Supplier Relationship"
    "Q17:Small-Quantity-Order Revenue"
    "Q18:Large Volume Customer"
    "Q19:Discounted Revenue"
    "Q20:Potential Part Promotion"
    "Q21:Suppliers Who Kept Orders Waiting"
    "Q22:Global Sales Opportunity"
)

# Quick mode: only run representative queries
if [ "$QUICK_MODE" = "true" ]; then
    TPCH_QUERIES=(
        "Q1:Pricing Summary Report"
        "Q3:Shipping Priority"
        "Q6:Forecasting Revenue Change"
        "Q9:Product Type Profit Measure"
        "Q18:Large Volume Customer"
    )
    NUM_ITERATIONS=3
    echo -e "${YELLOW}Quick mode enabled: Testing 5 representative queries with 3 iterations${NC}"
fi

# Feature flag configurations for Rust FE
declare -A RUST_CONFIGS
RUST_CONFIGS["baseline"]=""
RUST_CONFIGS["cost_based_join"]="DORIS_COST_BASED_JOIN=true"
RUST_CONFIGS["partition_pruning"]="DORIS_PARTITION_PRUNING=true"
RUST_CONFIGS["runtime_filters"]="DORIS_RUNTIME_FILTERS=true"
RUST_CONFIGS["all_optimizations"]="DORIS_COST_BASED_JOIN=true DORIS_PARTITION_PRUNING=true DORIS_RUNTIME_FILTERS=true DORIS_BUCKET_SHUFFLE=true"

# Prepare output directory
mkdir -p "$OUTPUT_DIR"/{logs,profiles,csv}
LOGFILE="$OUTPUT_DIR/benchmark.log"
RESULTS_CSV="$OUTPUT_DIR/csv/benchmark_results.csv"

log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOGFILE"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOGFILE"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOGFILE"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOGFILE"
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."

    # Check mysql client
    if ! command -v mysql &> /dev/null; then
        error "mysql client not found. Please install MySQL client."
        exit 1
    fi

    # Check Java FE connectivity
    if ! mysql -h "$JAVA_FE_HOST" -P "$JAVA_FE_PORT" -u root -e "SELECT 1" &> /dev/null; then
        error "Cannot connect to Java FE at $JAVA_FE_HOST:$JAVA_FE_PORT"
        exit 1
    fi
    success "Java FE connection OK"

    # Check Rust FE connectivity
    if ! mysql -h "$RUST_FE_HOST" -P "$RUST_FE_PORT" -u root -e "SELECT 1" &> /dev/null; then
        error "Cannot connect to Rust FE at $RUST_FE_HOST:$RUST_FE_PORT"
        exit 1
    fi
    success "Rust FE connection OK"

    # Check database exists
    if ! mysql -h "$JAVA_FE_HOST" -P "$JAVA_FE_PORT" -u root -e "USE tpch_${TPCH_SCALE}" &> /dev/null; then
        error "Database tpch_${TPCH_SCALE} not found in Java FE"
        exit 1
    fi

    if ! mysql -h "$RUST_FE_HOST" -P "$RUST_FE_PORT" -u root -e "USE tpch_${TPCH_SCALE}" &> /dev/null; then
        error "Database tpch_${TPCH_SCALE} not found in Rust FE"
        exit 1
    fi
    success "TPC-H SF${TPCH_SCALE} database verified"

    # Check Python for analysis
    if ! command -v python3 &> /dev/null; then
        warn "Python3 not found. Statistical analysis will be skipped."
    fi
}

# Warmup queries
warmup() {
    if [ "$SKIP_WARMUP" = "true" ]; then
        warn "Skipping warmup phase"
        return
    fi

    log "Starting warmup phase..."

    # Warmup Java FE
    log "Warming up Java FE..."
    for query_info in "${TPCH_QUERIES[@]}"; do
        query_num=$(echo "$query_info" | cut -d: -f1 | sed 's/Q//')
        mysql -h "$JAVA_FE_HOST" -P "$JAVA_FE_PORT" -u root tpch_${TPCH_SCALE} \
            < "../queries/q${query_num}.sql" > /dev/null 2>&1 || true
    done
    success "Java FE warmup complete"

    # Warmup Rust FE
    log "Warming up Rust FE..."
    for query_info in "${TPCH_QUERIES[@]}"; do
        query_num=$(echo "$query_info" | cut -d: -f1 | sed 's/Q//')
        mysql -h "$RUST_FE_HOST" -P "$RUST_FE_PORT" -u root tpch_${TPCH_SCALE} \
            < "../queries/q${query_num}.sql" > /dev/null 2>&1 || true
    done
    success "Rust FE warmup complete"
}

# Run a single query and measure timing
run_query() {
    local fe_type=$1
    local host=$2
    local port=$3
    local query_num=$4
    local query_name=$5
    local config=$6
    local iteration=$7

    local query_file="../queries/q${query_num}.sql"
    if [ ! -f "$query_file" ]; then
        error "Query file not found: $query_file"
        return 1
    fi

    # Set environment variables for this run
    if [ -n "$config" ]; then
        eval "export $config"
    fi

    # Execute query and measure time
    local start_time=$(date +%s%N)
    local output_file="$OUTPUT_DIR/logs/${fe_type}_Q${query_num}_${config}_iter${iteration}.log"

    mysql -h "$host" -P "$port" -u root tpch_${TPCH_SCALE} \
        -e "$(cat $query_file); SHOW QUERY PROFILE;" \
        > "$output_file" 2>&1
    local exit_code=$?

    local end_time=$(date +%s%N)
    local elapsed_ms=$(( ($end_time - $start_time) / 1000000 ))

    # Extract metrics from query profile
    local profile_file="$OUTPUT_DIR/profiles/${fe_type}_Q${query_num}_${config}_iter${iteration}.profile"
    grep "Profile" "$output_file" > "$profile_file" || true

    # Clear environment variables
    if [ -n "$config" ]; then
        for var in $(echo "$config" | tr ' ' '\n' | cut -d= -f1); do
            unset $var
        done
    fi

    if [ $exit_code -eq 0 ]; then
        echo "$fe_type,Q${query_num},$query_name,$config,$iteration,$elapsed_ms" >> "$RESULTS_CSV"
        log "  Q${query_num} (${config:-default}) iter$iteration: ${elapsed_ms}ms"
        return 0
    else
        error "Query Q${query_num} (${config}) iter$iteration failed"
        return 1
    fi
}

# Run benchmarks for Java FE
benchmark_java_fe() {
    log "Starting Java FE benchmarks..."

    for query_info in "${TPCH_QUERIES[@]}"; do
        query_num=$(echo "$query_info" | cut -d: -f1 | sed 's/Q//')
        query_name=$(echo "$query_info" | cut -d: -f2)

        log "Running Java FE Q${query_num}: $query_name"

        for ((i=1; i<=NUM_ITERATIONS; i++)); do
            run_query "java" "$JAVA_FE_HOST" "$JAVA_FE_PORT" \
                "$query_num" "$query_name" "baseline" "$i" || warn "Query failed"
            sleep 2  # Brief pause between iterations
        done
    done

    success "Java FE benchmarks complete"
}

# Run benchmarks for Rust FE
benchmark_rust_fe() {
    log "Starting Rust FE benchmarks..."

    for config_name in "${!RUST_CONFIGS[@]}"; do
        config_value="${RUST_CONFIGS[$config_name]}"
        log "Testing configuration: $config_name"

        for query_info in "${TPCH_QUERIES[@]}"; do
            query_num=$(echo "$query_info" | cut -d: -f1 | sed 's/Q//')
            query_name=$(echo "$query_info" | cut -d: -f2)

            log "Running Rust FE Q${query_num}: $query_name ($config_name)"

            for ((i=1; i<=NUM_ITERATIONS; i++)); do
                run_query "rust" "$RUST_FE_HOST" "$RUST_FE_PORT" \
                    "$query_num" "$query_name" "$config_name" "$i" || warn "Query failed"
                sleep 2
            done
        done
    done

    success "Rust FE benchmarks complete"
}

# Analyze results
analyze_results() {
    log "Analyzing results..."

    if ! command -v python3 &> /dev/null; then
        warn "Python3 not available. Skipping statistical analysis."
        return
    fi

    python3 - <<'PYTHON_SCRIPT'
import sys
import pandas as pd
import numpy as np
from pathlib import Path

# Read results
results_file = Path(sys.argv[1]) / 'csv' / 'benchmark_results.csv'
if not results_file.exists():
    print(f"Results file not found: {results_file}")
    sys.exit(1)

df = pd.read_csv(results_file, names=['FE_Type', 'Query', 'Query_Name', 'Config', 'Iteration', 'Time_ms'])

# Calculate statistics
summary = df.groupby(['FE_Type', 'Query', 'Config'])['Time_ms'].agg([
    ('Mean', 'mean'),
    ('Median', 'median'),
    ('Std', 'std'),
    ('Min', 'min'),
    ('Max', 'max')
]).reset_index()

# Save detailed summary
summary_file = Path(sys.argv[1]) / 'csv' / 'summary_statistics.csv'
summary.to_csv(summary_file, index=False)
print(f"Summary statistics saved to: {summary_file}")

# Calculate speedups
java_baseline = summary[summary['FE_Type'] == 'java'].set_index('Query')['Median']
rust_summary = summary[summary['FE_Type'] == 'rust'].copy()
rust_summary['Speedup'] = rust_summary.apply(
    lambda row: java_baseline.get(row['Query'], 0) / row['Median'] if row['Median'] > 0 else 0,
    axis=1
)

# Save speedup analysis
speedup_file = Path(sys.argv[1]) / 'csv' / 'speedup_analysis.csv'
rust_summary.to_csv(speedup_file, index=False)
print(f"Speedup analysis saved to: {speedup_file}")

# Generate markdown report
report_file = Path(sys.argv[1]) / 'BENCHMARK_SUMMARY.md'
with open(report_file, 'w') as f:
    f.write("# TPC-H SF100 Benchmark Results\n\n")
    f.write(f"**Date**: {pd.Timestamp.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")

    f.write("## Overall Summary\n\n")

    # Java FE baseline
    java_avg = summary[summary['FE_Type'] == 'java']['Median'].mean()
    f.write(f"**Java FE Average Query Time**: {java_avg:.2f} ms\n\n")

    # Rust FE by configuration
    for config in rust_summary['Config'].unique():
        config_data = rust_summary[rust_summary['Config'] == config]
        avg_time = config_data['Median'].mean()
        avg_speedup = config_data['Speedup'].mean()
        f.write(f"**Rust FE ({config}) Average**: {avg_time:.2f} ms ({avg_speedup:.2f}x speedup)\n\n")

    f.write("## Per-Query Results\n\n")
    f.write("| Query | Java FE (ms) | Rust Baseline (ms) | Rust All Opts (ms) | Speedup |\n")
    f.write("|-------|--------------|-------------------|-------------------|----------|\n")

    for query in sorted(df['Query'].unique()):
        java_time = summary[(summary['FE_Type'] == 'java') & (summary['Query'] == query)]['Median'].values
        rust_baseline_time = summary[(summary['FE_Type'] == 'rust') & (summary['Config'] == 'baseline') & (summary['Query'] == query)]['Median'].values
        rust_all_time = summary[(summary['FE_Type'] == 'rust') & (summary['Config'] == 'all_optimizations') & (summary['Query'] == query)]['Median'].values

        if len(java_time) > 0 and len(rust_all_time) > 0:
            speedup = java_time[0] / rust_all_time[0] if rust_all_time[0] > 0 else 0
            f.write(f"| {query} | {java_time[0]:.0f} | {rust_baseline_time[0]:.0f} | {rust_all_time[0]:.0f} | {speedup:.2f}x |\n")

print(f"Benchmark report saved to: {report_file}")

PYTHON_SCRIPT
    python3 - "$OUTPUT_DIR"

    success "Analysis complete"
}

# Main execution
main() {
    log "="*80
    log "TPC-H SF${TPCH_SCALE} Benchmark: Rust FE vs Java FE"
    log "="*80
    log ""
    log "Configuration:"
    log "  Scale Factor: SF${TPCH_SCALE}"
    log "  Iterations: ${NUM_ITERATIONS}"
    log "  Java FE: ${JAVA_FE_HOST}:${JAVA_FE_PORT}"
    log "  Rust FE: ${RUST_FE_HOST}:${RUST_FE_PORT}"
    log "  Output: ${OUTPUT_DIR}"
    log ""

    # Initialize results CSV
    echo "FE_Type,Query,Query_Name,Config,Iteration,Time_ms" > "$RESULTS_CSV"

    # Run benchmark phases
    check_prerequisites
    warmup

    log ""
    log "Starting benchmark execution..."
    local start_time=$(date +%s)

    benchmark_java_fe
    benchmark_rust_fe

    local end_time=$(date +%s)
    local total_time=$((end_time - start_time))

    log ""
    success "Benchmark execution complete in ${total_time}s"

    # Analyze and generate reports
    analyze_results

    log ""
    log "="*80
    log "Benchmark Complete!"
    log "="*80
    log "Results directory: $OUTPUT_DIR"
    log "  - Summary: $OUTPUT_DIR/BENCHMARK_SUMMARY.md"
    log "  - Raw data: $OUTPUT_DIR/csv/benchmark_results.csv"
    log "  - Statistics: $OUTPUT_DIR/csv/summary_statistics.csv"
    log "  - Speedup analysis: $OUTPUT_DIR/csv/speedup_analysis.csv"
    log "  - Query logs: $OUTPUT_DIR/logs/"
    log "  - Query profiles: $OUTPUT_DIR/profiles/"
    log ""
}

# Run main function
main "$@"
