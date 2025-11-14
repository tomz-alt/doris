# Migration to opensrv for Production

This guide shows how to migrate from the custom MySQL protocol implementation to `opensrv` for production use.

## Why Migrate to opensrv?

The current PoC implements a custom MySQL protocol handler. For production, `opensrv` offers:

1. **Battle-tested**: Used by Databend in production
2. **Complete Protocol Support**: Handles all MySQL protocol nuances
3. **Better Client Compatibility**: Works with all MySQL clients and tools
4. **Prepared Statements**: Full support for prepared statements
5. **SSL/TLS**: Built-in support for secure connections
6. **Compression**: Protocol-level compression support

## Migration Steps

### 1. Update Dependencies

**Cargo.toml**:
```toml
[dependencies]
# Replace custom MySQL implementation
opensrv-mysql = "0.6"

# Keep existing dependencies
tokio = { version = "1.35", features = ["full"] }
bytes = "1.5"
# ... other deps
```

### 2. Create MySQL Shim

Create `src/mysql/opensrv_shim.rs`:

```rust
use opensrv_mysql::{
    AsyncMysqlIntermediary,
    QueryResultWriter,
    Column,
    ColumnType,
    ColumnFlags,
    ErrorKind,
};
use async_trait::async_trait;
use std::sync::Arc;

use crate::query::QueryExecutor;
use crate::be::BackendClientPool;

pub struct DorisShim {
    query_executor: Arc<QueryExecutor>,
    be_client_pool: Arc<BackendClientPool>,
    current_db: Option<String>,
}

impl DorisShim {
    pub fn new(
        query_executor: Arc<QueryExecutor>,
        be_client_pool: Arc<BackendClientPool>,
    ) -> Self {
        Self {
            query_executor,
            be_client_pool,
            current_db: None,
        }
    }
}

#[async_trait]
impl AsyncMysqlIntermediary for DorisShim {
    async fn on_prepare(
        &mut self,
        query: &str,
        info: opensrv_mysql::StatementMetaWriter,
    ) -> Result<(), opensrv_mysql::Error> {
        // For PoC, we can reject prepared statements
        Err(opensrv_mysql::Error::new(
            ErrorKind::ER_UNKNOWN_ERROR,
            "Prepared statements not supported yet".to_string(),
        ))
    }

    async fn on_execute(
        &mut self,
        id: u32,
        params: opensrv_mysql::ParamParser,
        results: QueryResultWriter,
    ) -> Result<(), opensrv_mysql::Error> {
        // Handle prepared statement execution
        Err(opensrv_mysql::Error::new(
            ErrorKind::ER_UNKNOWN_ERROR,
            "Prepared statements not supported yet".to_string(),
        ))
    }

    async fn on_close(&mut self, id: u32) {
        // Clean up prepared statement
    }

    async fn on_query(
        &mut self,
        query: &str,
        results: QueryResultWriter,
    ) -> Result<(), opensrv_mysql::Error> {
        let query_trimmed = query.trim().to_lowercase();

        // Handle special commands
        if query_trimmed.starts_with("use ") {
            let db_name = query_trimmed
                .strip_prefix("use ")
                .unwrap()
                .trim_end_matches(';')
                .trim();
            self.current_db = Some(db_name.to_string());
            results.completed(0, 0)?;
            return Ok(());
        }

        if query_trimmed.starts_with("set ") {
            // Accept SET statements
            results.completed(0, 0)?;
            return Ok(());
        }

        // Handle metadata queries
        if query_trimmed == "select database()" {
            let cols = vec![Column {
                table: "".to_string(),
                column: "DATABASE()".to_string(),
                coltype: ColumnType::MYSQL_TYPE_VAR_STRING,
                colflags: ColumnFlags::empty(),
            }];

            results.start(&cols)?;

            let db = self.current_db.as_deref().unwrap_or("NULL");
            results.write_row(&[db])?;
            results.finish()?;

            return Ok(());
        }

        // Execute query via query executor
        let query_id = uuid::Uuid::new_v4();

        match self.query_executor.queue_query(
            query_id,
            query.to_string(),
            self.current_db.clone()
        ).await {
            Ok(()) => {
                match self.query_executor.execute_query(
                    query_id,
                    &self.be_client_pool
                ).await {
                    Ok(result) => {
                        if result.is_dml {
                            // DML query
                            results.completed(result.affected_rows, 0)?;
                        } else {
                            // SELECT query
                            let cols: Vec<Column> = result.columns.iter()
                                .map(|col| Column {
                                    table: "".to_string(),
                                    column: col.name.clone(),
                                    coltype: map_column_type(col.column_type),
                                    colflags: ColumnFlags::empty(),
                                })
                                .collect();

                            results.start(&cols)?;

                            for row in result.rows {
                                let values: Vec<&str> = row.values.iter()
                                    .map(|v| v.as_deref().unwrap_or("NULL"))
                                    .collect();
                                results.write_row(&values)?;
                            }

                            results.finish()?;
                        }
                        Ok(())
                    }
                    Err(e) => {
                        Err(opensrv_mysql::Error::new(
                            ErrorKind::ER_UNKNOWN_ERROR,
                            format!("Query execution failed: {}", e),
                        ))
                    }
                }
            }
            Err(e) => {
                Err(opensrv_mysql::Error::new(
                    ErrorKind::ER_QUERY_INTERRUPTED,
                    format!("Query queue full: {}", e),
                ))
            }
        }
    }

    async fn on_init(&mut self, database: &str) -> Result<(), opensrv_mysql::Error> {
        self.current_db = Some(database.to_string());
        Ok(())
    }
}

fn map_column_type(doris_type: u8) -> ColumnType {
    // Map Doris column types to MySQL column types
    match doris_type {
        0x00 => ColumnType::MYSQL_TYPE_DECIMAL,
        0x01 => ColumnType::MYSQL_TYPE_TINY,
        0x02 => ColumnType::MYSQL_TYPE_SHORT,
        0x03 => ColumnType::MYSQL_TYPE_LONG,
        0x04 => ColumnType::MYSQL_TYPE_FLOAT,
        0x05 => ColumnType::MYSQL_TYPE_DOUBLE,
        0x08 => ColumnType::MYSQL_TYPE_LONGLONG,
        0x0f => ColumnType::MYSQL_TYPE_VARCHAR,
        0xfd => ColumnType::MYSQL_TYPE_VAR_STRING,
        0xfe => ColumnType::MYSQL_TYPE_STRING,
        _ => ColumnType::MYSQL_TYPE_VAR_STRING,
    }
}
```

