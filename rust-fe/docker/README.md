# Docker Deployment - Rust FE vs Java FE Comparison

Quick start guide for running and comparing Rust FE against Java FE using Docker.

---

## Quick Start (5 Minutes)

```bash
# 1. Start the cluster (Rust FE + Java FE + 1 shared BE)
./docker/quickstart.sh

# 2. Connect to either FE
docker exec -it doris-mysql-client mysql -h doris-fe-java -P 9030 -u root   # Java FE
docker exec -it doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root   # Rust FE

# 3. Run test queries
./docker/test-query.sh
```

---

## Architecture

```
┌─────────────────┐         ┌─────────────────┐
│   Java FE       │         │   Rust FE       │
│  (Baseline)     │         │  (Optimized)    │
│   Port: 9030    │         │   Port: 9031    │
└────────┬────────┘         └────────┬────────┘
         │                           │
         └───────────┬───────────────┘
                     │
              ┌──────▼──────┐
              │             │
              │  Shared BE  │
              │             │
              │ Port: 9060  │
              └─────────────┘
```

**Benefits**:
- Single BE node - saves resources
- Direct comparison - same data, same hardware
- Easy testing - switch between FEs by changing port

---

## Services

| Service | Port | Purpose |
|---------|------|---------|
| **Java FE** | 9030 (MySQL) | Baseline performance |
| | 8030 (HTTP) | Web UI |
| **Rust FE** | 9031 (MySQL) | Optimized performance |
| | 8031 (HTTP) | Web UI |
| **BE** | 9060 (Service) | Shared backend |
| | 8040 (HTTP) | BE Web UI |

---

## Usage Examples

### 1. Basic Connectivity Test

```bash
# Test Java FE
docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e "SELECT 1"

# Test Rust FE
docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root -e "SELECT 1"
```

### 2. Check Cluster Status

```bash
# Java FE backend status
docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e "SHOW BACKENDS\G"

# Rust FE backend status
docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root -e "SHOW BACKENDS\G"
```

### 3. Create Test Database

```bash
# On Java FE
docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e "CREATE DATABASE test_db"

# On Rust FE
docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root -e "CREATE DATABASE test_db"
```

### 4. Run Simple Query Comparison

```bash
# Create test table on both FEs
for port in 9030 9031; do
  docker exec doris-mysql-client mysql -h doris-fe-$([ $port -eq 9030 ] && echo "java" || echo "rust") -P $port -u root <<'EOF'
CREATE DATABASE IF NOT EXISTS test;
USE test;
CREATE TABLE IF NOT EXISTS user (
    id INT,
    name VARCHAR(50),
    age INT
) DUPLICATE KEY(id)
DISTRIBUTED BY HASH(id) BUCKETS 1;

INSERT INTO user VALUES (1, 'Alice', 25), (2, 'Bob', 30), (3, 'Charlie', 35);
EOF
done

# Query on Java FE
time docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root test -e "SELECT * FROM user WHERE age > 25"

# Query on Rust FE (with optimizations)
time docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root test -e "SELECT * FROM user WHERE age > 25"
```

### 5. Compare Query Profiles

```bash
# Run query and get profile from Java FE
docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root test <<'EOF'
SELECT COUNT(*) FROM user WHERE age > 25;
SHOW QUERY PROFILE;
EOF

# Run same query and get profile from Rust FE
docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root test <<'EOF'
SELECT COUNT(*) FROM user WHERE age > 25;
SHOW QUERY PROFILE;
EOF
```

---

## Optimization Configuration

The Rust FE has optimizations **enabled by default**. You can toggle them via environment variables:

### Enable/Disable Optimizations

Edit `docker-compose.yml`:

```yaml
doris-fe-rust:
  environment:
    - ENABLE_COST_BASED_JOIN=true    # Cost-based join strategy
    - ENABLE_PARTITION_PRUNING=true  # Advanced partition pruning
    - ENABLE_RUNTIME_FILTERS=true    # Runtime filter propagation
    - ENABLE_BUCKET_SHUFFLE=true     # Bucket shuffle optimization
```

Then restart:

```bash
docker-compose up -d doris-fe-rust
```

### Test Different Configurations

```bash
# Baseline (all optimizations OFF)
docker-compose stop doris-fe-rust
docker-compose run -e ENABLE_COST_BASED_JOIN=false \
                    -e ENABLE_PARTITION_PRUNING=false \
                    -e ENABLE_RUNTIME_FILTERS=false \
                    -e ENABLE_BUCKET_SHUFFLE=false \
                    doris-fe-rust

# Only cost-based joins
docker-compose run -e ENABLE_COST_BASED_JOIN=true \
                    -e ENABLE_PARTITION_PRUNING=false \
                    doris-fe-rust

# All optimizations (default)
docker-compose up -d doris-fe-rust
```

