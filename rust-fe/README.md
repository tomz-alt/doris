# Doris Rust FE - Proof of Concept

A high-performance, async Rust implementation of Apache Doris Frontend (FE) service with MySQL protocol support and streaming load capabilities.

## Features

- **MySQL Protocol Support**: Full MySQL wire protocol implementation for seamless client compatibility
- **Streaming Load**: HTTP-based streaming load API compatible with Doris stream load interface
- **Query Queuing**: Intelligent query queue management with configurable concurrency limits
- **Async I/O**: Built on Tokio runtime for efficient async I/O operations
- **Backend Integration**: gRPC-based communication with Doris Backend (BE) nodes
- **High Performance**: Designed for low latency and high throughput

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Doris Rust FE                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────┐         ┌──────────────────┐         │
│  │  MySQL Server    │         │   HTTP Server    │         │
│  │  (Port 9030)     │         │   (Port 8030)    │         │
│  │                  │         │                  │         │
│  │  - Handshake     │         │  - Stream Load   │         │
│  │  - Authentication│         │  - Health Check  │         │
│  │  - Query Exec    │         │  - Status API    │         │
│  └────────┬─────────┘         └────────┬─────────┘         │
│           │                            │                   │
│           └────────────┬───────────────┘                   │
│                        │                                   │
│              ┌─────────▼──────────┐                        │
│              │  Query Executor    │                        │
│              │                    │                        │
│              │  - Query Queue     │                        │
│              │  - Concurrency     │                        │
│              │  - Slot Management │                        │
│              └─────────┬──────────┘                        │
│                        │                                   │
│              ┌─────────▼──────────┐                        │
│              │ BE Client Pool     │                        │
│              │                    │                        │
│              │  - gRPC Clients    │                        │
│              │  - Load Balancing  │                        │
│              │  - Query Execution │                        │
│              └─────────┬──────────┘                        │
└──────────────────────────┼──────────────────────────────────┘
                          │
                          │ gRPC
                          │
                ┌─────────▼──────────┐
                │  Backend Nodes     │
                │  (Port 9070)       │
                │                    │
                │  - Query Execution │
                │  - Data Storage    │
                └────────────────────┘
```

## Prerequisites

- Rust 1.70+ (with cargo)
- Protocol Buffer compiler (protoc)
- MySQL client (for testing)
- curl (for testing stream load)

### Install Dependencies

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y build-essential protobuf-compiler mysql-client curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**macOS:**
```bash
brew install protobuf mysql-client
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Building

```bash
cd rust-fe

# Build in debug mode
cargo build

# Build in release mode (optimized)
cargo build --release
```

## Configuration

Create `fe_config.json` in the rust-fe directory:

```json
{
  "mysql_port": 9030,
  "http_port": 8030,
  "query_queue_size": 1024,
  "max_concurrent_queries": 100,
  "backend_nodes": [
    {
      "host": "127.0.0.1",
      "port": 9060,
      "grpc_port": 9070
    }
  ]
}
```

If no config file is found, default configuration will be used.

## Running

```bash
# Run in debug mode
cargo run

# Run in release mode
cargo run --release

# Run with custom log level
RUST_LOG=debug cargo run
```

The service will start:
- MySQL server on port 9030
- HTTP server on port 8030

## Testing

### 1. MySQL Protocol Tests

```bash
# Run MySQL verification tests
./tests/verify_mysql.sh

# Or manually test with mysql client
mysql -h 127.0.0.1 -P 9030 -u root

# Run some queries
mysql> SELECT 1;
mysql> SHOW DATABASES;
mysql> USE test;
mysql> SHOW TABLES;
mysql> SELECT * FROM example_table LIMIT 10;
```

### 2. Stream Load Tests

```bash
# Run stream load verification
./tests/verify_stream_load.sh

# Or manually test with curl
curl -X PUT \
  -H "label: test_load_$(date +%s)" \
  -H "format: csv" \
  -H "column_separator: ," \
  --data-binary @tests/test_data.csv \
  "http://127.0.0.1:8030/api/test/products/_stream_load"
```

### 3. Load Tests (Query Queuing)

```bash
# Run concurrent query load test
./tests/load_test.sh

# Custom configuration
CONCURRENT=20 QUERIES=500 ./tests/load_test.sh
```

### 4. Health & Status Checks

```bash
# Health check
curl http://127.0.0.1:8030/api/health

# Service status
curl http://127.0.0.1:8030/api/status
```

## API Reference

### MySQL Protocol

