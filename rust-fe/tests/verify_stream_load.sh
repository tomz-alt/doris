#!/bin/bash

# Verification script for Stream Load functionality

set -e

echo "===== Doris Rust FE Stream Load Verification ====="
echo ""

HOST="${HTTP_HOST:-127.0.0.1}"
PORT="${HTTP_PORT:-8030}"
DB="test"
TABLE="products"

echo "Testing stream load at http://$HOST:$PORT"
echo ""

# Test 1: Health check
echo "Test 1: Health check"
curl -s "http://$HOST:$PORT/api/health" | python3 -m json.tool && echo "✓ Health check successful" || echo "✗ Health check failed"
echo ""

# Test 2: Status check
echo "Test 2: Status check"
curl -s "http://$HOST:$PORT/api/status" | python3 -m json.tool && echo "✓ Status check successful" || echo "✗ Status check failed"
echo ""

# Test 3: Stream load with test data
echo "Test 3: Stream load"
RESPONSE=$(curl -s -w "\n%{http_code}" -X PUT \
    -H "label: test_load_$(date +%s)" \
    -H "format: csv" \
    -H "column_separator: ," \
    --data-binary @tests/test_data.csv \
    "http://$HOST:$PORT/api/$DB/$TABLE/_stream_load")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | head -n-1)

echo "HTTP Status: $HTTP_CODE"
echo "Response:"
echo "$BODY" | python3 -m json.tool

if [ "$HTTP_CODE" = "200" ]; then
    echo "✓ Stream load successful"
else
    echo "✗ Stream load failed"
fi
echo ""

# Test 4: Stream load with custom data
echo "Test 4: Stream load with inline data"
echo -e "id,name,value\n100,test_item,999.99" | curl -s -X PUT \
    -H "label: test_load_inline_$(date +%s)" \
    -H "format: csv" \
    -H "column_separator: ," \
    --data-binary @- \
    "http://$HOST:$PORT/api/$DB/$TABLE/_stream_load" | python3 -m json.tool && echo "✓ Inline stream load successful" || echo "✗ Inline stream load failed"
echo ""

echo "===== Stream Load Verification Complete ====="
