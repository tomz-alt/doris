# Action Plan: Get to Working TPC-H Tests

## Current Reality
- ✅ Foundation built (schema, parser, catalog, MySQL protocol)
- ❌ Execution engine NOT implemented
- ❌ Cannot run TPC-H queries yet
- ❌ No data loaded
- ❌ Not tested with real BE

## Critical Path to Success

### Step 1: Solve Protoc Issue ⚡ BLOCKER
**Time**: 1-2 hours
**Priority**: P0 (blocks everything)

**Option A: Install protoc properly**
```bash
# Try to get protoc somehow
# Check if we can download binary
# Or use system package manager
```

**Option B: Use pre-generated code** (RECOMMENDED)
```bash
# On a machine with protoc:
cd /home/user/doris/rust-fe
cargo build  # This generates Rust code in target/

# Copy generated files:
cp target/debug/build/doris-rust-fe-*/out/*.rs src/be/generated/

# Update src/be/mod.rs to include generated code directly
```

**Option C: Docker build environment**
```dockerfile
FROM rust:1.75
RUN apt-get update && apt-get install -y protobuf-compiler
WORKDIR /app
COPY . .
RUN cargo build --release
```

**Decision**: Choose ONE and proceed

---

### Step 2: Update MySQL Handler (Use Catalog)
**Time**: 1 hour
**Priority**: P1 (quick win, shows progress)

**File**: `src/mysql/connection.rs`

**Changes**:
```rust
use crate::metadata::catalog;

async fn handle_metadata_query(&mut self, query: &str) -> Result<()> {
    let catalog = catalog::catalog();
    let query_lower = query.to_lowercase();

    // SHOW DATABASES
    if query_lower.starts_with("show databases") {
        let databases = catalog.list_databases();
        let columns = vec![
            ColumnDefinition::new("Database".to_string(), ColumnType::VarString)
        ];
        let rows: Vec<ResultRow> = databases.into_iter()
            .map(|db| ResultRow::new(vec![Some(db)]))
            .collect();
        return self.send_result_set(columns, rows).await;
    }

    // USE database
    if query_lower.starts_with("use ") {
        let db_name = query_lower.strip_prefix("use ")
            .unwrap()
            .trim_end_matches(';')
            .trim();

        if catalog.database_exists(db_name) {
            self.current_db = Some(db_name.to_string());
            return self.send_ok().await;
        } else {
            return self.send_error(1049, format!("Unknown database '{}'", db_name)).await;
        }
    }

    // SHOW TABLES
    if query_lower.starts_with("show tables") {
        let db = self.current_db.as_ref().unwrap_or(&"tpch".to_string());

        match catalog.list_tables(db) {
            Ok(tables) => {
                let columns = vec![
                    ColumnDefinition::new(format!("Tables_in_{}", db), ColumnType::VarString)
                ];
                let rows: Vec<ResultRow> = tables.into_iter()
                    .map(|t| ResultRow::new(vec![Some(t)]))
                    .collect();
                return self.send_result_set(columns, rows).await;
            }
            Err(e) => {
                return self.send_error(1049, e).await;
            }
        }
    }

    // DESC/DESCRIBE table
    if query_lower.starts_with("desc ") || query_lower.starts_with("describe ") {
        let table_name = if query_lower.starts_with("desc ") {
            query_lower.strip_prefix("desc ")
        } else {
            query_lower.strip_prefix("describe ")
        }.unwrap().trim_end_matches(';').trim();

        let db = self.current_db.as_ref().unwrap_or(&"tpch".to_string());

        match catalog.get_table_columns(db, table_name) {
            Some(columns) => {
                // Create result set with column info
                let result_columns = vec![
                    ColumnDefinition::new("Field".to_string(), ColumnType::VarString),
                    ColumnDefinition::new("Type".to_string(), ColumnType::VarString),
                    ColumnDefinition::new("Null".to_string(), ColumnType::VarString),
                    ColumnDefinition::new("Key".to_string(), ColumnType::VarString),
                    ColumnDefinition::new("Default".to_string(), ColumnType::VarString),
                    ColumnDefinition::new("Extra".to_string(), ColumnType::VarString),
                ];

                let rows: Vec<ResultRow> = columns.into_iter()
                    .map(|col| {
                        ResultRow::new(vec![
                            Some(col.name.clone()),
                            Some(format!("{:?}", col.data_type)),
                            Some(if col.nullable { "YES" } else { "NO" }.to_string()),
                            Some(if col.primary_key { "PRI" } else { "" }.to_string()),
                            col.default_value.clone(),
                            Some(if col.auto_increment { "auto_increment" } else { "" }.to_string()),
                        ])
                    })
                    .collect();

                return self.send_result_set(result_columns, rows).await;
            }
            None => {
                return self.send_error(1146, format!("Table '{}.{}' doesn't exist", db, table_name)).await;
            }
        }
    }

    // Fall through to existing logic
    Ok(())
}
```

