# Proving Rust FE ↔ Doris BE Integration with TPC-H

## Goal
Demonstrate Rust FE successfully queries TPC-H data stored in Doris BE.

## Current Status

**✅ What Works:**
- MySQL protocol (Rust FE ↔ Client)
- gRPC client (Rust FE → BE)
- BE accepts gRPC connections
- DataFusion local queries

**❌ What's Missing:**
- **Metadata sync**: Rust FE can't discover BE tables
- Rust FE has isolated DataFusion catalog
- No query routing to BE (all queries go to DataFusion)

## Implementation Plan

### Phase 1: Metadata Synchronization (Critical)

**Problem**: Rust FE doesn't know about tables stored in BE.

**Solution**: Implement metadata fetch from BE on startup/demand.

#### Step 1.1: Add Metadata Service Client

```rust
// src/metadata/be_catalog.rs

use tonic::Request;
use crate::proto::internal_service::pbackend_service_client::PBackendServiceClient;
use crate::proto::types::{PGetTableMetaRequest, PGetTableMetaResponse};

pub struct BECatalog {
    be_client: PBackendServiceClient<tonic::transport::Channel>,
}

impl BECatalog {
    pub async fn fetch_databases(&self) -> Result<Vec<String>> {
        // Call BE to list databases
        let request = Request::new(PGetDatabasesRequest {});
        let response = self.be_client.get_databases(request).await?;
        Ok(response.into_inner().databases)
    }

    pub async fn fetch_tables(&self, database: &str) -> Result<Vec<TableMeta>> {
        // Call BE to list tables in database
        let request = Request::new(PGetTablesRequest {
            database: database.to_string(),
        });
        let response = self.be_client.get_tables(request).await?;

        // Convert BE table metadata to DataFusion schema
        let tables = response.into_inner().tables
            .into_iter()
            .map(|t| self.convert_table_meta(t))
            .collect();

        Ok(tables)
    }

    fn convert_table_meta(&self, be_table: PTableMeta) -> TableMeta {
        // Convert Doris column types to DataFusion types
        let schema = Schema::new(
            be_table.columns.into_iter().map(|col| {
                Field::new(
                    &col.name,
                    self.doris_type_to_arrow(&col.type_),
                    col.nullable,
                )
            }).collect()
        );

        TableMeta {
            name: be_table.name,
            schema: Arc::new(schema),
            location: TableLocation::BE(be_table.tablet_info),
        }
    }

    fn doris_type_to_arrow(&self, doris_type: &str) -> DataType {
        match doris_type {
            "INT" => DataType::Int32,
            "BIGINT" => DataType::Int64,
            "VARCHAR" => DataType::Utf8,
            "DOUBLE" => DataType::Float64,
            "DATE" => DataType::Date32,
            "DATETIME" => DataType::Timestamp(TimeUnit::Microsecond, None),
            "DECIMAL" => DataType::Decimal128(18, 2),
            _ => DataType::Utf8, // fallback
        }
    }
}
```

#### Step 1.2: Sync Metadata to DataFusion Catalog

```rust
// src/catalog/hybrid_catalog.rs

pub struct HybridCatalog {
    datafusion_catalog: Arc<MemoryCatalogProvider>,
    be_catalog: Arc<BECatalog>,
}

impl HybridCatalog {
    pub async fn sync_from_be(&self) -> Result<()> {
        info!("Syncing metadata from BE...");

        // Fetch all databases from BE
        let databases = self.be_catalog.fetch_databases().await?;

        for db_name in databases {
            // Create schema provider for this database
            let schema = Arc::new(MemorySchemaProvider::new());

            // Fetch all tables in this database
            let tables = self.be_catalog.fetch_tables(&db_name).await?;

            for table in tables {
                // Register BE-backed table in DataFusion catalog
                let be_table = Arc::new(BETableProvider::new(
                    table.schema.clone(),
                    table.location,
                    self.be_catalog.clone(),
                ));

                schema.register_table(
                    table.name.clone(),
                    be_table,
                )?;

                info!("Registered BE table: {}.{}", db_name, table.name);
            }

            // Register schema in catalog
            self.datafusion_catalog.register_schema(&db_name, schema)?;
        }

        info!("Metadata sync complete");
        Ok(())
    }
}
```

#### Step 1.3: BE-Backed Table Provider

```rust
// src/execution/be_table.rs

pub struct BETableProvider {
    schema: SchemaRef,
    location: TableLocation,
    be_catalog: Arc<BECatalog>,
}

#[async_trait]
impl TableProvider for BETableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        state: &SessionState,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Return execution plan that queries BE
        Ok(Arc::new(BEScanExec::new(
            self.schema.clone(),
            self.location.clone(),
            projection.cloned(),
            filters.to_vec(),
            limit,
            self.be_catalog.clone(),
        )))
    }
}
```

#### Step 1.4: BE Scan Executor

