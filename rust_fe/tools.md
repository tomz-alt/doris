# Tools

## Test & Build
```bash
cargo test                  # All (121 tests)
cargo test -p fe-catalog    # Catalog only (102)
cargo test -p fe-analysis   # SQL parser (11)
cargo test -p fe-qe         # Query executor (4)
cargo test tpch             # TPC-H tests (3)
cargo bench                 # Benchmarks
```

## Quality
```bash
cargo fmt                   # Format
cargo clippy -- -D warnings # Lint
cargo check                 # Fast check
```

## Java Reference (Read-Only)
```bash
# Find implementations
find fe/fe-core/src -name "*.java" | grep -i catalog

# Run Java tests to verify behaviors
cd fe/fe-core && mvn test -Dtest=DatabaseTest

# View results
less target/surefire-reports/*.txt
```

## Analysis
```bash
cargo tree                  # Deps
cargo bloat --release       # Size
cargo expand                # Macros
```

## Performance
```bash
cargo flamegraph            # Profile
hyperfine 'cargo run'       # Benchmark
```
