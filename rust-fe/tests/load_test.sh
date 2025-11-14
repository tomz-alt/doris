#!/bin/bash

# Load test script to verify query queuing

set -e

echo "===== Doris Rust FE Load Test ====="
echo ""

HOST="${MYSQL_HOST:-127.0.0.1}"
PORT="${MYSQL_PORT:-9030}"
USER="${MYSQL_USER:-root}"
CONCURRENT="${CONCURRENT:-10}"
QUERIES="${QUERIES:-100}"

echo "Configuration:"
echo "  Host: $HOST:$PORT"
echo "  User: $USER"
echo "  Concurrent connections: $CONCURRENT"
echo "  Queries per connection: $QUERIES"
echo ""

if ! command -v mysql &> /dev/null; then
    echo "Error: mysql client not found"
    exit 1
fi

echo "Starting load test..."
echo ""

run_queries() {
    local conn_id=$1
    local queries=$2

    for i in $(seq 1 $queries); do
        mysql -h $HOST -P $PORT -u $USER -e "SELECT $i AS query_num, 'connection_$conn_id' AS conn" 2>/dev/null > /dev/null
    done

    echo "Connection $conn_id: Completed $queries queries"
}

# Start concurrent connections
START_TIME=$(date +%s)

for i in $(seq 1 $CONCURRENT); do
    run_queries $i $QUERIES &
done

# Wait for all background jobs
wait

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))
TOTAL_QUERIES=$((CONCURRENT * QUERIES))
QPS=$((TOTAL_QUERIES / DURATION))

echo ""
echo "===== Load Test Results ====="
echo "Total queries: $TOTAL_QUERIES"
echo "Duration: ${DURATION}s"
echo "Queries per second: $QPS"
echo ""
echo "✓ Load test complete"
