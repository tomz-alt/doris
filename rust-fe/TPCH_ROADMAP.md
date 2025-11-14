# TPC-H Implementation Roadmap

## Current Progress

### ✅ Completed (Phase 1)
1. **Core Architecture**
   - MySQL protocol server (`src/mysql/`)
   - HTTP streaming load server (`src/http/`)
   - Query queue with concurrency control (`src/query/`)
   - Async Tokio runtime
   - Configuration system

2. **Dependencies Added**
   - `sqlparser = "0.43"` - For SQL parsing
   - `lazy_static = "1.4"` - For global state

3. **Metadata System**
   - Data types (`src/metadata/types.rs`)
   - Table/Column schema (`src/metadata/schema.rs`)
   - TPC-H schema builder (all 8 tables)
   - Type system with MySQL mapping

### 🔄 In Progress (Phase 2)
Need to create these files:

#### 1. Catalog Manager (`src/metadata/catalog.rs`)
```rust
// Global catalog that holds all databases
// Thread-safe access with DashMap
// Initialize with TPC-H schema
```

#### 2. SQL Parser Integration (`src/parser/mod.rs`)
```rust
// Use sqlparser-rs to parse SQL
// Convert to internal AST
// Validate against catalog
```

#### 3. Query Planner (`src/planner/mod.rs`)
```rust
// Convert AST to execution plan
// Generate plan fragments for BE
// Optimize simple queries
```

#### 4. BE Communication Layer Updates
- Update to work with/without protoc
- Mock BE responses for testing
- Real BE integration when ready

### ⏳ Pending (Phase 3)

#### 5. Complete Integration
- Wire up parser -> planner -> executor
- Handle TPC-H specific query patterns
- Result formatting

#### 6. Testing & Verification
- Unit tests for each component
- Integration tests
- TPC-H query tests

## Implementation Plan

### Step 1: Catalog Manager (Next)

Create `src/metadata/catalog.rs`:

```rust
use dashmap::DashMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref GLOBAL_CATALOG: CatalogManager = {
        let catalog = CatalogManager::new();
        // Initialize with TPC-H schema
        catalog.add_database(Database::create_tpch_schema());
        catalog
    };
}

pub struct CatalogManager {
    databases: DashMap<String, Database>,
}

impl CatalogManager {
    pub fn new() -> Self { ... }
    pub fn get_database(&self, name: &str) -> Option<...> { ... }
    pub fn get_table(&self, db: &str, table: &str) -> Option<...> { ... }
    pub fn add_database(&self, db: Database) { ... }
    pub fn list_databases(&self) -> Vec<String> { ... }
}

pub fn catalog() -> &'static CatalogManager {
    &GLOBAL_CATALOG
}
```

### Step 2: SQL Parser

Create `src/parser/mod.rs`:

```rust
use sqlparser::ast::*;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

pub fn parse_sql(sql: &str) -> Result<Vec<Statement>> {
    let dialect = GenericDialect{};
    Parser::parse_sql(&dialect, sql)
}

pub fn validate_query(stmt: &Statement, catalog: &Catalog) -> Result<()> {
    // Check that referenced tables/columns exist
    // Type checking
}
```

### Step 3: Query Planner

Create `src/planner/mod.rs`:

```rust
pub struct QueryPlan {
    pub fragments: Vec<PlanFragment>,
    pub result_schema: Vec<ColumnDef>,
}

pub struct PlanFragment {
    pub fragment_id: i64,
    pub plan_tree: PlanNode,
    pub target_be: String,
}

pub enum PlanNode {
    Scan { table: String, columns: Vec<String> },
    Filter { predicate: Expr, input: Box<PlanNode> },
    Project { columns: Vec<Expr>, input: Box<PlanNode> },
    Aggregate { ... },
    Join { ... },
}

pub fn plan_query(stmt: &Statement, catalog: &Catalog) -> Result<QueryPlan> {
    // Convert SQL AST to execution plan
    // For SELECT queries:
    // 1. Identify tables (FROM clause)
    // 2. Build scan nodes
    // 3. Add filters (WHERE clause)
    // 4. Add projections (SELECT clause)
    // 5. Add aggregations (GROUP BY)
    // 6. Add sorting (ORDER BY)
    // 7. Add limit (LIMIT)
}
```

