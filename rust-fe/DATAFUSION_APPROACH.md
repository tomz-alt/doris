# Using DataFusion for Query Planning and Execution

## Game Changer: DataFusion Integration

Instead of building a query planner from scratch, we can use **Apache Arrow DataFusion** - a complete SQL query engine in Rust!

## What is DataFusion?

DataFusion is Apache Arrow's query execution framework:
- ✅ Complete SQL parser
- ✅ Query planner and optimizer
- ✅ Physical execution engine
- ✅ Already tested with TPC-H benchmarks
- ✅ Native Rust, async, uses Arrow columnar format
- ✅ Extensible with custom data sources

## Architecture Options

### Option 1: DataFusion as Planning Layer (RECOMMENDED)
```
MySQL Client
    ↓
SQL Query
    ↓
DataFusion Parser → DataFusion Planner → Optimize
    ↓
Convert to Doris Plan Fragments
    ↓
Send to Doris BE via gRPC
    ↓
Execute on BE, Return Results
    ↓
Format to MySQL Protocol
```

**Advantages:**
- Leverage DataFusion's optimizer
- Get TPC-H support "for free"
- BE still does the actual execution (distributed)
- Clean separation: planning vs execution

### Option 2: DataFusion as Executor (Simpler for PoC)
```
MySQL Client
    ↓
SQL Query
    ↓
DataFusion Parser → Planner → Executor
    ↓
Read from CSV/Parquet files
    ↓
Execute query entirely in DataFusion
    ↓
Format results to MySQL Protocol
```

**Advantages:**
- No BE needed for initial testing
- Can run TPC-H queries immediately
- Simpler setup
- Full Rust stack

**Disadvantages:**
- Bypasses Doris BE (not real Doris)
- Doesn't test FE-BE communication
- Not distributed

### Option 3: Hybrid Approach
```
Use DataFusion for PoC → Later integrate with BE
```

Start with Option 2 to prove TPC-H works, then add Option 1 for real BE integration.

## Implementation Plan with DataFusion

### Step 1: Add DataFusion Dependency (15 min)

**Cargo.toml**:
```toml
[dependencies]
# DataFusion for query planning and execution
datafusion = "34.0"
arrow = "49.0"
arrow-schema = "49.0"

# Remove or keep sqlparser as DataFusion includes it
# sqlparser = "0.43"  # Can remove, DataFusion uses this internally
```

### Step 2: Create DataFusion Context (30 min)

**File**: `src/planner/datafusion.rs` (NEW)

```rust
use datafusion::prelude::*;
use datafusion::error::Result as DFResult;
use datafusion::arrow::datatypes::Schema;
use std::sync::Arc;

pub struct DataFusionPlanner {
    ctx: SessionContext,
}

impl DataFusionPlanner {
    pub async fn new() -> Self {
        let ctx = SessionContext::new();

        // Register TPC-H tables
        Self::register_tpch_tables(&ctx).await;

        Self { ctx }
    }

    async fn register_tpch_tables(ctx: &SessionContext) {
        // Option A: Use CSV files (for testing)
        // ctx.register_csv("lineitem", "data/lineitem.tbl", CsvReadOptions::new()).await.unwrap();

        // Option B: Use custom TableProvider that talks to BE
        // ctx.register_table("lineitem", Arc::new(DorisTableProvider::new())).unwrap();

        // For now, register from our metadata catalog
        use crate::metadata::catalog;
        let catalog = catalog::catalog();

        if let Some(db) = catalog.get_database("tpch") {
            for table_name in db.list_tables() {
                if let Some(table) = db.get_table(&table_name) {
                    // Create Arrow schema from our table definition
                    let arrow_schema = table_to_arrow_schema(&table);

                    // Register empty table (will add data source later)
                    ctx.register_table(
                        &table_name,
                        Arc::new(EmptyTable::new(Arc::new(arrow_schema)))
                    ).unwrap();
                }
            }
        }
    }

    pub async fn plan_query(&self, sql: &str) -> DFResult<LogicalPlan> {
        // DataFusion does everything: parse, validate, plan, optimize
        let df = self.ctx.sql(sql).await?;
        let plan = df.logical_plan().clone();
        Ok(plan)
    }

    pub async fn execute_query(&self, sql: &str) -> DFResult<DataFrame> {
        self.ctx.sql(sql).await
    }
}

use datafusion::datasource::empty::EmptyTable;
use crate::metadata::schema::Table;
use arrow::datatypes::{DataType as ArrowDataType, Field, Schema as ArrowSchema};

fn table_to_arrow_schema(table: &Table) -> ArrowSchema {
    let fields: Vec<Field> = table.columns.iter()
        .map(|col| {
            let arrow_type = match &col.data_type {
                crate::metadata::types::DataType::TinyInt => ArrowDataType::Int8,
                crate::metadata::types::DataType::SmallInt => ArrowDataType::Int16,
                crate::metadata::types::DataType::Int => ArrowDataType::Int32,
                crate::metadata::types::DataType::BigInt => ArrowDataType::Int64,
                crate::metadata::types::DataType::Float => ArrowDataType::Float32,
                crate::metadata::types::DataType::Double => ArrowDataType::Float64,
                crate::metadata::types::DataType::Decimal { precision, scale } => {
                    ArrowDataType::Decimal128(*precision as u8, *scale as i8)
                }
                crate::metadata::types::DataType::Varchar { .. }
                | crate::metadata::types::DataType::String
                | crate::metadata::types::DataType::Text => ArrowDataType::Utf8,
                crate::metadata::types::DataType::Date => ArrowDataType::Date32,
                crate::metadata::types::DataType::DateTime
                | crate::metadata::types::DataType::Timestamp => {
                    ArrowDataType::Timestamp(arrow::datatypes::TimeUnit::Millisecond, None)
                }
                crate::metadata::types::DataType::Boolean => ArrowDataType::Boolean,
                _ => ArrowDataType::Utf8, // Default
            };

            Field::new(&col.name, arrow_type, col.nullable)
        })
        .collect();

    ArrowSchema::new(fields)
}
```

