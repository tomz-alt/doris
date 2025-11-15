#!/bin/bash
# Test query script - compares Rust FE vs Java FE performance
set -e

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "======================================"
echo "Rust FE vs Java FE Query Comparison"
echo "======================================"
echo ""

# Sample TPC-H style queries
TEST_QUERIES=(
    "Simple scan:SELECT COUNT(*) FROM lineitem WHERE l_quantity > 45"
    "Aggregation:SELECT l_returnflag, COUNT(*), SUM(l_quantity) FROM lineitem GROUP BY l_returnflag"
    "Join:SELECT COUNT(*) FROM orders o JOIN customer c ON o.o_custkey = c.c_custkey WHERE c.c_mktsegment = 'BUILDING'"
)

# Function to run query and measure time
run_query() {
    local fe_name=$1
    local fe_host=$2
    local fe_port=$3
    local query=$4

    local start=$(date +%s%N)
    docker exec doris-mysql-client mysql -h "$fe_host" -P "$fe_port" -u root -e "$query" &>/dev/null
    local end=$(date +%s%N)

    local elapsed_ms=$(( ($end - $start) / 1000000 ))
    echo "$elapsed_ms"
}

# Check if test database exists
echo "Checking for test database..."
if ! docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e "USE tpch_1" &>/dev/null; then
    echo ""
    echo -e "${YELLOW}No TPC-H database found.${NC}"
    echo "Would you like to create a small test dataset? (y/n)"
    read -r response

    if [[ "$response" =~ ^[Yy]$ ]]; then
        echo "Creating test database and tables..."

        # Create database on both FEs
        docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e "CREATE DATABASE IF NOT EXISTS tpch_1"
        docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root -e "CREATE DATABASE IF NOT EXISTS tpch_1"

        # Create simple tables
        cat <<'EOF' | docker exec -i doris-mysql-client mysql -h doris-fe-java -P 9030 -u root tpch_1
CREATE TABLE IF NOT EXISTS customer (
    c_custkey INT NOT NULL,
    c_name VARCHAR(25) NOT NULL,
    c_mktsegment CHAR(10) NOT NULL
) DUPLICATE KEY(c_custkey)
DISTRIBUTED BY HASH(c_custkey) BUCKETS 1;

CREATE TABLE IF NOT EXISTS orders (
    o_orderkey INT NOT NULL,
    o_custkey INT NOT NULL,
    o_orderdate DATE NOT NULL
) DUPLICATE KEY(o_orderkey)
DISTRIBUTED BY HASH(o_orderkey) BUCKETS 1;

CREATE TABLE IF NOT EXISTS lineitem (
    l_orderkey INT NOT NULL,
    l_quantity DECIMAL(15,2) NOT NULL,
    l_returnflag CHAR(1) NOT NULL,
    l_shipdate DATE NOT NULL
) DUPLICATE KEY(l_orderkey)
DISTRIBUTED BY HASH(l_orderkey) BUCKETS 1;
EOF

        # Same for Rust FE
        cat <<'EOF' | docker exec -i doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root tpch_1
CREATE TABLE IF NOT EXISTS customer (
    c_custkey INT NOT NULL,
    c_name VARCHAR(25) NOT NULL,
    c_mktsegment CHAR(10) NOT NULL
) DUPLICATE KEY(c_custkey)
DISTRIBUTED BY HASH(c_custkey) BUCKETS 1;

CREATE TABLE IF NOT EXISTS orders (
    o_orderkey INT NOT NULL,
    o_custkey INT NOT NULL,
    o_orderdate DATE NOT NULL
) DUPLICATE KEY(o_orderkey)
DISTRIBUTED BY HASH(o_orderkey) BUCKETS 1;

CREATE TABLE IF NOT EXISTS lineitem (
    l_orderkey INT NOT NULL,
    l_quantity DECIMAL(15,2) NOT NULL,
    l_returnflag CHAR(1) NOT NULL,
    l_shipdate DATE NOT NULL
) DUPLICATE KEY(l_orderkey)
DISTRIBUTED BY HASH(l_orderkey) BUCKETS 1;
EOF

        echo -e "${GREEN}✓${NC} Test tables created"
        echo ""
        echo "You can now load test data with:"
        echo "  ./docker/load-test-data.sh"
        echo ""
        exit 0
    else
        echo "Skipping tests - no data available"
        exit 0
    fi
fi

echo ""
echo "Running query comparison..."
echo "======================================"

# Run tests
for query_info in "${TEST_QUERIES[@]}"; do
    query_name=$(echo "$query_info" | cut -d: -f1)
    query_sql=$(echo "$query_info" | cut -d: -f2)

    echo ""
    echo "Test: $query_name"
    echo "Query: ${query_sql:0:60}..."

    # Run on Java FE
    echo -n "  Java FE:  "
    java_time=$(run_query "Java FE" "doris-fe-java" 9030 "$query_sql")
    echo "${java_time} ms"

    # Run on Rust FE
    echo -n "  Rust FE:  "
    rust_time=$(run_query "Rust FE" "doris-fe-rust" 9031 "$query_sql")
    echo "${rust_time} ms"

    # Calculate speedup
    if [ "$java_time" -gt 0 ] && [ "$rust_time" -gt 0 ]; then
        speedup=$(awk "BEGIN {printf \"%.2f\", $java_time/$rust_time}")
        if (( $(awk "BEGIN {print ($speedup > 1.0)}") )); then
            echo -e "  ${GREEN}Speedup:  ${speedup}x faster${NC}"
        elif (( $(awk "BEGIN {print ($speedup < 0.95)}") )); then
            echo -e "  ${YELLOW}Speedup:  ${speedup}x (slower)${NC}"
        else
            echo "  Speedup:  ${speedup}x (similar)"
        fi
    fi
done

echo ""
echo "======================================"
echo "Query Profile Comparison"
echo "======================================"
echo ""
echo "Java FE optimization info:"
docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e "SHOW QUERY PROFILE\G" | grep -E "FragmentInstanceId|ExchangeNode|HashJoinNode" | head -20

echo ""
echo "Rust FE optimization info:"
docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root -e "SHOW QUERY PROFILE\G" | grep -E "FragmentInstanceId|ExchangeNode|HashJoinNode|RuntimeFilter|PartitionPruning" | head -20

echo ""
echo "======================================"
echo -e "${GREEN}Comparison complete!${NC}"
echo "======================================"
echo ""