**Test**:
```bash
cargo build
cargo run

# In another terminal
mysql -h 127.0.0.1 -P 9030 -u root

mysql> SHOW DATABASES;
mysql> USE tpch;
mysql> SHOW TABLES;
mysql> DESC lineitem;
```

---

### Step 3: Implement Basic Query Planner
**Time**: 4-6 hours
**Priority**: P1 (critical for execution)

**File**: `src/planner/mod.rs` (NEW)

```rust
use sqlparser::ast::{Statement, Query, Select, SelectItem, TableFactor, Expr};
use crate::metadata::{catalog, ColumnDef};
use crate::error::{DorisError, Result};

#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub fragments: Vec<PlanFragment>,
    pub output_schema: Vec<ColumnDef>,
}

#[derive(Debug, Clone)]
pub struct PlanFragment {
    pub fragment_id: i64,
    pub plan_node: PlanNode,
}

#[derive(Debug, Clone)]
pub enum PlanNode {
    Scan {
        table: String,
        database: String,
        columns: Vec<String>,
        schema: Vec<ColumnDef>,
    },
    Filter {
        predicate: String,  // SQL expression
        input: Box<PlanNode>,
    },
    Project {
        columns: Vec<String>,
        input: Box<PlanNode>,
    },
    Limit {
        limit: i64,
        offset: i64,
        input: Box<PlanNode>,
    },
}

pub fn plan_query(stmt: &Statement, default_db: &str) -> Result<QueryPlan> {
    match stmt {
        Statement::Query(query) => plan_select_query(query, default_db),
        _ => Err(DorisError::QueryExecution("Unsupported statement type".to_string())),
    }
}

fn plan_select_query(query: &Query, default_db: &str) -> Result<QueryPlan> {
    let select = query.body.as_select()
        .ok_or_else(|| DorisError::QueryExecution("Expected SELECT".to_string()))?;

    // Build plan from bottom up
    let mut plan_node = build_scan_node(select, default_db)?;

    // Add WHERE filter
    if let Some(ref selection) = select.selection {
        plan_node = PlanNode::Filter {
            predicate: format!("{}", selection),
            input: Box::new(plan_node),
        };
    }

    // Add LIMIT
    if let Some(ref limit_expr) = query.limit {
        if let Expr::Value(sqlparser::ast::Value::Number(n, _)) = limit_expr {
            let limit = n.parse::<i64>()
                .map_err(|_| DorisError::QueryExecution("Invalid LIMIT".to_string()))?;

            let offset = if let Some(ref offset_expr) = query.offset {
                if let Expr::Value(sqlparser::ast::Value::Number(n, _)) = offset_expr {
                    n.parse::<i64>().unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            };

            plan_node = PlanNode::Limit {
                limit,
                offset,
                input: Box::new(plan_node),
            };
        }
    }

    // Get output schema
    let output_schema = extract_output_schema(&plan_node)?;

    Ok(QueryPlan {
        fragments: vec![PlanFragment {
            fragment_id: 1,
            plan_node,
        }],
        output_schema,
    })
}

fn build_scan_node(select: &Select, default_db: &str) -> Result<PlanNode> {
    if select.from.is_empty() {
        return Err(DorisError::QueryExecution("No FROM clause".to_string()));
    }

    let table_with_joins = &select.from[0];

    if let TableFactor::Table { name, .. } = &table_with_joins.relation {
        let table_name = name.to_string();
        let parts: Vec<&str> = table_name.split('.').collect();

        let (db, table) = match parts.len() {
            1 => (default_db, parts[0]),
            2 => (parts[0], parts[1]),
            _ => return Err(DorisError::QueryExecution("Invalid table name".to_string())),
        };

        let catalog = catalog::catalog();
        let table_schema = catalog.get_table(db, table)
            .ok_or_else(|| DorisError::QueryExecution(format!("Table {} not found", table)))?;

        // Extract columns to scan
        let columns = extract_scan_columns(select, &table_schema.columns)?;

        Ok(PlanNode::Scan {
            table: table.to_string(),
            database: db.to_string(),
            columns,
            schema: table_schema.columns.clone(),
        })
    } else {
        Err(DorisError::QueryExecution("Complex FROM clause not supported yet".to_string()))
    }
}

fn extract_scan_columns(select: &Select, schema: &[ColumnDef]) -> Result<Vec<String>> {
    let mut columns = Vec::new();

    for item in &select.projection {
        match item {
            SelectItem::Wildcard(_) => {
                // SELECT * - return all columns
                return Ok(schema.iter().map(|c| c.name.clone()).collect());
            }
            SelectItem::UnnamedExpr(Expr::Identifier(ident)) => {
                columns.push(ident.value.clone());
            }
            SelectItem::ExprWithAlias { expr, alias } => {
                if let Expr::Identifier(ident) = expr {
                    columns.push(ident.value.clone());
                }
            }
            _ => {
                // For complex expressions, we'll need all columns
                return Ok(schema.iter().map(|c| c.name.clone()).collect());
            }
        }
    }

    Ok(columns)
}

fn extract_output_schema(node: &PlanNode) -> Result<Vec<ColumnDef>> {
    match node {
        PlanNode::Scan { schema, .. } => Ok(schema.clone()),
        PlanNode::Filter { input, .. } => extract_output_schema(input),
        PlanNode::Project { input, .. } => extract_output_schema(input),
        PlanNode::Limit { input, .. } => extract_output_schema(input),
    }
}
```

