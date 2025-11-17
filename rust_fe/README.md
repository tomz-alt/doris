# Doris Rust Frontend

This is the Rust implementation of Apache Doris Frontend (FE), migrating from the original Java implementation.

## Overview

The Rust FE aims to provide:
- **Better Performance**: Lower latency and higher throughput
- **Lower Memory Usage**: Efficient memory management with Rust's ownership model
- **Type Safety**: Compile-time guarantees to prevent common bugs
- **Modern Architecture**: Leveraging async/await and modern Rust ecosystem

## Project Structure

```
rust_fe/
├── fe-common/           # Common utilities, types, and error handling
├── fe-catalog/          # Catalog management (databases, tables, columns)
├── fe-analysis/         # SQL parsing and semantic analysis
├── fe-nereids/          # New query optimizer (Nereids)
├── fe-planner/          # Query planning
├── fe-qe/               # Query execution engine
├── fe-service/          # Main service implementations
├── fe-rpc/              # RPC client/server
├── fe-persist/          # Metadata persistence
├── fe-system/           # System management (BE/FE nodes)
├── fe-statistics/       # Statistics collection and management
├── fe-load/             # Data loading
├── fe-transaction/      # Transaction management
├── fe-mysql-protocol/   # MySQL protocol implementation
├── fe-http/             # HTTP API server
├── fe-alter/            # Schema change operations
├── fe-datasource/       # External datasource support
├── fe-ha/               # High availability
├── fe-metric/           # Metrics collection
├── fe-job/              # Job scheduling and management
└── fe-main/             # Main entry point
```

## Building

### Prerequisites

- Rust 1.75 or later
- Git

### Build

```bash
# Build in debug mode
cargo build

# Build in release mode (optimized)
cargo build --release

# Build only fe-main
cargo build --bin doris-fe --release
```

### Test

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p fe-common

# Run tests with output
cargo test -- --nocapture
```

## Running

```bash
# Run in development mode
cargo run --bin doris-fe

# Run with custom config
cargo run --bin doris-fe -- --config /path/to/fe.conf

# Run in release mode
cargo run --release --bin doris-fe
```

## Configuration

Create a `fe.conf` file in TOML format:

```toml
meta_dir = "./doris-meta"
http_port = 8030
rpc_port = 9020
query_port = 9030
edit_log_port = 9010
role = "FOLLOWER"
node_name = "fe_rust_node_1"
enable_election = true
log_level = "info"
log_dir = "./log"
```

## Development Status

See [rust_fe_migration_todos.md](../rust_fe_migration_todos.md) for the complete migration roadmap.

### Completed Modules
- ✅ **fe-common**: Foundational types, error handling, configuration
- ✅ **fe-catalog**: Basic catalog management (Database, Table, Column, Partition, Index, Replica)
- ✅ **fe-main**: Entry point and server skeleton

### In Progress
- 🚧 Complete fe-catalog functionality
- 🚧 SQL parser and analysis module
- 🚧 Query optimizer (Nereids)

### TODO
- ⏳ Query planner
- ⏳ Query execution engine
- ⏳ All services (HTTP, RPC, MySQL)
- ⏳ Data loading
- ⏳ Transaction management
- ⏳ And much more...

## Architecture

### Async Runtime
The Rust FE uses **Tokio** as the async runtime for all I/O operations.

### Concurrency
- `Arc<RwLock<T>>` for shared mutable state
- `DashMap` for concurrent hash maps
- `parking_lot` for efficient synchronization primitives

### Error Handling
All operations return `Result<T, DorisError>` for explicit error handling.

### Logging
Using `tracing` for structured logging with configurable levels.

## Performance Goals

| Metric | Java FE | Rust FE Target | Status |
|--------|---------|----------------|--------|
| Memory Usage | Baseline | -40-50% | 🎯 |
| Query Latency | Baseline | -20-30% | 🎯 |
| Throughput | Baseline | +20-30% | 🎯 |
| Startup Time | Baseline | -50% | 🎯 |

## Contributing

See the main [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.

### Code Style

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Check without building
cargo check
```

## License

Apache License 2.0

## Roadmap

### Phase 1 (Q1 2025) - Foundation
- [x] Project setup
- [x] Common utilities
- [x] Basic catalog
- [ ] Complete catalog implementation
- [ ] SQL parser

### Phase 2 (Q2 2025) - Core Query Engine
- [ ] Query analysis
- [ ] Nereids optimizer
- [ ] Query planner
- [ ] Basic execution engine

### Phase 3 (Q3 2025) - Services
- [ ] MySQL protocol
- [ ] HTTP API
- [ ] RPC services
- [ ] Metadata persistence

### Phase 4 (Q4 2025) - Advanced Features
- [ ] Data loading
- [ ] Transaction management
- [ ] High availability
- [ ] External data sources

### Phase 5 (2026) - Production Ready
- [ ] Full feature parity
- [ ] Performance optimization
- [ ] Production testing
- [ ] Migration tooling