---

## Loading Test Data

### Option 1: Small Test Dataset (Quick)

```bash
./docker/test-query.sh  # Will prompt to create test tables
```

### Option 2: TPC-H SF1 (1GB - Realistic)

```bash
# Generate TPC-H data (on host machine)
git clone https://github.com/electrum/tpch-dbgen.git
cd tpch-dbgen
make
./dbgen -s 1  # Generates ~1GB of data

# Create tables
docker exec -i doris-mysql-client mysql -h doris-fe-java -P 9030 -u root < ../scripts/tpch/create_tables_sf1.sql
docker exec -i doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root < ../scripts/tpch/create_tables_sf1.sql

# Load data (example for customer table)
curl --location-trusted -u root: \
  -H "column_separator:|" \
  -T customer.tbl \
  http://localhost:8030/api/tpch_1/customer/_stream_load
```

---

## Troubleshooting

### Problem: Containers won't start

```bash
# Check status
docker-compose ps

# View logs
docker-compose logs doris-fe-rust
docker-compose logs doris-fe-java
docker-compose logs doris-be

# Restart everything
docker-compose restart
```

### Problem: BE not showing as alive

```bash
# Check BE status
docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root -e "SHOW BACKENDS\G"

# If Alive=false, wait a minute - BE takes time to start
# Or check BE logs:
docker-compose logs doris-be
```

### Problem: Out of memory

```bash
# Increase Docker memory limit to at least 8GB
# Docker Desktop -> Settings -> Resources -> Memory

# Or reduce FE memory in docker-compose.yml:
#   environment:
#     - JAVA_OPTS=-Xmx2048m  # For Java FE
```

### Problem: Can't connect to MySQL

```bash
# Check if FE is healthy
docker exec doris-fe-rust /opt/doris-rust-fe/bin/doris-rust-fe --version

# Check ports are exposed
docker ps | grep doris-fe

# Try connecting from within the network
docker exec -it doris-mysql-client bash
mysql -h doris-fe-rust -P 9031 -u root
```

---

## Performance Benchmarking

### Quick Performance Test

```bash
# Run automated comparison
./docker/test-query.sh
```

### Manual Benchmark

```bash
# Test query on Java FE (10 iterations)
for i in {1..10}; do
  time docker exec doris-mysql-client mysql -h doris-fe-java -P 9030 -u root test \
    -e "SELECT COUNT(*) FROM large_table WHERE date > '2024-01-01'"
done

# Test same query on Rust FE
for i in {1..10}; do
  time docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root test \
    -e "SELECT COUNT(*) FROM large_table WHERE date > '2024-01-01'"
done
```

### Check Optimization Application

```bash
# View Rust FE logs for optimization messages
docker-compose logs doris-fe-rust | grep -E "Cost-based|Partition|Runtime filter|Bucket shuffle"

# Check query profile
docker exec doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root test <<'EOF'
SELECT * FROM orders WHERE o_orderdate > '2024-01-01';
SHOW QUERY PROFILE;
EOF
```

---

## Cleanup

```bash
# Stop containers (keeps data)
docker-compose stop

# Stop and remove containers (keeps data)
docker-compose down

# Remove everything including data volumes
docker-compose down -v

# Remove images
docker rmi $(docker images | grep doris-fe-rust | awk '{print $3}')
```

---

## Next Steps

1. **Load real data**: Use TPC-H SF1 or your own dataset
2. **Run benchmarks**: Compare query performance between FEs
3. **Analyze profiles**: Check query profiles for optimization evidence
4. **Tune configuration**: Adjust optimization thresholds based on workload
5. **Scale testing**: Add more BE nodes for larger datasets

---

## Accessing Web UIs

- **Java FE UI**: http://localhost:8030
- **Rust FE UI**: http://localhost:8031 (if implemented)
- **BE UI**: http://localhost:8040

---

## Useful Commands Reference

```bash
# Start cluster
./docker/quickstart.sh

# Connect to Java FE
docker exec -it doris-mysql-client mysql -h doris-fe-java -P 9030 -u root

# Connect to Rust FE
docker exec -it doris-mysql-client mysql -h doris-fe-rust -P 9031 -u root

# View Rust FE logs
docker-compose logs -f doris-fe-rust

# View BE logs
docker-compose logs -f doris-be

# Restart Rust FE only
docker-compose restart doris-fe-rust

# Run test queries
./docker/test-query.sh

# Shell into container
docker exec -it doris-fe-rust bash

# Check disk usage
docker system df

# Clean up unused containers/images
docker system prune
```

---

## Support

For issues or questions:
- Check logs: `docker-compose logs <service-name>`
- Review Doris documentation: https://doris.apache.org/docs
- Check Rust FE implementation docs in the repository

Happy benchmarking! 🚀