---

### Step 4: Wire Execution Pipeline
**Time**: 2-3 hours
**Priority**: P1

**File**: `src/query/executor.rs`

Add to beginning of file:
```rust
use crate::parser;
use crate::planner;
use sqlparser::ast::Statement;
```

Update `execute_internal`:
```rust
async fn execute_internal(
    &self,
    query: QueuedQuery,
    be_client_pool: &Arc<BackendClientPool>,
) -> Result<QueryResult> {
    debug!("Executing query: {}", query.query);

    // Parse SQL
    let statements = parser::parse_sql(&query.query)?;

    if statements.is_empty() {
        return Err(DorisError::QueryExecution("Empty query".to_string()));
    }

    let stmt = &statements[0];

    // Validate against catalog
    parser::validate_statement(stmt)?;

    // Plan the query
    let db = query.database.as_deref().unwrap_or("tpch");
    let plan = planner::plan_query(stmt, db)?;

    info!("Query plan: {} fragments", plan.fragments.len());

    // Execute on BE (or mock)
    self.execute_plan(plan, be_client_pool).await
}

async fn execute_plan(
    &self,
    plan: planner::QueryPlan,
    be_client_pool: &Arc<BackendClientPool>,
) -> Result<QueryResult> {
    // For now, return mock data matching the plan's schema
    info!("Executing plan with {} fragments", plan.fragments.len());

    // Generate mock data based on schema
    let columns: Vec<ColumnDefinition> = plan.output_schema.iter()
        .map(|col| {
            let mysql_type = col.data_type.to_mysql_type();
            ColumnDefinition {
                catalog: "def".to_string(),
                schema: "".to_string(),
                table: "".to_string(),
                org_table: "".to_string(),
                name: col.name.clone(),
                org_name: col.name.clone(),
                character_set: 33,
                column_length: col.data_type.default_length(),
                column_type: mysql_type,
                flags: 0,
                decimals: 0,
            }
        })
        .collect();

    // Generate a few mock rows
    let rows = vec![
        ResultRow::new(vec![Some("mock_value_1".to_string()); columns.len()]),
        ResultRow::new(vec![Some("mock_value_2".to_string()); columns.len()]),
    ];

    Ok(QueryResult::new_select(columns, rows))
}
```

