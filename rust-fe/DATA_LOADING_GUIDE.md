# Data Loading Guide: Doris Tools vs Direct Stream Load

## Current Implementation vs Doris Tools Repo

### What We Currently Use ❌

Our scripts use **direct Stream Load API** via curl:

```bash
# Current approach in generate_data.sh
curl --location-trusted \
    -u root: \
    -H "column_separator:|" \
    -H "columns: $(get_columns $table)" \
    -T "$file" \
    "http://${FE_HOST}:${http_port}/api/${DATABASE}/${table}/_stream_load"
```

**Problems with this approach:**
- ❌ Not using official Doris tools repo scripts
- ❌ No doris-cluster.conf configuration
- ❌ Manual curl commands instead of wrapper scripts
- ❌ Doesn't match official Doris benchmark methodology

### What Doris Tools Repo Uses ✅

The official Apache Doris tools repository approach:

```bash
# Official Doris tools/tpch-tools approach
cd $DORIS_HOME/tools/tpch-tools

# 1. Configure cluster
vim conf/doris-cluster.conf

# 2. Build dbgen
./bin/build-tpch-dbgen.sh

# 3. Generate data
./bin/gen-tpch-data.sh -s 1

# 4. Create tables
./bin/create-tpch-tables.sh -s 1

# 5. Load data (uses doris-cluster.conf)
./bin/load-tpch-data.sh

# 6. Run queries
./bin/run-tpch-queries.sh -s 1
```

**Benefits of this approach:**
- ✅ Official Doris methodology
- ✅ Uses doris-cluster.conf for configuration
- ✅ Wrapper scripts handle errors
- ✅ Matches published benchmark results
- ✅ Battle-tested by Doris team

## Recommended Approach

### Option 1: Use Official Doris Tools Repo (Recommended) ✅

Clone the Doris repository and use the official tools:

```bash
# 1. Clone Doris repo
git clone https://github.com/apache/doris.git
cd doris/tools/tpch-tools

# 2. Configure cluster connection
cat > conf/doris-cluster.conf <<EOF
# FE HTTP connection
export FE_HOST='127.0.0.1'
export FE_HTTP_PORT='8030'
export FE_QUERY_PORT='9030'
export USER='root'
export PASSWORD=''
export DB='tpch_sf1'
EOF

# 3. Build and generate data
./bin/build-tpch-dbgen.sh
./bin/gen-tpch-data.sh -s 1

# 4. Create tables and load data
./bin/create-tpch-tables.sh -s 1
./bin/load-tpch-data.sh

# 5. Verify
mysql -h 127.0.0.1 -P 9030 -u root tpch_sf1 -e "SELECT COUNT(*) FROM lineitem"
```

### Option 2: Use Our Standalone Scripts (Quick Start) ⚡

For quick testing without cloning Doris repo:

```bash
# Uses our generate_data.sh (current implementation)
./scripts/tpch/generate_data.sh --scale 1 --port 9030
```

**Use this when:**
- Quick local testing
- Don't want to clone full Doris repo
- Single FE endpoint
- Simple setup

## Update Our Scripts to Use Tools Repo Approach

### New Wrapper Script: `load_with_tools.sh`

```bash
#!/bin/bash
# Wrapper to use official Doris tools for data loading

DORIS_REPO=${DORIS_REPO:-"/tmp/doris"}
BENCHMARK=${1:-"tpch"}  # tpch or tpcds
SCALE_FACTOR=${2:-1}
FE_HOST=${3:-"127.0.0.1"}
FE_HTTP_PORT=${4:-8030}
FE_QUERY_PORT=${5:-9030}

echo "Using official Doris tools repo approach..."

# 1. Clone Doris if needed
if [ ! -d "$DORIS_REPO" ]; then
    echo "Cloning Doris repository..."
    git clone --depth 1 https://github.com/apache/doris.git "$DORIS_REPO"
fi

# 2. Navigate to tools
if [ "$BENCHMARK" = "tpch" ]; then
    cd "$DORIS_REPO/tools/tpch-tools"
else
    cd "$DORIS_REPO/tools/tpcds-tools"
fi

# 3. Configure cluster
cat > conf/doris-cluster.conf <<EOF
export FE_HOST='${FE_HOST}'
export FE_HTTP_PORT='${FE_HTTP_PORT}'
export FE_QUERY_PORT='${FE_QUERY_PORT}'
export USER='root'
export PASSWORD=''
export DB='${BENCHMARK}_sf${SCALE_FACTOR}'
EOF

# 4. Build and run
./bin/build-${BENCHMARK}-dbgen.sh
./bin/gen-${BENCHMARK}-data.sh -s ${SCALE_FACTOR}
./bin/create-${BENCHMARK}-tables.sh -s ${SCALE_FACTOR}
./bin/load-${BENCHMARK}-data.sh

echo "✓ Data loaded using official Doris tools!"
```