```rust
// src/execution/be_scan.rs

pub struct BEScanExec {
    schema: SchemaRef,
    location: TableLocation,
    projection: Option<Vec<usize>>,
    filters: Vec<Expr>,
    limit: Option<usize>,
    be_catalog: Arc<BECatalog>,
}

impl ExecutionPlan for BEScanExec {
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream> {
        // Build SQL query for BE
        let sql = self.build_be_query();

        // Execute via BE client (gRPC)
        let stream = self.be_catalog.execute_query(sql)?;

        Ok(stream)
    }

    fn build_be_query(&self) -> String {
        // Convert DataFusion plan to Doris SQL
        let mut sql = format!("SELECT ");

        // Projection
        if let Some(proj) = &self.projection {
            let cols: Vec<_> = proj.iter()
                .map(|i| self.schema.field(*i).name())
                .collect();
            sql.push_str(&cols.join(", "));
        } else {
            sql.push('*');
        }

        // FROM
        sql.push_str(&format!(" FROM {}", self.location.table_name()));

        // WHERE (filters)
        if !self.filters.is_empty() {
            sql.push_str(" WHERE ");
            // Convert filters to SQL
            let filter_sql = self.filters_to_sql(&self.filters);
            sql.push_str(&filter_sql);
        }

        // LIMIT
        if let Some(lim) = self.limit {
            sql.push_str(&format!(" LIMIT {}", lim));
        }

        sql
    }
}
```

### Phase 2: Load TPC-H Data

#### Step 2.1: Use Official Doris Tools (via Java FE)

```bash
# Clone Doris tools
cd /tmp
git clone https://github.com/apache/doris.git
cd doris/tools/tpch-tools

# Configure for Java FE
cat > conf/doris-cluster.conf <<EOF
export FE_HOST='127.0.0.1'
export FE_HTTP_PORT='8030'      # Java FE
export FE_QUERY_PORT='9030'     # Java FE
export USER='root'
export PASSWORD=''
export DB='tpch_sf1'
EOF

# Generate data (SF1 = 1GB)
./bin/build-tpch-dbgen.sh
./bin/gen-tpch-data.sh -s 1

# Create tables and load data (via Java FE → BE)
./bin/create-tpch-tables.sh -s 1
./bin/load-tpch-data.sh
```

**Result**: TPC-H tables (`lineitem`, `orders`, `customer`, etc.) stored in BE.

#### Step 2.2: Verify Data in BE

```bash
# Connect to Java FE, verify data
mysql -h 127.0.0.1 -P 9030 -u root

mysql> USE tpch_sf1;
mysql> SHOW TABLES;
+--------------------+
| Tables_in_tpch_sf1 |
+--------------------+
| customer           |
| lineitem           |
| nation             |
| orders             |
| part               |
| partsupp           |
| region             |
| supplier           |
+--------------------+

mysql> SELECT COUNT(*) FROM lineitem;
+----------+
| count(*) |
+----------+
|  6001215 | -- ~6M rows for SF1
+----------+

mysql> SELECT COUNT(*) FROM orders;
+----------+
| count(*) |
+----------+
|  1500000 | -- 1.5M rows
+----------+
```

### Phase 3: Sync Metadata to Rust FE

#### Step 3.1: Add Sync Command to Rust FE

```rust
// src/bin/rust-fe.rs

#[tokio::main]
async fn main() -> Result<()> {
    // ... existing setup ...

    // Initialize BE catalog
    let be_catalog = Arc::new(BECatalog::new(
        be_client.clone(),
    ));

    // Initialize hybrid catalog
    let catalog = Arc::new(HybridCatalog::new(
        datafusion_catalog,
        be_catalog,
    ));

    // Sync metadata from BE on startup
    info!("Syncing metadata from Doris BE...");
    catalog.sync_from_be().await?;
    info!("Metadata sync complete. Rust FE ready.");

    // ... start MySQL server ...
}
```

#### Step 3.2: Verify Sync

```bash
# Start Rust FE (with metadata sync)
cargo run --bin rust-fe

# Expected output:
# INFO Syncing metadata from BE...
# INFO Registered BE table: tpch_sf1.customer
# INFO Registered BE table: tpch_sf1.lineitem
# INFO Registered BE table: tpch_sf1.nation
# INFO Registered BE table: tpch_sf1.orders
# INFO Registered BE table: tpch_sf1.part
# INFO Registered BE table: tpch_sf1.partsupp
# INFO Registered BE table: tpch_sf1.region
# INFO Registered BE table: tpch_sf1.supplier
# INFO Metadata sync complete. Rust FE ready.
```

### Phase 4: Run TPC-H Queries via Rust FE

#### Step 4.1: Simple Query Test

```bash
# Connect to Rust FE
mysql -h 127.0.0.1 -P 9031 -u root

mysql> USE tpch_sf1;
Database changed

mysql> SHOW TABLES;
+--------------------+
| Tables_in_tpch_sf1 |
+--------------------+
| customer           | <-- From BE!
| lineitem           |
| nation             |
| orders             |
| part               |
| partsupp           |
| region             |
| supplier           |
+--------------------+

mysql> SELECT COUNT(*) FROM lineitem;
+----------+
| count(*) |
+----------+
|  6001215 | <-- Queried from BE via Rust FE!
+----------+
```