---

### Step 5: Test Pipeline
**Time**: 1 hour

```bash
# Build
SKIP_PROTO=1 cargo build

# Run
SKIP_PROTO=1 cargo run

# Test in another terminal
mysql -h 127.0.0.1 -P 9030 -u root

mysql> USE tpch;
mysql> SELECT * FROM lineitem LIMIT 10;
# Should return mock data now!

mysql> SELECT l_orderkey, l_partkey FROM lineitem LIMIT 5;
# Should work
```

---

### Step 6: Setup Real BE
**Time**: 2-4 hours

```bash
# Build Doris BE
cd /home/user/doris
./build.sh --be

# Start BE
cd output/be
./bin/start_be.sh

# Check BE is running
curl http://localhost:8040/api/health
```

---

### Step 7: Connect FE to Real BE
**Time**: 2-3 hours

**Prerequisites**: protoc working

Update `fe_config.json`:
```json
{
  "mysql_port": 9030,
  "http_port": 8030,
  "backend_nodes": [
    {
      "host": "127.0.0.1",
      "port": 9060,
      "grpc_port": 9070
    }
  ]
}
```

Implement real BE execution in `src/be/client.rs`

---

### Step 8: Load TPC-H Data
**Time**: 2-4 hours

```bash
# Generate data
cd /tmp
git clone https://github.com/electrum/tpch-dbgen.git
cd tpch-dbgen
make
./dbgen -s 1  # Scale factor 1 = 1GB

# This generates:
# nation.tbl, region.tbl, part.tbl, supplier.tbl,
# partsupp.tbl, customer.tbl, orders.tbl, lineitem.tbl
```

Implement LOAD DATA or use stream load:
```sql
LOAD DATA LOCAL INFILE '/tmp/tpch-dbgen/lineitem.tbl'
INTO TABLE tpch.lineitem
FIELDS TERMINATED BY '|';
```

---

### Step 9: Run TPC-H Queries
**Time**: 4-8 hours (testing + debugging)

```sql
-- TPC-H Query 1
SELECT
    l_returnflag,
    l_linestatus,
    SUM(l_quantity) AS sum_qty,
    SUM(l_extendedprice) AS sum_base_price,
    ...
FROM lineitem
WHERE l_shipdate <= DATE '1998-12-01'
GROUP BY l_returnflag, l_linestatus
ORDER BY l_returnflag, l_linestatus;
```

Debug issues, fix bugs, iterate.

---

## Summary Timeline

| Phase | Task | Time | Cumulative |
|-------|------|------|------------|
| 1 | Fix protoc | 1-2h | 2h |
| 2 | Update MySQL handler | 1h | 3h |
| 3 | Implement planner | 4-6h | 9h |
| 4 | Wire pipeline | 2-3h | 12h |
| 5 | Test pipeline | 1h | 13h |
| 6 | Setup BE | 2-4h | 17h |
| 7 | Connect to BE | 2-3h | 20h |
| 8 | Load data | 2-4h | 24h |
| 9 | Run TPC-H | 4-8h | 32h |

**Realistic total**: **25-35 hours** of focused development

## What to Do Right Now

**Choose one**:

### Option A: Quick Demo (Metadata Only)
- Fix Step 2 only (1 hour)
- Shows: SHOW DATABASES, SHOW TABLES, DESC working
- Good for presentation

### Option B: Functional but Incomplete (Mock Data)
- Complete Steps 1-5 (10-15 hours)
- Shows: Queries parse, plan, return mock data
- Good for architecture demo

### Option C: Full TPC-H Support
- Complete all steps (25-35 hours)
- Shows: Real TPC-H queries on real data
- Production-ready proof

**Recommendation**: Start with Option A, then decide based on results.
