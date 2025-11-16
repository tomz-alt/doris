#!/bin/bash

# Verification script for MySQL protocol

set -e

echo "===== Doris Rust FE MySQL Protocol Verification ====="
echo ""

# Check if mysql client is available
if ! command -v mysql &> /dev/null; then
    echo "Error: mysql client not found. Please install it first."
    echo "On Ubuntu/Debian: sudo apt-get install mysql-client"
    echo "On macOS: brew install mysql-client"
    exit 1
fi

HOST="${MYSQL_HOST:-127.0.0.1}"
PORT="${MYSQL_PORT:-9030}"
USER="${MYSQL_USER:-root}"

echo "Connecting to MySQL server at $HOST:$PORT as user $USER"
echo ""

# Test 1: Connection test
echo "Test 1: Connection test"
mysql -h $HOST -P $PORT -u $USER -e "SELECT 1" 2>/dev/null && echo "✓ Connection successful" || echo "✗ Connection failed"
echo ""

# Test 2: Version check
echo "Test 2: Version check"
mysql -h $HOST -P $PORT -u $USER -e "SELECT VERSION()" 2>/dev/null && echo "✓ Version query successful" || echo "✗ Version query failed"
echo ""

# Test 3: Show databases
echo "Test 3: Show databases"
mysql -h $HOST -P $PORT -u $USER -e "SHOW DATABASES" 2>/dev/null && echo "✓ Show databases successful" || echo "✗ Show databases failed"
echo ""

# Test 4: Use TPC-H database (Rust FE default catalog)
echo "Test 4: Use database"
mysql -h $HOST -P $PORT -u $USER -e "USE tpch" 2>/dev/null && echo "✓ Use database successful" || echo "✗ Use database failed"
echo ""

# Test 5: Show tables in TPC-H
echo "Test 5: Show tables"
mysql -h $HOST -P $PORT -u $USER -D tpch -e "SHOW TABLES" 2>/dev/null && echo "✓ Show tables successful" || echo "✗ Show tables failed"
echo ""

# Test 6: Simple SELECT query (no table dependency)
echo "Test 6: Select query"
mysql -h $HOST -P $PORT -u $USER -e "SELECT 1" 2>/dev/null && echo "✓ Select query successful" || echo "✗ Select query failed"
echo ""

# Test 7: Ping
echo "Test 7: Ping test"
mysqladmin -h $HOST -P $PORT -u $USER ping 2>/dev/null && echo "✓ Ping successful" || echo "✗ Ping failed"
echo ""

echo "===== MySQL Protocol Verification Complete ====="
