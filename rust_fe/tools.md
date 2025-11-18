# Tools

## Test & Build
```bash
cargo test                        # All (200 tests)
cargo test -p fe-catalog          # Catalog only (102)
cargo test -p fe-analysis         # SQL parser (11)
cargo test -p fe-qe               # Query executor (40)
cargo test -p fe-planner          # Query planner (9)
cargo test -p fe-mysql-protocol   # MySQL protocol (28)
cargo test -p fe-backend-client   # BE client (6)
cargo test comparison_tests       # Rust vs Java FE comparison (34)
cargo test tpch                   # TPC-H tests
cargo bench                       # Benchmarks
```

## Quality
```bash
cargo fmt                   # Format
cargo clippy -- -D warnings # Lint
cargo check                 # Fast check
```

## Java Reference (Read-Only - DO NOT MODIFY)
```bash
# Find implementations
find fe/fe-core/src -name "*.java" | grep -i catalog

# Run Java tests to verify behaviors match
cd fe/fe-core && mvn test -Dtest=DatabaseTest

# View results
less target/surefire-reports/*.txt

# Save Java FE query results for comparison
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT ..." > /tmp/java_fe_results.txt
```

## Analysis
```bash
cargo tree                  # Deps
cargo bloat --release       # Size
cargo expand                # Macros
```

## C++ BE Integration (Blocked - protoc needed)
```bash
# Install protoc (requires root)
sudo apt-get install protobuf-compiler

# Build with protobuf generation
cargo build -p fe-backend-client

# Test with MockBackend (works without protoc)
cargo test -p fe-backend-client
```

## MySQL Server
```bash
# Start Rust FE
cargo run --bin doris-fe

# Connect with MySQL CLI
mysql -h 127.0.0.1 -P 9030 -u root

# Execute TPC-H Q1
mysql -h 127.0.0.1 -P 9030 -u root tpch < query1.sql
```

## Performance
```bash
cargo flamegraph            # Profile
hyperfine 'cargo run'       # Benchmark
```
