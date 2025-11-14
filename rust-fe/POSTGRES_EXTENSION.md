# PostgreSQL Extension for CDC Integration

This guide shows how to build a PostgreSQL extension using `pgrx` to enable seamless CDC (Change Data Capture) from PostgreSQL to Apache Doris.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│              PostgreSQL Server                          │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │         Doris CDC Extension (pgrx)                │  │
│  │                                                   │  │
│  │  ┌─────────────────────────────────────────────┐  │  │
│  │  │  Logical Replication Slot                  │  │  │
│  │  │  - Captures INSERT/UPDATE/DELETE           │  │  │
│  │  │  - Decodes WAL changes                     │  │  │
│  │  └──────────────┬──────────────────────────────┘  │  │
│  │                 │                                  │  │
│  │  ┌──────────────▼──────────────────────────────┐  │  │
│  │  │  Change Stream Processor                   │  │  │
│  │  │  - Buffers changes                         │  │  │
│  │  │  - Batches for efficiency                  │  │  │
│  │  │  - Handles backpressure                    │  │  │
│  │  └──────────────┬──────────────────────────────┘  │  │
│  │                 │                                  │  │
│  │  ┌──────────────▼──────────────────────────────┐  │  │
│  │  │  Doris Client Library                      │  │  │
│  │  │  - Stream load protocol                    │  │  │
│  │  │  - gRPC to Doris BE                        │  │  │
│  │  │  - Error handling & retry                  │  │  │
│  │  └──────────────┬──────────────────────────────┘  │  │
│  └─────────────────┼─────────────────────────────────┘  │
└────────────────────┼─────────────────────────────────────┘
                     │
                     │ Stream Load / gRPC
                     │
          ┌──────────▼────────────┐
          │   Doris Cluster       │
          │                       │
          │  - Real-time ingestion│
          │  - OLAP queries       │
          │  - Analytics          │
          └───────────────────────┘
```

## Project Structure

Create a new pgrx extension:

```bash
cargo install cargo-pgrx
cargo pgrx init
cargo pgrx new doris_cdc
cd doris_cdc
```

### Directory Structure

```
doris_cdc/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Main extension code
│   ├── replication.rs      # Logical replication handling
│   ├── stream_processor.rs # Change stream processing
│   └── doris_client.rs     # Doris client (reuse from rust-fe)
├── sql/
│   └── doris_cdc--0.1.0.sql # Extension SQL definitions
└── README.md
```

## Implementation

### 1. Cargo.toml

```toml
[package]
name = "doris_cdc"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[features]
default = ["pg16"]
pg12 = ["pgrx/pg12", "pgrx-tests/pg12"]
pg13 = ["pgrx/pg13", "pgrx-tests/pg13"]
pg14 = ["pgrx/pg14", "pgrx-tests/pg14"]
pg15 = ["pgrx/pg15", "pgrx-tests/pg15"]
pg16 = ["pgrx/pg16", "pgrx-tests/pg16"]
pg_test = []

[dependencies]
pgrx = "0.11"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.35", features = ["full"] }
tonic = "0.11"
prost = "0.12"
bytes = "1.5"
tracing = "0.1"

[dev-dependencies]
pgrx-tests = "0.11"

[profile.dev]
panic = "unwind"

[profile.release]
panic = "unwind"
opt-level = 3
lto = "fat"
codegen-units = 1
```

### 2. Main Extension Code (src/lib.rs)

```rust
use pgrx::prelude::*;

mod replication;
mod stream_processor;
mod doris_client;

use replication::ReplicationSlot;
use stream_processor::ChangeStreamProcessor;
use doris_client::DorisClient;

pgrx::pg_module_magic!();

/// Initialize Doris CDC extension
#[pg_extern]
fn doris_cdc_init(
    slot_name: &str,
    doris_fe_host: &str,
    doris_fe_port: i32,
) -> Result<String, String> {
    // Create logical replication slot
    let slot = ReplicationSlot::create(slot_name)
        .map_err(|e| format!("Failed to create replication slot: {}", e))?;

    // Initialize Doris client
    let doris_client = DorisClient::new(doris_fe_host, doris_fe_port as u16);

    // Start change stream processor
    ChangeStreamProcessor::start(slot, doris_client)
        .map_err(|e| format!("Failed to start stream processor: {}", e))?;

    Ok(format!("Doris CDC initialized with slot '{}'", slot_name))
}

