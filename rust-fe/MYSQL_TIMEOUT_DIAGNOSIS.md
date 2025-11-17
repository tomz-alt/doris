# MySQL Protocol Timeout Diagnosis

## ✅ ISSUE RESOLVED

**Root Cause**: Build configuration error - using `--no-default-features` disables critical features.

**Solution**: Always build with default features:
```bash
# ✅ CORRECT
cargo build --features real_be_proto

# ❌ WRONG
cargo build --no-default-features --features real_be_proto
```

---

## Original Problem Statement (RESOLVED)
Rust FE server times out when handling SELECT queries via MySQL protocol, even though handshake and metadata queries work.

## Evidence
- @@version_comment query succeeds → basic packet I/O works
- SELECT queries timeout → something hangs during query execution
- Timeout occurs in `read_packet()` waiting for next command → suggests previous command didn't complete properly

## AGENTS.md Alignment

### Principle #2: Use Java FE as specification
Need to compare with Java FE's MySQL protocol handler to understand correct flow.

### Principle #4: Observability
Current logging is insufficient to diagnose where execution stalls.

## Proposed Fixes (Ordered by Priority)

### Fix 1: Add Comprehensive Debug Logging (IMMEDIATE)
**Location**: `src/mysql/connection.rs`

Add logging at critical points:
```rust
// In handle_query(), line 261:
info!("Starting query execution: {}", query);
match self.query_executor.execute_sql(&mut self.session, &query, &self.be_client_pool).await {
    Ok(result) => {
        info!("Query execution completed, rows: {}, cols: {}", result.rows.len(), result.columns.len());
        self.send_query_result(result).await?;
        info!("Query result sent successfully");
    }
    Err(e) => {
        error!("Query execution failed: {}", e);
        self.send_error(1064, format!("Query failed: {}", e)).await?;
    }
}
info!("handle_query() completed successfully");
```

**Why**: This will show exactly where the code hangs (before execute_sql, during, or after).

### Fix 2: Check Query Executor Path
**Location**: `src/query/executor.rs`

The issue may be in the query execution path:
1. Check if `execute_sql()` is even being called
2. Check if it's waiting on BE client pool
3. Check if plan conversion is hanging

**Diagnostic steps**:
```bash
# Run with debug logging
RUST_LOG=debug cargo run --bin doris-rust-fe --features real_be_proto -- --config fe_config.json
```

Look for:
- "Starting query execution"
- "Query execution completed"
- If neither appears, problem is before execute_sql
- If first appears but not second, problem is in execute_sql

### Fix 3: Verify BE Client Pool State
**Location**: `src/be/pool.rs`

Check if BE connection is healthy:
- Is the client pool initialized correctly?
- Does it have available connections?
- Is the BE actually responding?

**Test**:
```bash
# Verify BE is reachable
docker exec doris-be netstat -an | grep 8060
# Should show LISTEN

# Check from Rust FE host
nc -zv 172.20.80.3 8060
# Should connect
```

### Fix 4: Add Timeout to Query Execution
**Location**: `src/mysql/connection.rs:261`

Wrap execute_sql with timeout to prevent infinite hangs:
```rust
use tokio::time::{timeout, Duration};

// In handle_query()
let query_timeout = Duration::from_secs(10); // Configurable
match timeout(query_timeout,
    self.query_executor.execute_sql(&mut self.session, &query, &self.be_client_pool)
).await {
    Ok(Ok(result)) => {
        self.send_query_result(result).await?;
    }
    Ok(Err(e)) => {
        error!("Query execution failed: {}", e);
        self.send_error(1064, format!("Query failed: {}", e)).await?;
    }
    Err(_) => {
        error!("Query execution timed out after {:?}", query_timeout);
        self.send_error(1064, "Query execution timeout".to_string()).await?;
    }
}
```

**Why**: Provides graceful degradation and clear error messages per AGENTS.md principle #4.

### Fix 5: Check Sequence ID Management
**Location**: `src/mysql/connection.rs:152`

Line 152 resets sequence_id to 1 for each command. Verify this matches MySQL protocol spec.

**Potential issue**: If result set is incomplete, client may be waiting for more packets.

**Verification**:
- Compare with Java FE's MySQL implementation
- Check MySQL protocol docs for sequence_id behavior during multi-packet result sets

### Fix 6: Verify Result Set Encoding
**Location**: `src/mysql/connection.rs:428-490`

The `send_result_set()` function encodes results. Verify:
1. Column count is correct
2. All columns are sent
3. Proper EOF/OK delimiter after columns (lines 455-465)
4. All rows are sent
5. Final EOF/OK packet is sent (lines 478-488)

**Test with working query**:
```sql
-- This works:
SELECT @@version_comment LIMIT 1

-- This should also work if result encoding is correct:
SELECT 'test' as col1

-- Then try actual table:
SELECT * FROM tpch_sf1.lineitem LIMIT 1
```

## Immediate Action Plan

1. **Add logging** (Fix 1) - Do this first
2. **Run test with debug logs** - Identify exact hang point
3. **Based on logs, apply** Fix 2, 3, or 4
4. **Document findings** in current_impl.md

## Expected Outcomes

After Fix 1, we should see one of:
- **Hangs before "Starting query execution"** → Problem in query parsing or routing
- **Hangs after "Starting" but before "completed"** → Problem in query executor (Fix 2/3)
- **"completed" but no "sent successfully"** → Problem in result encoding (Fix 6)
- **All logs appear but still timeout** → Problem in command loop logic (Fix 5)

## References
- AGENTS.md: Principle #2 (Java FE reference), #4 (Observability)
- MySQL Protocol: https://dev.mysql.com/doc/dev/mysql-server/latest/PAGE_PROTOCOL.html
- Current implementation: src/mysql/connection.rs:138-275
