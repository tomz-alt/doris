# Tools & Commands

## Build & Test
```bash
cargo test                    # All tests (52)
cargo test -p fe-catalog      # Catalog tests only
cargo test --quiet            # Less output
cargo bench                   # Benchmarks
```

## Code Quality
```bash
cargo fmt                     # Format
cargo clippy -- -D warnings   # Lint strict
cargo check                   # Fast check
```

## Java FE Reference (Read-Only)
```bash
# Find Java implementations
find fe/fe-core/src -name "Database.java"
grep -r "registerTable" fe/fe-core/src/test

# Run Java tests to verify behavior
cd fe/fe-core
mvn test -Dtest=DatabaseTest
mvn test -Dtest=ColumnTest

# View test results
less target/surefire-reports/*.txt
```

## Analysis
```bash
cargo tree                    # Dependencies
cargo bloat --release        # Binary size
cargo expand                  # Macro expansion
```

## Performance
```bash
cargo flamegraph             # CPU profile
hyperfine 'cargo run'        # Benchmark
valgrind --tool=massif       # Memory profile
```
