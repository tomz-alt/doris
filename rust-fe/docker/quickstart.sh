#!/bin/bash
# Quick start script for Doris Rust FE + Java FE comparison
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "======================================"
echo "Doris Rust FE vs Java FE Quick Start"
echo "======================================"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

step() {
    echo -e "${BLUE}==>${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

warn() {
    echo -e "${YELLOW}!${NC} $1"
}

# Step 1: Build and start containers
step "Building Rust FE Docker image..."
cd "$PROJECT_ROOT"
docker-compose build doris-fe-rust

step "Starting containers (BE + Java FE + Rust FE)..."
docker-compose up -d

echo ""
step "Waiting for services to start..."
sleep 10

# Step 2: Wait for Java FE to be ready
step "Waiting for Java FE to be ready..."
timeout=120
while [ $timeout -gt 0 ]; do
    if docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e "SELECT 1" &>/dev/null; then
        success "Java FE is ready!"
        break
    fi
    sleep 2
    timeout=$((timeout-2))
done

if [ $timeout -le 0 ]; then
    warn "Java FE failed to start in time"
    exit 1
fi

# Step 3: Add BE to Java FE
step "Adding BE to Java FE cluster..."
docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e \
    "ALTER SYSTEM ADD BACKEND 'doris-be:9050';" 2>/dev/null || \
    warn "BE might already be added to Java FE"

sleep 5

# Step 4: Wait for Rust FE to be ready
step "Waiting for Rust FE to be ready..."
timeout=120
while [ $timeout -gt 0 ]; do
    if docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root -e "SELECT 1" &>/dev/null; then
        success "Rust FE is ready!"
        break
    fi
    sleep 2
    timeout=$((timeout-2))
done

if [ $timeout -le 0 ]; then
    warn "Rust FE failed to start in time"
    exit 1
fi

# Step 5: Add BE to Rust FE
step "Adding BE to Rust FE cluster..."
docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root -e \
    "ALTER SYSTEM ADD BACKEND 'doris-be:9050';" 2>/dev/null || \
    warn "BE might already be added to Rust FE"

sleep 5

# Step 6: Verify backends
step "Verifying cluster status..."

echo ""
echo "Java FE Backend Status:"
docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e "SHOW BACKENDS\G"

echo ""
echo "Rust FE Backend Status:"
docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root -e "SHOW BACKENDS\G"

echo ""
success "Cluster is ready!"
echo ""
echo "======================================"
echo "Access Information:"
echo "======================================"
echo ""
echo "Java FE (Baseline):"
echo "  MySQL:  mysql -h 127.0.0.1 -P 9030 -u root"
echo "  HTTP:   http://localhost:8030"
echo ""
echo "Rust FE (Optimized):"
echo "  MySQL:  mysql -h 127.0.0.1 -P 9031 -u root"
echo "  HTTP:   http://localhost:8031"
echo ""
echo "Backend:"
echo "  HTTP:   http://localhost:8040"
echo ""
echo "======================================"
echo "Quick Commands:"
echo "======================================"
echo ""
echo "# Connect to Java FE:"
echo "docker exec -it doris-mysql-client mysql -h doris-fe-java -P 9030 -u root"
echo ""
echo "# Connect to Rust FE:"
echo "docker exec -it doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root"
echo ""
echo "# Run test query comparison:"
echo "./docker/test-query.sh"
echo ""
echo "# View logs:"
echo "docker-compose logs -f doris-fe-rust"
echo "docker-compose logs -f doris-fe-java"
echo ""
echo "# Stop cluster:"
echo "docker-compose down"
echo ""
echo "# Clean up everything (including data):"
echo "docker-compose down -v"
echo ""
