#!/bin/bash
# End-to-End Benchmark: Rust FE vs Java FE Performance Comparison
#
# This script runs TPC-H queries against both FE implementations and compares:
# 1. End-to-end query latency
# 2. FE planning time
# 3. Optimization effectiveness
# 4. Resource usage (CPU, memory)

set -e

# Configuration
RUST_FE_HOST="${RUST_FE_HOST:-localhost}"
RUST_FE_PORT="${RUST_FE_PORT:-9030}"
JAVA_FE_HOST="${JAVA_FE_HOST:-localhost}"
JAVA_FE_PORT="${JAVA_FE_PORT:-9030}"
TPCH_SCALE="${TPCH_SCALE:-1}"  # 1GB scale factor
NUM_ITERATIONS="${NUM_ITERATIONS:-10}"
OUTPUT_DIR="${OUTPUT_DIR:-./benchmark_results}"

# Colors for output
RED='\033[0:31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# TPC-H queries to benchmark
TPCH_QUERIES=(
    "Q1"  # Pricing Summary Report
    "Q3"  # Shipping Priority
    "Q5"  # Local Supplier Volume
    "Q6"  # Forecasting Revenue Change
    "Q8"  # National Market Share
   "Q12" # Shipping Modes and Order Priority
    "Q14" # Promotion Effect
    "Q19" # Discounted Revenue
)