### Step 4: Integration

Update `src/query/executor.rs`:

```rust
async fn execute_select(&self, query: QueuedQuery, ...) -> Result<QueryResult> {
    // 1. Parse SQL
    let statements = parser::parse_sql(&query.query)?;
    let stmt = &statements[0];

    // 2. Validate against catalog
    parser::validate_query(stmt, catalog())?;

    // 3. Plan query
    let plan = planner::plan_query(stmt, catalog())?;

    // 4. Execute on BE (or mock)
    let result = self.execute_plan(plan, be_client_pool).await?;

    // 5. Format results
    Ok(result)
}
```

### Step 5: TPC-H Queries

Test with TPC-H Query 1:

```sql
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
FROM
    lineitem
WHERE
    l_shipdate <= DATE '1998-12-01'
GROUP BY
    l_returnflag,
    l_linestatus
ORDER BY
    l_returnflag,
    l_linestatus;
```

## Build & Test Process

### Without BE (Mock Mode)
```bash
# Build with SKIP_PROTO
SKIP_PROTO=1 cargo build

# Run FE
cargo run

# Test with TPC-H schema
mysql -h 127.0.0.1 -P 9030 -u root

# Show TPC-H tables
USE tpch;
SHOW TABLES;

# Describe table
DESC lineitem;

# Run simple query
SELECT * FROM lineitem LIMIT 10;

# Run TPC-H Q1 (with mock data)
SELECT l_returnflag, COUNT(*) FROM lineitem GROUP BY l_returnflag;
```

### With Real BE
```bash
# Build with protoc
cargo build

# Start BE first
cd /home/user/doris
./output/be/bin/start_be.sh

# Update fe_config.json with BE address
# Start FE
cd rust-fe
cargo run

# Test with real data
mysql -h 127.0.0.1 -P 9030 -u root
```

## Estimated Timeline

- **Catalog Manager**: 30 min
- **SQL Parser Integration**: 1 hour
- **Query Planner**: 2-3 hours
- **Integration & Testing**: 1-2 hours
- **TPC-H Queries**: 1 hour

**Total**: ~6-8 hours for working TPC-H support

## Current Blockers

1. **protoc** - Not available, using SKIP_PROTO fallback
   - **Impact**: Can't connect to real BE yet
   - **Workaround**: Mock BE responses for now

2. **Bash environment** - Some instability
   - **Impact**: Build commands failing
   - **Workaround**: Use cargo commands directly

## Next Actions

1. ✅ Create catalog manager
2. ✅ Create SQL parser integration
3. ✅ Create query planner
4. ⏳ Wire everything together
5. ⏳ Test with mock data
6. ⏳ Connect to real BE (when protoc available)
7. ⏳ Load TPC-H data
8. ⏳ Run full TPC-H benchmark

## Files Created So Far

```
rust-fe/
├── src/
│   ├── metadata/
│   │   ├── mod.rs          ✅ Created
│   │   ├── types.rs        ✅ Created
│   │   ├── schema.rs       ✅ Created (with TPC-H schema)
│   │   └── catalog.rs      ⏳ Next
│   ├── parser/
│   │   └── mod.rs          ⏳ To create
│   └── planner/
│       ├── mod.rs          ⏳ To create
│       └── fragment.rs     ⏳ To create
├── Cargo.toml              ✅ Updated (added sqlparser)
├── BUILD_STATUS.md         ✅ Created
└── TPCH_ROADMAP.md         ✅ This file
```

## Success Criteria

The implementation will be considered successful when:

1. ✅ TPC-H schema is defined and available
2. ⏳ Can connect via MySQL protocol
3. ⏳ Can execute `SHOW DATABASES` and see "tpch"
4. ⏳ Can execute `USE tpch` and `SHOW TABLES`
5. ⏳ Can execute `DESC lineitem` and see all columns
6. ⏳ Can execute simple `SELECT * FROM lineitem LIMIT 10`
7. ⏳ Can execute TPC-H Query 1 (with aggregations)
8. ⏳ Results match expected format
9. ⏳ Can run all 22 TPC-H queries
10. ⏳ Performance is reasonable (for PoC)

Let me know if you want me to proceed with implementing the remaining components!