The service implements standard MySQL protocol, supporting:

- **Connection & Authentication**: MySQL native password authentication
- **Commands**:
  - `COM_QUERY`: Execute SQL queries
  - `COM_INIT_DB`: Change database
  - `COM_PING`: Keepalive ping
  - `COM_QUIT`: Close connection

### HTTP REST API

#### Stream Load

**Endpoint**: `PUT /api/{db}/{table}/_stream_load`

**Headers**:
- `label`: Unique label for this load (optional)
- `format`: Data format (csv, json) - default: csv
- `column_separator`: Column separator for CSV - default: ','

**Example**:
```bash
curl -X PUT \
  -H "label: my_load_label" \
  -H "format: csv" \
  --data-binary @data.csv \
  "http://localhost:8030/api/mydb/mytable/_stream_load"
```

**Response**:
```json
{
  "TxnId": "uuid",
  "Label": "my_load_label",
  "Status": "Success",
  "Message": "OK",
  "NumberTotalRows": 100,
  "NumberLoadedRows": 100,
  "NumberFilteredRows": 0,
  "LoadBytes": 1024,
  "LoadTimeMs": 150
}
```

#### Health Check

**Endpoint**: `GET /api/health`

**Response**:
```json
{
  "status": "healthy",
  "service": "doris-rust-fe"
}
```

#### Status

**Endpoint**: `GET /api/status`

**Response**:
```json
{
  "service": "doris-rust-fe",
  "query_queue": {
    "size": 10,
    "available_slots": 90,
    "max_concurrent": 100
  },
  "backend_nodes": 3
}
```

## Query Queuing Behavior

The service implements intelligent query queuing similar to the Java FE:

1. **Queue Management**: Queries are queued when submitted
2. **Concurrency Control**: Maximum concurrent queries enforced via semaphore
3. **FIFO Execution**: Queries executed in order when slots available
4. **Backpressure**: Returns error when queue is full

This ensures:
- Fair resource allocation
- Protection against overload
- Predictable performance under high load

## Performance Characteristics

- **Async I/O**: All I/O operations are async via Tokio
- **Zero-copy**: Efficient buffer management with `bytes` crate
- **Connection Pooling**: Reusable gRPC connections to BE nodes
- **Low Latency**: Direct memory access, minimal allocations
- **High Throughput**: Lock-free data structures where possible

## Migration to opensrv

For production use, consider migrating to `opensrv` for better MySQL protocol compatibility:

### Why opensrv?

- Battle-tested MySQL protocol implementation
- Used by Databend in production
- Better compatibility with MySQL clients
- Handles edge cases and advanced features

### Migration Steps

1. **Add opensrv dependency**:
```toml
[dependencies]
opensrv-mysql = "0.6"
```

2. **Replace custom MySQL module**:
```rust
use opensrv_mysql::{AsyncMysqlIntermediary, QueryResultWriter};

struct DorisShim {
    query_executor: Arc<QueryExecutor>,
}

#[async_trait]
impl AsyncMysqlIntermediary for DorisShim {
    async fn on_query(&mut self, query: &str, writer: QueryResultWriter) -> Result<()> {
        // Execute query via query_executor
        // Write results via writer
    }
}
```

3. **Start server**:
```rust
let listener = TcpListener::bind("0.0.0.0:9030").await?;
loop {
    let (stream, _) = listener.accept().await?;
    let shim = DorisShim { query_executor: executor.clone() };
    tokio::spawn(opensrv_mysql::serve(stream, shim));
}
```

## PostgreSQL Extension Support

To make this embeddable as a PostgreSQL extension for CDC:

### Using pgrx

1. **Add pgrx dependency**:
```toml
[dependencies]
pgrx = "0.11"
```

2. **Create extension wrapper**:
```rust
use pgrx::prelude::*;

#[pg_extern]
fn doris_stream_load(
    table_name: &str,
    data: &str
) -> Result<i64, String> {
    // Call into DorisClient to stream data
}

#[pg_extern]
fn doris_query(sql: &str) -> Result<Vec<String>, String> {
    // Execute query on Doris cluster
}
```

3. **Build extension**:
```bash
cargo pgrx init
cargo pgrx package
```

### Architecture for PostgreSQL Integration