## Comparison Table

| Feature | Our Scripts | Doris Tools Repo |
|---------|-------------|------------------|
| **Installation** | Standalone | Requires Doris clone |
| **Configuration** | Command-line args | doris-cluster.conf |
| **Data Loading** | Direct curl Stream Load | Wrapper scripts |
| **Error Handling** | Basic | Comprehensive |
| **Doris Version Compatibility** | Generic | Version-specific |
| **Official Methodology** | ❌ No | ✅ Yes |
| **Benchmark Compliance** | Partial | Full |
| **Best For** | Quick testing | Official benchmarks |

## Stream Load API Details

Both approaches ultimately use **Stream Load API**, but differently:

### Direct Curl (Our Current Approach)

```bash
# Load lineitem table
curl --location-trusted \
    -u root: \
    -H "column_separator:|" \
    -H "columns:l_orderkey,l_partkey,l_suppkey,..." \
    -T lineitem.tbl \
    "http://127.0.0.1:8030/api/tpch_sf1/lineitem/_stream_load"
```

### Doris Tools Wrapper (Official Approach)

```bash
# load-tpch-data.sh wraps Stream Load with:
# - Automatic column mapping
# - Error retry logic
# - Progress tracking
# - Parallel loading
# - Transaction management
```

## Recommendation for Production Benchmarks

**For comparing Rust FE vs Java FE:**

1. **Use official Doris tools** for data loading
2. **Use our scripts** for benchmark execution and visualization
3. **Ensure both FEs load from same source**

### Complete Workflow

```bash
# 1. Load data using official Doris tools (Java FE)
cd doris/tools/tpch-tools
# Edit conf/doris-cluster.conf to point to Java FE (port 9030)
./bin/load-tpch-data.sh

# 2. Run benchmark comparing both FEs
cd /path/to/rust-fe
python3 scripts/benchmark_tpch.py \
    --java-port 9030 \
    --rust-port 9031 \
    --scale 1 \
    --rounds 5

# 3. View ClickBench-style results
open tpch_results.html
```

## MySQL INSERT Alternative

**NOT recommended** for TPC-H/TPC-DS due to performance:

```sql
-- This would be VERY slow for millions of rows
LOAD DATA LOCAL INFILE 'lineitem.tbl'
INTO TABLE lineitem
FIELDS TERMINATED BY '|'
LINES TERMINATED BY '\n';
```

**Stream Load is ~10-100x faster** than INSERT for bulk loading.

## Action Items

To fully align with Doris tools repo:

- [ ] Update `generate_data.sh` to optionally use tools repo
- [ ] Add `doris-cluster.conf` template
- [ ] Create wrapper script for tools repo approach
- [ ] Document both approaches in README
- [ ] Add benchmarking methodology guide

## References

- [Doris TPC-H Tools](https://github.com/apache/doris/tree/master/tools/tpch-tools)
- [Doris TPC-DS Tools](https://github.com/apache/doris/tree/master/tools/tpcds-tools)
- [Stream Load Documentation](https://doris.apache.org/docs/data-operate/import/stream-load-manual)
- [TPC-H Benchmark Documentation](https://doris.apache.org/docs/benchmark/tpch/)

## Summary

**Current State:**
- ✅ Scripts work but use direct Stream Load curl
- ❌ Not using official Doris tools repo methodology

**Recommendation:**
- 🔄 **For benchmarking: Use official Doris tools for data loading**
- ✅ **For visualization: Use our ClickBench-style scripts**

This ensures benchmark results match official Doris methodology while providing beautiful visualization!