### 3. Update MySQL Server

Create `src/mysql/opensrv_server.rs`:

```rust
use tokio::net::TcpListener;
use std::sync::Arc;
use tracing::{info, error};

use crate::error::Result;
use crate::query::QueryExecutor;
use crate::be::BackendClientPool;
use super::opensrv_shim::DorisShim;

pub struct MysqlServer {
    port: u16,
    query_executor: Arc<QueryExecutor>,
    be_client_pool: Arc<BackendClientPool>,
}

impl MysqlServer {
    pub fn new(
        port: u16,
        query_executor: Arc<QueryExecutor>,
        be_client_pool: Arc<BackendClientPool>,
    ) -> Self {
        Self {
            port,
            query_executor,
            be_client_pool,
        }
    }

    pub async fn serve(self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;

        info!("MySQL server (opensrv) listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let query_executor = self.query_executor.clone();
                    let be_client_pool = self.be_client_pool.clone();

                    info!("Accepted connection from {}", addr);

                    tokio::spawn(async move {
                        let shim = DorisShim::new(query_executor, be_client_pool);

                        if let Err(e) = opensrv_mysql::AsyncMysqlIntermediary::run_on(
                            shim,
                            stream
                        ).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}
```

### 4. Update Module Structure

**src/mysql/mod.rs**:
```rust
#[cfg(feature = "opensrv")]
mod opensrv_shim;
#[cfg(feature = "opensrv")]
mod opensrv_server;

#[cfg(not(feature = "opensrv"))]
mod protocol;
#[cfg(not(feature = "opensrv"))]
mod packet;
#[cfg(not(feature = "opensrv"))]
mod server;
#[cfg(not(feature = "opensrv"))]
mod connection;

#[cfg(feature = "opensrv")]
pub use opensrv_server::MysqlServer;

#[cfg(not(feature = "opensrv"))]
pub use server::MysqlServer;
```

### 5. Update Cargo.toml Features

```toml
[features]
default = []
opensrv = ["opensrv-mysql"]

[dependencies]
# Make opensrv optional
opensrv-mysql = { version = "0.6", optional = true }
```

### 6. Build and Test

```bash
# Build with opensrv
cargo build --release --features opensrv

# Run
cargo run --release --features opensrv

# Test
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT 1"
```

## Benefits After Migration

1. **SSL/TLS Support**: Enable with `opensrv` configuration
2. **Prepared Statements**: Full support
3. **Better Error Messages**: More MySQL-compatible error codes
4. **Protocol Compression**: Reduce network bandwidth
5. **Authentication Plugins**: Support for various auth methods

## Advanced Configuration

### Enable SSL/TLS

```rust
use opensrv_mysql::SslConfig;

let ssl_config = SslConfig {
    cert_path: "path/to/cert.pem",
    key_path: "path/to/key.pem",
};

// Pass ssl_config when creating server
```

### Custom Authentication

```rust
#[async_trait]
impl AsyncMysqlIntermediary for DorisShim {
    async fn on_auth(
        &mut self,
        username: &str,
        password: &[u8],
    ) -> Result<bool, opensrv_mysql::Error> {
        // Implement custom authentication logic
        // e.g., check against LDAP, OAuth, etc.
        Ok(true)
    }
}
```

## Performance Considerations

- `opensrv` adds minimal overhead compared to custom implementation
- Prepared statements can improve performance for repeated queries
- SSL/TLS adds ~10-20% latency but essential for security
- Protocol compression helps with large result sets

## Gradual Migration

You can support both implementations:

```bash
# Use custom implementation
cargo run

# Use opensrv
cargo run --features opensrv
```

This allows gradual testing and migration.

## Testing Checklist

After migration, test:

- [ ] Basic connection and authentication
- [ ] Simple SELECT queries
- [ ] DML operations (INSERT, UPDATE, DELETE)
- [ ] Database switching (USE command)
- [ ] SHOW commands
- [ ] SET commands
- [ ] Error handling
- [ ] Concurrent connections
- [ ] Long-running queries
- [ ] Connection timeouts
- [ ] SSL/TLS connections (if enabled)
- [ ] Prepared statements (if implemented)

## Production Recommendations

1. **Use opensrv** for MySQL protocol
2. **Enable SSL/TLS** for security
3. **Implement proper authentication** (not just accept any password)
4. **Add connection limits** to prevent resource exhaustion
5. **Monitor connections** with metrics
6. **Set query timeouts** to prevent long-running queries
7. **Add rate limiting** per user/connection

## References

- [opensrv GitHub](https://github.com/databendlabs/opensrv)
- [opensrv MySQL Documentation](https://docs.rs/opensrv-mysql/)
- [MySQL Protocol Documentation](https://dev.mysql.com/doc/internals/en/client-server-protocol.html)