```
┌────────────────────────────────────────┐
│         PostgreSQL Server              │
│  ┌──────────────────────────────────┐  │
│  │      Doris Extension (pgrx)      │  │
│  │                                  │  │
│  │  - Logical Replication Slot     │  │
│  │  - CDC Stream Processor         │  │
│  │  - Doris Client Library         │  │
│  │    (reuse BE client code)       │  │
│  └───────────┬──────────────────────┘  │
└──────────────┼─────────────────────────┘
               │
               │ Change Stream
               │
      ┌────────▼────────┐
      │  Doris Cluster  │
      │                 │
      │  - Stream Load  │
      │  - Real-time    │
      └─────────────────┘
```

### Decoupling Strategy

To make components reusable:

1. **Core Library** (`doris-core`):
   - Query executor
   - BE client
   - Protocol definitions

2. **MySQL Server** (`doris-mysql-server`):
   - MySQL protocol layer
   - Uses opensrv or custom implementation

3. **PostgreSQL Extension** (`doris-pg-extension`):
   - pgrx wrapper
   - CDC capture
   - Stream to Doris

4. **HTTP Server** (`doris-http-server`):
   - REST API
   - Stream load endpoint

Each component imports `doris-core` for common functionality.

## Limitations (PoC)

This is a proof-of-concept with the following limitations:

1. **Authentication**: Simplified (accepts any password)
2. **SQL Parsing**: Minimal parsing, relies on BE for execution
3. **Prepared Statements**: Not implemented
4. **Transactions**: Not fully implemented
5. **Metadata**: Mock responses for SHOW commands
6. **Error Handling**: Basic error messages

For production use:
- Implement proper authentication
- Add SQL parser (e.g., sqlparser-rs)
- Implement full transaction support
- Add comprehensive metadata management
- Enhance error handling and logging

## Directory Structure

```
rust-fe/
├── Cargo.toml              # Dependencies
├── build.rs                # Build script for protobuf
├── proto/                  # Protocol buffer definitions
│   └── backend_service.proto
├── src/
│   ├── main.rs            # Entry point
│   ├── config.rs          # Configuration
│   ├── error.rs           # Error types
│   ├── mysql/             # MySQL protocol
│   │   ├── mod.rs
│   │   ├── protocol.rs
│   │   ├── packet.rs
│   │   ├── connection.rs
│   │   └── server.rs
│   ├── http/              # HTTP server
│   │   ├── mod.rs
│   │   ├── server.rs
│   │   └── handlers.rs
│   ├── query/             # Query execution
│   │   ├── mod.rs
│   │   ├── executor.rs
│   │   └── queue.rs
│   └── be/                # Backend communication
│       ├── mod.rs
│       ├── client.rs
│       └── pool.rs
└── tests/
    ├── test_data.csv
    ├── verify_mysql.sh
    ├── verify_stream_load.sh
    └── load_test.sh
```

## Development

### Code Quality

```bash
# Format code
cargo fmt

# Run clippy lints
cargo clippy --all-targets --all-features

# Run tests
cargo test
```

### Logging

Set log level via `RUST_LOG` environment variable:

```bash
# Debug level
RUST_LOG=debug cargo run

# Trace level (very verbose)
RUST_LOG=trace cargo run

# Module-specific logging
RUST_LOG=doris_rust_fe::mysql=trace,doris_rust_fe::query=debug cargo run
```

## Benchmarking

```bash
# Build optimized binary
cargo build --release

# Run with benchmarking tools
# MySQL benchmark
mysqlslap -h 127.0.0.1 -P 9030 -u root \
  --query="SELECT * FROM test.example_table LIMIT 10" \
  --concurrency=50 --iterations=100

# HTTP benchmark
ab -n 1000 -c 10 -T "text/csv" -p tests/test_data.csv \
  http://127.0.0.1:8030/api/test/products/_stream_load
```

## Contributing

This is a PoC implementation. For production use, consider:

1. Security audits
2. Performance testing at scale
3. Integration testing with real Doris BE nodes
4. Comprehensive error handling
5. Metrics and monitoring
6. Configuration validation

## License

Apache License 2.0 (consistent with Apache Doris)

## References

- [Apache Doris Documentation](https://doris.apache.org/docs/)
- [MySQL Protocol Documentation](https://dev.mysql.com/doc/internals/en/client-server-protocol.html)
- [opensrv](https://github.com/databendlabs/opensrv) - MySQL server implementation
- [rust-postgres](https://github.com/rust-postgres/rust-postgres) - PostgreSQL client
- [pgrx](https://github.com/pgcentralfoundation/pgrx) - PostgreSQL extension framework
- [Tokio Documentation](https://tokio.rs/)
- [tonic gRPC](https://github.com/hyperium/tonic)