### Step 3: Wire to Query Executor (1 hour)

**Update**: `src/query/executor.rs`

```rust
use crate::planner::datafusion::DataFusionPlanner;
use datafusion::arrow::record_batch::RecordBatch;
use futures::stream::StreamExt;

// Add to QueryExecutor
pub struct QueryExecutor {
    queue: Arc<QueryQueue>,
    datafusion: Arc<DataFusionPlanner>,
}

impl QueryExecutor {
    pub async fn new(max_queue_size: usize, max_concurrent: usize) -> Self {
        Self {
            queue: Arc::new(QueryQueue::new(max_queue_size, max_concurrent)),
            datafusion: Arc::new(DataFusionPlanner::new().await),
        }
    }

    async fn execute_select(
        &self,
        query: QueuedQuery,
        _be_client_pool: &Arc<BackendClientPool>,
    ) -> Result<QueryResult> {
        info!("Executing SELECT via DataFusion: {}", query.query);

        // Use DataFusion to execute
        match self.datafusion.execute_query(&query.query).await {
            Ok(df) => {
                // Collect results
                let batches: Vec<RecordBatch> = df.collect().await
                    .map_err(|e| DorisError::QueryExecution(format!("DataFusion error: {}", e)))?;

                // Convert Arrow batches to MySQL result format
                self.arrow_to_mysql_result(batches)
            }
            Err(e) => {
                error!("DataFusion execution failed: {}", e);
                Err(DorisError::QueryExecution(format!("Query failed: {}", e)))
            }
        }
    }

    fn arrow_to_mysql_result(&self, batches: Vec<RecordBatch>) -> Result<QueryResult> {
        if batches.is_empty() {
            return Ok(QueryResult::empty());
        }

        // Get schema from first batch
        let schema = batches[0].schema();

        // Convert to MySQL column definitions
        let columns: Vec<ColumnDefinition> = schema.fields().iter()
            .map(|field| {
                let mysql_type = arrow_to_mysql_type(field.data_type());
                ColumnDefinition::new(field.name().clone(), mysql_type)
            })
            .collect();

        // Convert rows
        let mut rows = Vec::new();
        for batch in batches {
            for row_idx in 0..batch.num_rows() {
                let mut values = Vec::new();

                for col_idx in 0..batch.num_columns() {
                    let array = batch.column(col_idx);
                    let value = if array.is_null(row_idx) {
                        None
                    } else {
                        Some(array_value_to_string(array, row_idx))
                    };
                    values.push(value);
                }

                rows.push(ResultRow::new(values));
            }
        }

        Ok(QueryResult::new_select(columns, rows))
    }
}

use arrow::datatypes::DataType as ArrowDataType;
use crate::mysql::packet::ColumnType;

fn arrow_to_mysql_type(arrow_type: &ArrowDataType) -> ColumnType {
    match arrow_type {
        ArrowDataType::Int8 | ArrowDataType::UInt8 => ColumnType::Tiny,
        ArrowDataType::Int16 | ArrowDataType::UInt16 => ColumnType::Short,
        ArrowDataType::Int32 | ArrowDataType::UInt32 => ColumnType::Long,
        ArrowDataType::Int64 | ArrowDataType::UInt64 => ColumnType::LongLong,
        ArrowDataType::Float32 => ColumnType::Float,
        ArrowDataType::Float64 => ColumnType::Double,
        ArrowDataType::Decimal128(_, _) => ColumnType::NewDecimal,
        ArrowDataType::Utf8 | ArrowDataType::LargeUtf8 => ColumnType::VarString,
        ArrowDataType::Date32 | ArrowDataType::Date64 => ColumnType::Date,
        ArrowDataType::Timestamp(_, _) => ColumnType::DateTime,
        ArrowDataType::Boolean => ColumnType::Tiny,
        _ => ColumnType::VarString,
    }
}

use arrow::array::Array;

fn array_value_to_string(array: &Arc<dyn Array>, idx: usize) -> String {
    use arrow::array::*;

    match array.data_type() {
        ArrowDataType::Int32 => {
            array.as_any().downcast_ref::<Int32Array>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Int64 => {
            array.as_any().downcast_ref::<Int64Array>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Float64 => {
            array.as_any().downcast_ref::<Float64Array>().unwrap().value(idx).to_string()
        }
        ArrowDataType::Utf8 => {
            array.as_any().downcast_ref::<StringArray>().unwrap().value(idx).to_string()
        }
        // Add more types as needed
        _ => format!("{:?}", array.slice(idx, 1)),
    }
}
```