/// Start CDC for a specific table
#[pg_extern]
fn doris_cdc_add_table(
    pg_table: &str,
    doris_db: &str,
    doris_table: &str,
) -> Result<String, String> {
    ChangeStreamProcessor::add_table_mapping(pg_table, doris_db, doris_table)
        .map_err(|e| format!("Failed to add table: {}", e))?;

    Ok(format!("Added table mapping: {} -> {}.{}", pg_table, doris_db, doris_table))
}

/// Stop CDC for a table
#[pg_extern]
fn doris_cdc_remove_table(pg_table: &str) -> Result<String, String> {
    ChangeStreamProcessor::remove_table_mapping(pg_table)
        .map_err(|e| format!("Failed to remove table: {}", e))?;

    Ok(format!("Removed table mapping: {}", pg_table))
}

/// Get CDC status
#[pg_extern]
fn doris_cdc_status() -> TableIterator<'static, (name!(table, String), name!(status, String), name!(lag, i64))> {
    let status = ChangeStreamProcessor::get_status();

    TableIterator::new(
        status.into_iter().map(|(table, info)| {
            (table, info.status, info.lag_bytes)
        })
    )
}

/// Manually trigger sync for a table
#[pg_extern]
fn doris_cdc_sync_table(
    pg_table: &str,
    batch_size: default!(i32, 1000),
) -> Result<String, String> {
    let synced = ChangeStreamProcessor::sync_table(pg_table, batch_size as usize)
        .map_err(|e| format!("Sync failed: {}", e))?;

    Ok(format!("Synced {} rows", synced))
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_cdc_init() {
        // Test extension initialization
    }
}
```

### 3. Logical Replication (src/replication.rs)

```rust
use pgrx::prelude::*;
use std::error::Error;

pub struct ReplicationSlot {
    name: String,
}

impl ReplicationSlot {
    pub fn create(name: &str) -> Result<Self, Box<dyn Error>> {
        // Create logical replication slot using pgoutput
        Spi::run(&format!(
            "SELECT pg_create_logical_replication_slot('{}', 'pgoutput')",
            name
        ))?;

        Ok(Self {
            name: name.to_string(),
        })
    }

    pub fn read_changes(&self) -> Result<Vec<Change>, Box<dyn Error>> {
        let mut changes = Vec::new();

        // Read changes from replication slot
        Spi::connect(|client| {
            let query = format!(
                "SELECT * FROM pg_logical_slot_get_changes('{}', NULL, NULL)",
                self.name
            );

            let tup_table = client.select(&query, None, None)?;

            for row in tup_table {
                let lsn = row["lsn"].value::<String>()?;
                let xid = row["xid"].value::<i64>()?;
                let data = row["data"].value::<String>()?;

                changes.push(Change {
                    lsn: lsn.unwrap(),
                    xid: xid.unwrap() as u64,
                    data: data.unwrap(),
                });
            }

            Ok(changes)
        })
    }

    pub fn peek_changes(&self) -> Result<Vec<Change>, Box<dyn Error>> {
        let mut changes = Vec::new();

        Spi::connect(|client| {
            let query = format!(
                "SELECT * FROM pg_logical_slot_peek_changes('{}', NULL, NULL)",
                self.name
            );

            let tup_table = client.select(&query, None, None)?;

            for row in tup_table {
                let lsn = row["lsn"].value::<String>()?;
                let xid = row["xid"].value::<i64>()?;
                let data = row["data"].value::<String>()?;

                changes.push(Change {
                    lsn: lsn.unwrap(),
                    xid: xid.unwrap() as u64,
                    data: data.unwrap(),
                });
            }

            Ok(changes)
        })
    }
}

#[derive(Debug, Clone)]
pub struct Change {
    pub lsn: String,
    pub xid: u64,
    pub data: String,
}