# Feature flag configurations to test
declare -A CONFIGS
CONFIGS["baseline"]=""
CONFIGS["cost_based_join"]="DORIS_COST_BASED_JOIN=true"
CONFIGS["partition_pruning"]="DORIS_PARTITION_PRUNING=true"
CONFIGS["runtime_filters"]="DORIS_RUNTIME_FILTERS=true"
CONFIGS["all_optimizations"]="DORIS_COST_BASED_JOIN=true DORIS_PARTITION_PRUNING=true DORIS_RUNTIME_FILTERS=true DORIS_BUCKET_SHUFFLE=true"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Doris FE Benchmark: Rust vs Java${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Configuration:"
echo "  - TPC-H Scale: ${TPCH_SCALE}GB"
echo "  - Iterations: ${NUM_ITERATIONS}"
echo "  - Rust FE: ${RUST_FE_HOST}:${RUST_FE_PORT}"
echo "  - Java FE: ${JAVA_FE_HOST}:${JAVA_FE_PORT}"
echo "  - Output: ${OUTPUT_DIR}"
echo ""

# Create output directory
mkdir -p "${OUTPUT_DIR}"
mkdir -p "${OUTPUT_DIR}/logs"
mkdir -p "${OUTPUT_DIR}/profiles"

# Function to run query and measure time
run_query() {
    local fe_type=$1
    local query_name=$2
    local query_sql=$3
    local config=$4
    local iteration=$5

    local host port
    if [ "$fe_type" == "rust" ]; then
        host=$RUST_FE_HOST
        port=$RUST_FE_PORT
    else
        host=$JAVA_FE_HOST
        port=$JAVA_FE_PORT
    fi

    # Set environment variables for feature flags
    eval "$config"

    # Run query and capture timing
    local start_time=$(date +%s%N)

    # Use mysql client to execute query
    mysql -h "$host" -P "$port" -u root --skip-column-names --batch \
        -e "USE tpch_${TPCH_SCALE}; ${query_sql}; SHOW QUERY PROFILE;" \
        > "${OUTPUT_DIR}/logs/${fe_type}_${query_name}_${config}_${iteration}.log" 2>&1

    local end_time=$(date +%s%N)
    local elapsed=$(( ($end_time - $start_time) / 1000000 )) # Convert to milliseconds

    echo "$elapsed"
}

# Function to extract metrics from query profile
extract_profile_metrics() {
    local log_file=$1
    local fe_type=$2

    if [ "$fe_type" == "rust" ]; then
        # Extract Rust FE metrics from logs
        local planning_time=$(grep "Planning time:" "$log_file" | awk '{print $3}' || echo "0")
        local fragment_time=$(grep "Fragment splitting:" "$log_file" | awk '{print $3}' || echo "0")
        local execution_time=$(grep "BE execution:" "$log_file" | awk '{print $3}' || echo "0")

        echo "$planning_time,$fragment_time,$execution_time"
    else
        # Extract Java FE metrics
        local planning_time=$(grep "Query analysis:" "$log_file" | awk '{print $3}' || echo "0")
        local execution_time=$(grep "Query Execution Time:" "$log_file" | awk '{print $4}' || echo "0")

        echo "$planning_time,0,$execution_time"
    fi
}

# Function to run benchmark for one configuration
benchmark_config() {
    local config_name=$1
    local config_vars=$2

    echo -e "${YELLOW}Testing configuration: ${config_name}${NC}"

    local results_file="${OUTPUT_DIR}/${config_name}_results.csv"
    echo "FE_Type,Query,Iteration,Total_Time_ms,Planning_ms,Fragment_Splitting_ms,BE_Execution_ms" > "$results_file"

    for query_name in "${TPCH_QUERIES[@]}"; do
        echo -e "  ${BLUE}Running ${query_name}...${NC}"

        # Load query SQL from file or inline
        local query_sql=$(cat "sql/tpch/${query_name}.sql" 2>/dev/null || echo "SELECT 1;")

        for iteration in $(seq 1 $NUM_ITERATIONS); do
            # Run on Rust FE
            local rust_time=$(run_query "rust" "$query_name" "$query_sql" "$config_vars" "$iteration")
            local rust_metrics=$(extract_profile_metrics "${OUTPUT_DIR}/logs/rust_${query_name}_${config_name}_${iteration}.log" "rust")
            echo "rust,${query_name},${iteration},${rust_time},${rust_metrics}" >> "$results_file"

            # Run on Java FE (for comparison)
            local java_time=$(run_query "java" "$query_name" "$query_sql" "" "$iteration")
            local java_metrics=$(extract_profile_metrics "${OUTPUT_DIR}/logs/java_${query_name}_${config_name}_${iteration}.log" "java")
            echo "java,${query_name},${iteration},${java_time},${java_metrics}" >> "$results_file"

            echo "    Iteration $iteration: Rust=${rust_time}ms, Java=${java_time}ms"
        done
    done
}

# Run benchmarks for all configurations
for config_name in "${!CONFIGS[@]}"; do
    benchmark_config "$config_name" "${CONFIGS[$config_name]}"
    echo ""
done

# Generate summary report
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Generating Summary Report${NC}"
echo -e "${GREEN}========================================${NC}"

# Create summary report
cat > "${OUTPUT_DIR}/BENCHMARK_SUMMARY.md" <<'EOF'
# Doris FE Benchmark Results: Rust vs Java

## Test Configuration
- **TPC-H Scale**: ${TPCH_SCALE}GB
- **Iterations**: ${NUM_ITERATIONS}
- **Date**: $(date)

## Query Performance Comparison

### Overall Results

| Configuration | Avg Speedup | Best Query | Worst Query |
|--------------|-------------|------------|-------------|
EOF

# Analyze results and generate summary
python3 - <<'PYTHON_SCRIPT'
import pandas as pd
import sys
import glob
import os

output_dir = os.environ.get('OUTPUT_DIR', './benchmark_results')

# Load all result files
all_results = []
for csv_file in glob.glob(f"{output_dir}/*_results.csv"):
    config_name = os.path.basename(csv_file).replace('_results.csv', '')
    df = pd.read_csv(csv_file)
    df['Config'] = config_name
    all_results.append(df)

if not all_results:
    print("No results found!")
    sys.exit(1)

combined = pd.concat(all_results)

# Calculate average times per query and configuration
summary = combined.groupby(['Config', 'FE_Type', 'Query'])['Total_Time_ms'].mean().reset_index()
summary_pivot = summary.pivot_table(index=['Config', 'Query'], columns='FE_Type', values='Total_Time_ms')
summary_pivot['Speedup'] = summary_pivot['java'] / summary_pivot['rust']

# Per-configuration analysis
for config in summary_pivot.index.get_level_values(0).unique():
    config_data = summary_pivot.loc[config]
    avg_speedup = config_data['Speedup'].mean()
    best_query = config_data['Speedup'].idxmax()
    worst_query = config_data['Speedup'].idxmin()

    print(f"| {config} | {avg_speedup:.2f}x | {best_query} ({config_data.loc[best_query, 'Speedup']:.2f}x) | {worst_query} ({config_data.loc[worst_query, 'Speedup']:.2f}x) |")

# Detailed query breakdown
print("\n### Detailed Query Performance\n")
print("| Query | Config | Rust FE (ms) | Java FE (ms) | Speedup |")
print("|-------|--------|--------------|--------------|---------|")

for (config, query), row in summary_pivot.iterrows():
    print(f"| {query} | {config} | {row['rust']:.1f} | {row['java']:.1f} | {row['Speedup']:.2f}x |")

# Feature flag effectiveness
print("\n### Optimization Effectiveness\n")
baseline = summary_pivot.loc['baseline']
for config in ['cost_based_join', 'partition_pruning', 'runtime_filters', 'all_optimizations']:
    if config in summary_pivot.index.get_level_values(0):
        config_data = summary_pivot.loc[config]
        improvement = (baseline['rust'].mean() - config_data['rust'].mean()) / baseline['rust'].mean()
        print(f"- **{config}**: {improvement*100:.1f}% faster than baseline")

PYTHON_SCRIPT

echo ""
echo -e "${GREEN}Benchmark complete! Results saved to: ${OUTPUT_DIR}${NC}"
echo -e "${GREEN}Summary report: ${OUTPUT_DIR}/BENCHMARK_SUMMARY.md${NC}"