### Step 4: Register CSV Data Source (30 min)

For testing with TPC-H data:

```rust
impl DataFusionPlanner {
    pub async fn register_tpch_csv_files(&self, data_dir: &str) -> DFResult<()> {
        let csv_options = CsvReadOptions::new()
            .delimiter(b'|')
            .has_header(false);

        // Register all TPC-H tables from .tbl files
        let tables = vec![
            "nation", "region", "part", "supplier",
            "partsupp", "customer", "orders", "lineitem"
        ];

        for table in tables {
            let path = format!("{}/{}.tbl", data_dir, table);
            self.ctx.register_csv(table, &path, csv_options.clone()).await?;
        }

        Ok(())
    }
}
```

### Step 5: Update Main (30 min)

**Update**: `src/main.rs`

```rust
mod planner;  // Add planner module

#[tokio::main]
async fn main() -> Result<()> {
    // ... existing setup ...

    // Create query executor with DataFusion
    let query_executor = Arc::new(query::QueryExecutor::new(
        config.query_queue_size,
        config.max_concurrent_queries,
    ).await);  // Note: now async

    // If TPC-H data directory is configured, register CSV files
    if let Ok(data_dir) = std::env::var("TPCH_DATA_DIR") {
        info!("Registering TPC-H data from: {}", data_dir);
        query_executor.register_tpch_csv(&data_dir).await?;
    }

    // ... rest of setup ...
}
```

### Step 6: Add Planner Module

**File**: `src/planner/mod.rs`

```rust
pub mod datafusion;

pub use datafusion::DataFusionPlanner;
```

## Testing with DataFusion

### Generate TPC-H Data
```bash
cd /tmp
git clone https://github.com/electrum/tpch-dbgen.git
cd tpch-dbgen
make
./dbgen -s 0.01  # Scale factor 0.01 = 10MB (good for testing)

# Generates: nation.tbl, region.tbl, etc.
export TPCH_DATA_DIR=/tmp/tpch-dbgen
```

### Run Rust FE
```bash
cd /home/user/doris/rust-fe

# Build with DataFusion
cargo build

# Run with TPC-H data
TPCH_DATA_DIR=/tmp/tpch-dbgen cargo run
```

### Test Queries
```bash
mysql -h 127.0.0.1 -P 9030 -u root

mysql> USE tpch;
mysql> SELECT COUNT(*) FROM lineitem;
mysql> SELECT * FROM nation;

# TPC-H Query 1
mysql> SELECT
    l_returnflag,
    l_linestatus,
    SUM(l_quantity) AS sum_qty,
    SUM(l_extendedprice) AS sum_base_price,
    COUNT(*) AS count_order
FROM lineitem
WHERE l_shipdate <= DATE '1998-12-01'
GROUP BY l_returnflag, l_linestatus
ORDER BY l_returnflag, l_linestatus;
```

## Benefits of DataFusion Approach

1. **Immediate TPC-H Support** - DataFusion already handles all TPC-H queries
2. **Battle-tested** - Used in production by many projects
3. **Optimizations** - Get query optimization for free
4. **Less Code** - Don't write planner from scratch
5. **Fast Development** - 4-6 hours instead of 20-30 hours
6. **Arrow Native** - Efficient columnar data handling

## Timeline with DataFusion

| Task | Time | Cumulative |
|------|------|------------|
| Add DataFusion dependency | 15m | 15m |
| Create DataFusion context | 30m | 45m |
| Wire to executor | 1h | 1h 45m |
| Arrow to MySQL conversion | 1h | 2h 45m |
| Register CSV data source | 30m | 3h 15m |
| Update MySQL handler | 1h | 4h 15m |
| Testing & debugging | 2h | 6h 15m |

**Total: ~6 hours** instead of 25-35 hours!

## Migration Path

### Phase 1: DataFusion Only (PoC)
- Use DataFusion for everything
- Read from CSV files
- Prove TPC-H works
- **Time: 6 hours**

### Phase 2: Hybrid (Best of Both)
- DataFusion for planning
- Convert plan to Doris BE format
- BE for execution
- **Time: +8 hours**

### Phase 3: Full Integration
- DataFusion generates optimized plans
- Send to distributed BE cluster
- Production ready
- **Time: +10 hours**

## Next Steps

1. **Add DataFusion dependency** to Cargo.toml
2. **Create DataFusionPlanner** module
3. **Wire to QueryExecutor**
4. **Test with simple queries**
5. **Generate TPC-H data with dbgen**
6. **Run full TPC-H benchmark**

This approach is **much more practical** and can have TPC-H running in **hours not weeks**!