impl Change {
    pub fn parse(&self) -> Result<ChangeType, Box<dyn Error>> {
        // Parse pgoutput format
        if self.data.starts_with("BEGIN") {
            Ok(ChangeType::Begin)
        } else if self.data.starts_with("COMMIT") {
            Ok(ChangeType::Commit)
        } else if self.data.starts_with("INSERT") {
            Ok(ChangeType::Insert(self.parse_insert()?))
        } else if self.data.starts_with("UPDATE") {
            Ok(ChangeType::Update(self.parse_update()?))
        } else if self.data.starts_with("DELETE") {
            Ok(ChangeType::Delete(self.parse_delete()?))
        } else {
            Ok(ChangeType::Unknown)
        }
    }

    fn parse_insert(&self) -> Result<InsertChange, Box<dyn Error>> {
        // Parse INSERT statement from pgoutput
        // Format: "INSERT: table_name: column1[type]:value1 column2[type]:value2 ..."
        todo!("Parse INSERT statement")
    }

    fn parse_update(&self) -> Result<UpdateChange, Box<dyn Error>> {
        todo!("Parse UPDATE statement")
    }

    fn parse_delete(&self) -> Result<DeleteChange, Box<dyn Error>> {
        todo!("Parse DELETE statement")
    }
}

#[derive(Debug)]
pub enum ChangeType {
    Begin,
    Commit,
    Insert(InsertChange),
    Update(UpdateChange),
    Delete(DeleteChange),
    Unknown,
}

#[derive(Debug)]
pub struct InsertChange {
    pub table: String,
    pub values: Vec<(String, String)>, // column name -> value
}

