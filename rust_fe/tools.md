# Useful Tools & Commands

## Build & Test
```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Run all tests
cargo test -p fe-catalog       # Test specific crate
cargo bench                    # Run benchmarks
```

## Code Quality
```bash
cargo fmt                      # Format code
cargo clippy                   # Lint code
cargo clippy -- -D warnings    # Lint with errors
cargo check                    # Fast compilation check
```

## Analysis
```bash
cargo tree                     # Dependency tree
cargo bloat --release         # Binary size analysis
cargo expand                   # Macro expansion
```

## Java FE Reference
```bash
# Find Java implementations
find fe/fe-core/src -name "Catalog.java"
find fe/fe-core/src -name "Database.java"

# Search for specific behavior
grep -r "createDatabase" fe/fe-core/src
grep -r "transaction" fe/fe-core/src

# Count lines/files per module
find fe/fe-core/src/main/java/org/apache/doris/catalog -name "*.java" | wc -l
```

## Testing Java FE (Read-only)
```bash
# Run Java FE tests to understand behavior
cd fe/fe-core
mvn test -Dtest=CatalogTest
mvn test -Dtest=DatabaseTest

# View test output
less target/surefire-reports/*.txt
```

## Performance Tools
```bash
cargo flamegraph             # CPU profiling
cargo instruments            # macOS profiling
valgrind --tool=massif       # Memory profiling
hyperfine 'cargo run'        # Benchmarking
```