#### Step 4.2: TPC-H Q1 (Pricing Summary Report)

```sql
-- Via Rust FE (port 9031)
SELECT
    l_returnflag,
    l_linestatus,
    SUM(l_quantity) AS sum_qty,
    SUM(l_extendedprice) AS sum_base_price,
    SUM(l_extendedprice * (1 - l_discount)) AS sum_disc_price,
    SUM(l_extendedprice * (1 - l_discount) * (1 + l_tax)) AS sum_charge,
    AVG(l_quantity) AS avg_qty,
    AVG(l_extendedprice) AS avg_price,
    AVG(l_discount) AS avg_disc,
    COUNT(*) AS count_order
FROM lineitem
WHERE l_shipdate <= DATE '1998-12-01' - INTERVAL '90' DAY
GROUP BY l_returnflag, l_linestatus
ORDER BY l_returnflag, l_linestatus;
```

**Expected**: Results from BE, routed through Rust FE.

#### Step 4.3: TPC-H Q3 (Shipping Priority - Join Query)

```sql
SELECT
    l_orderkey,
    SUM(l_extendedprice * (1 - l_discount)) AS revenue,
    o_orderdate,
    o_shippriority
FROM customer, orders, lineitem
WHERE c_mktsegment = 'BUILDING'
    AND c_custkey = o_custkey
    AND l_orderkey = o_orderkey
    AND o_orderdate < DATE '1995-03-15'
    AND l_shipdate > DATE '1995-03-15'
GROUP BY l_orderkey, o_orderdate, o_shippriority
ORDER BY revenue DESC, o_orderdate
LIMIT 10;
```

**Expected**: 3-way join executed in BE, results via Rust FE.

### Phase 5: Performance Comparison

#### Step 5.1: Run Same Query via Both FEs

```bash
# Java FE (baseline)
time mysql -h 127.0.0.1 -P 9030 -u root tpch_sf1 < q1.sql

# Rust FE (optimized routing)
time mysql -h 127.0.0.1 -P 9031 -u root tpch_sf1 < q1.sql
```

#### Step 5.2: Expected Results

```
Java FE:  2.34s (baseline)
Rust FE:  2.28s (similar, routing overhead minimal)

Proof: ✅ Rust FE successfully queries BE data!
```

### Phase 6: Full TPC-H Benchmark

```bash
# Run TPC-H benchmark via Rust FE
./scripts/benchmark_tpch.sh \
    --rust-host 127.0.0.1 \
    --rust-port 9031 \
    --scale 1 \
    --rounds 3

# Expected: All 22 queries execute successfully
# Data comes from BE, routed through Rust FE
```

## Success Criteria

**✅ Proof of Integration:**

1. **Metadata Sync**: Rust FE discovers BE tables ✓
2. **Query Routing**: Queries go to BE, not local DataFusion ✓
3. **Data Retrieval**: Results from BE returned to client ✓
4. **TPC-H Q1**: Aggregation query works ✓
5. **TPC-H Q3**: Join query works ✓
6. **Performance**: Comparable to Java FE ✓

## Implementation Timeline

**Day 1**: Metadata sync (Steps 1.1-1.4)
- Implement BECatalog, HybridCatalog
- Sync metadata on startup

**Day 2**: BE query execution (Steps 1.4 continued)
- Implement BEScanExec
- SQL generation from DataFusion plan

**Day 3**: Load TPC-H + Test (Steps 2-4)
- Load data via official tools
- Test queries via Rust FE

**Day 4**: Benchmark (Steps 5-6)
- Compare Java vs Rust FE
- Run full TPC-H suite

## Alternative: Quick Prototype (Today)

If we want to prove it works **right now** without full metadata sync:

### Quick Prototype: Hardcoded TPC-H Schema

```rust
// Register TPC-H tables manually (skip metadata fetch)
pub fn register_tpch_tables(catalog: &Arc<HybridCatalog>) {
    // Hardcode lineitem schema
    let lineitem_schema = Schema::new(vec![
        Field::new("l_orderkey", DataType::Int64, false),
        Field::new("l_partkey", DataType::Int64, false),
        Field::new("l_quantity", DataType::Float64, false),
        Field::new("l_extendedprice", DataType::Float64, false),
        Field::new("l_discount", DataType::Float64, false),
        // ... all 16 columns
    ]);

    let lineitem = Arc::new(BETableProvider::new(
        Arc::new(lineitem_schema),
        TableLocation::BE("tpch_sf1.lineitem".to_string()),
        be_catalog,
    ));

    catalog.register_table("tpch_sf1", "lineitem", lineitem)?;

    // Repeat for all 8 TPC-H tables
}
```

**Pro**: Works immediately
**Con**: Hardcoded, not scalable

---

## Next Steps

**Choose approach:**

1. **Full Implementation** (4 days): Proper metadata sync, production-ready
2. **Quick Prototype** (Today): Hardcoded schemas, proves concept

**Which do you want?** I can start implementing either approach! 🚀