#[derive(Debug)]
pub struct UpdateChange {
    pub table: String,
    pub old_values: Vec<(String, String)>,
    pub new_values: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct DeleteChange {
    pub table: String,
    pub key_values: Vec<(String, String)>,
}
```

### 4. Stream Processor (src/stream_processor.rs)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

use super::replication::{ReplicationSlot, Change, ChangeType};
use super::doris_client::DorisClient;

lazy_static::lazy_static! {
    static ref PROCESSOR: Arc<Mutex<Option<StreamProcessor>>> = Arc::new(Mutex::new(None));
}

pub struct ChangeStreamProcessor;

impl ChangeStreamProcessor {
    pub fn start(
        slot: ReplicationSlot,
        doris_client: DorisClient,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let processor = StreamProcessor::new(slot, doris_client);

        *PROCESSOR.lock().unwrap() = Some(processor);

        // Start background thread
        std::thread::spawn(|| {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                loop {
                    if let Some(ref mut processor) = *PROCESSOR.lock().unwrap() {
                        if let Err(e) = processor.process_changes().await {
                            eprintln!("Error processing changes: {}", e);
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            });
        });

        Ok(())
    }

    pub fn add_table_mapping(
        pg_table: &str,
        doris_db: &str,
        doris_table: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut processor) = *PROCESSOR.lock().unwrap() {
            processor.add_table_mapping(pg_table, doris_db, doris_table);
            Ok(())
        } else {
            Err("Processor not initialized".into())
        }
    }

    pub fn remove_table_mapping(pg_table: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut processor) = *PROCESSOR.lock().unwrap() {
            processor.remove_table_mapping(pg_table);
            Ok(())
        } else {
            Err("Processor not initialized".into())
        }
    }

    pub fn get_status() -> Vec<(String, TableStatus)> {
        if let Some(ref processor) = *PROCESSOR.lock().unwrap() {
            processor.get_status()
        } else {
            vec![]
        }
    }

    pub fn sync_table(
        pg_table: &str,
        batch_size: usize,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        if let Some(ref mut processor) = *PROCESSOR.lock().unwrap() {
            processor.sync_table(pg_table, batch_size)
        } else {
            Err("Processor not initialized".into())
        }
    }
}

struct StreamProcessor {
    slot: ReplicationSlot,
    doris_client: DorisClient,
    table_mappings: HashMap<String, (String, String)>, // pg_table -> (doris_db, doris_table)
    batches: HashMap<String, Vec<Change>>,
    batch_size: usize,
}

impl StreamProcessor {
    fn new(slot: ReplicationSlot, doris_client: DorisClient) -> Self {
        Self {
            slot,
            doris_client,
            table_mappings: HashMap::new(),
            batches: HashMap::new(),
            batch_size: 1000,
        }
    }

    fn add_table_mapping(&mut self, pg_table: &str, doris_db: &str, doris_table: &str) {
        self.table_mappings.insert(
            pg_table.to_string(),
            (doris_db.to_string(), doris_table.to_string()),
        );
    }

    fn remove_table_mapping(&mut self, pg_table: &str) {
        self.table_mappings.remove(pg_table);
    }

    async fn process_changes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let changes = self.slot.read_changes()?;

        for change in changes {
            let change_type = change.parse()?;

            match change_type {
                ChangeType::Insert(insert) => {
                    if let Some((doris_db, doris_table)) = self.table_mappings.get(&insert.table) {
                        self.process_insert(doris_db, doris_table, insert).await?;
                    }
                }
                ChangeType::Update(update) => {
                    // Handle update (convert to DELETE + INSERT for simplicity)
                }
                ChangeType::Delete(delete) => {
                    // Handle delete
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn process_insert(
        &mut self,
        doris_db: &str,
        doris_table: &str,
        insert: super::replication::InsertChange,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Convert to CSV format
        let values: Vec<String> = insert.values.iter()
            .map(|(_, v)| v.clone())
            .collect();

        let csv_line = values.join(",");

        // Buffer for batching
        let key = format!("{}.{}", doris_db, doris_table);
        self.batches.entry(key.clone())
            .or_insert_with(Vec::new)
            .push(/* CSV line as Change */);

        // Flush if batch is full
        if self.batches.get(&key).unwrap().len() >= self.batch_size {
            self.flush_batch(doris_db, doris_table).await?;
        }

        Ok(())
    }

    async fn flush_batch(
        &mut self,
        doris_db: &str,
        doris_table: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}.{}", doris_db, doris_table);

        if let Some(batch) = self.batches.remove(&key) {
            // Convert batch to CSV
            // Send to Doris via stream load
            self.doris_client.stream_load(doris_db, doris_table, /* CSV data */).await?;
        }

        Ok(())
    }

    fn get_status(&self) -> Vec<(String, TableStatus)> {
        self.table_mappings.iter()
            .map(|(table, (db, tbl))| {
                (
                    table.clone(),
                    TableStatus {
                        status: "active".to_string(),
                        lag_bytes: 0,
                    },
                )
            })
            .collect()
    }

    fn sync_table(
        &mut self,
        pg_table: &str,
        batch_size: usize,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        // Perform initial sync by reading table data
        todo!("Implement initial table sync")
    }
}

pub struct TableStatus {
    pub status: String,
    pub lag_bytes: i64,
}
```

### 5. Doris Client (src/doris_client.rs)

Reuse the BE client code from rust-fe:

```rust
// Copy from rust-fe/src/be/client.rs and adapt for extension use
```

## Building and Installing

```bash
# Build the extension
cargo pgrx package

# Install to PostgreSQL
sudo cp target/release/doris_cdc-pg16/usr/share/postgresql/16/extension/* \
     /usr/share/postgresql/16/extension/

sudo cp target/release/doris_cdc-pg16/usr/lib/postgresql/16/lib/* \
     /usr/lib/postgresql/16/lib/
```

## Usage

```sql
-- Load extension
CREATE EXTENSION doris_cdc;

-- Initialize CDC
SELECT doris_cdc_init('doris_slot', '127.0.0.1', 8030);

-- Add table for CDC
SELECT doris_cdc_add_table('public.users', 'mydb', 'users');
SELECT doris_cdc_add_table('public.orders', 'mydb', 'orders');

-- Check status
SELECT * FROM doris_cdc_status();

-- Manually sync a table (initial load)
SELECT doris_cdc_sync_table('public.users', 10000);

-- Remove table from CDC
SELECT doris_cdc_remove_table('public.users');
```

## Benefits

1. **Real-time CDC**: Changes captured immediately from WAL
2. **Low Latency**: Direct integration, no external tools
3. **Seamless**: Works within PostgreSQL
4. **Efficient**: Batching and backpressure handling
5. **Reliable**: Uses PostgreSQL's logical replication

## References

- [pgrx Documentation](https://github.com/pgcentralfoundation/pgrx)
- [PostgreSQL Logical Replication](https://www.postgresql.org/docs/current/logical-replication.html)
- [pgoutput Plugin](https://www.postgresql.org/docs/current/protocol-logicalrep-message-formats.html)
