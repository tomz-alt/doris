# Tools

## Test & Build
```bash
cargo test                  # All (67 tests)
cargo test -p fe-catalog    # Catalog only (63)
cargo test edge_case        # Edge cases (15)
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

# Run Java tests
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
