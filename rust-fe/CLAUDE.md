# Instructions for Claude Sessions

## 🎯 Critical Session Start Protocol

**ALWAYS** start every new session by reading and reviewing these key tracking files:

### 1. Primary Tracking Files (READ FIRST)

```bash
# Read these files at the START of EVERY session
- todo.md              # Current tasks, blockers, and progress
- tools.md             # Working commands, known issues, diagnostics
- current_impl.md      # Implementation details and architecture
```

### 2. Supplementary Context Files (READ AS NEEDED)

```bash
# Read when relevant to current task
- SUCCESS_REPORT.md            # Fixed issues and major achievements
- TYPE_CASTING_FIX_REPORT.md   # Type system fix documentation
- TODO_CURRENT.md              # Detailed current status
- scan_ranges.json             # BE tablet metadata
- fe_config.json               # Server configuration
```

## 🏗️ Code Development Principles

**ALWAYS follow the guidelines in `AGENTS.md` when writing or modifying code:**
- Keep core boundaries clean (execute_sql + parser + catalog + planner + BE RPC)
- Use Java FE behavior as specification
- Hide transport details behind clear interfaces
- Prioritize resource control and observability

## 📝 File Maintenance Rules

### ALWAYS Keep These Files Updated

**After completing ANY significant work, update:**

1. **todo.md** - Task status and blockers (concise, current state only)
   - Move completed items to "Completed" section
   - Update "In Progress" with current status
   - Add new blockers or issues discovered

2. **tools.md** - Commands and diagnostics
   - Add new working commands
   - Update "Known Issues" section with fixes
   - Document diagnostic procedures

3. **current_impl.md** - Architecture reference (keep short, future-focused)
   - Key component locations
   - Critical implementation details
   - Java FE reference points
   - Current limitations and next steps

### When to Update

**Immediate updates required after:**
- ✅ Fixing a bug or error
- ✅ Completing a feature or milestone
- ✅ Discovering a new issue or blocker
- ✅ Adding new diagnostic commands
- ✅ Making architectural changes

**Create new report files for:**
- Major bug fixes (like `TYPE_CASTING_FIX_REPORT.md`)
- Milestone achievements (like `SUCCESS_REPORT.md`)
- Complex investigations (like `MYSQL_TIMEOUT_DIAGNOSIS.md`)

## 🔍 Session Workflow

### Start of Session

0. **Clean up background processes** (CRITICAL - prevents port conflicts)
   ```bash
   # Kill all rust-fe processes from previous sessions
   pkill -f "doris-rust-fe"
   sleep 2

   # Verify ports are free
   lsof -i :9031 -i :8031  # Should return nothing (exit code 1)
   ```
1. **Read tracking files** (todo.md, tools.md, current_impl.md)
2. **Identify current blocker** from todo.md "In Progress" section
3. **Review recent fixes** in "Completed" section
4. **Check diagnostic commands** in tools.md for debugging
5. **Ask user** if they want to continue from last blocker or switch focus

### During Work

1. **Use tools.md** as reference for build/test commands
2. **Update todo.md** as tasks progress (use TodoWrite tool)
3. **Document new findings** immediately
4. **Add diagnostic commands** to tools.md as discovered

### End of Session

1. **Update all three tracking files** (todo.md, tools.md, current_impl.md)
2. **Mark completed tasks** with checkmarks and results
3. **Document current blocker** clearly for next session
4. **Create report files** for major achievements
5. **Commit message suggestion** (if user requests git commit)

## 📂 File Organization

### Project Structure

```
rust-fe/
├── CLAUDE.md                      ← This file (session instructions)
├── todo.md                        ← ALWAYS update after work
├── tools.md                       ← ALWAYS update with commands
├── current_impl.md                ← ALWAYS update with architecture
├── SUCCESS_REPORT.md              ← Major achievements
├── TYPE_CASTING_FIX_REPORT.md    ← Specific fix documentation
├── TODO_CURRENT.md                ← Detailed status snapshot
├── scan_ranges.json               ← BE configuration
├── fe_config.json                 ← FE configuration
├── src/                           ← Source code
│   ├── be/                        ← BE communication
│   ├── catalog/                   ← Table registration
│   ├── metadata/                  ← Type system
│   ├── planner/                   ← Query planning
│   ├── query/                     ← Query execution
│   └── mysql/                     ← MySQL protocol
└── target/                        ← Build artifacts
```

### Report File Naming Convention

- `SUCCESS_REPORT.md` - Overall achievements
- `{ISSUE}_FIX_REPORT.md` - Specific bug fix (e.g., `TYPE_CASTING_FIX_REPORT.md`)
- `{FEATURE}_DIAGNOSIS.md` - Investigation docs (e.g., `MYSQL_TIMEOUT_DIAGNOSIS.md`)
- `TODO_CURRENT.md` - Snapshot of current state
- `TPCH_STATUS.md` - Phase-specific status

## 🎯 Current Project State (2025-11-17)

### Phase 1 Goal
Demonstrate Rust FE ↔ BE integration with TPC-H basic scan:
- Execute `SELECT * FROM lineitem LIMIT 3`
- Return 3 rows of actual data to MySQL client

### Recent Achievements ✅
1. **MySQL Protocol Timeout** - FIXED (don't use --no-default-features)
2. **Type Casting Error** - FIXED (populate metadata catalog from DataFusion schema)
3. **Query Execution Pipeline** - WORKING (end-to-end communication established)

### Current Blocker ⚠️
**Empty Row Batch**: BE returns `TResultBatch` with 0 rows despite scan ranges being loaded
- Scan ranges ARE being loaded: `✓ Loaded scan ranges for tpch_sf1.lineitem: 1 nodes, 3 total ranges`
- Tablet metadata verified correct (IDs match, version 3 matches)
- Fragment executes successfully on BE
- BUT: TResultBatch still has 0 rows (should have 13 rows from 3 tablets)
- **Likely Issue**: TPaloScanRange construction or encoding differs from Java FE

### Next Steps
1. **Compare Java FE vs Rust FE Thrift payloads** (procedure now in tools.md)
2. Investigate TPaloScanRange parameter differences
3. Check BE logs for errors (if accessible)
4. Verify schema_hash and other scan range parameters
5. Fix payload construction to match Java FE

## 🚀 Quick Start Commands

### Build and Run
```bash
# Build (CORRECT - with default features)
cargo build --features real_be_proto

# Run server
RUST_LOG=info cargo run --features real_be_proto -- --config fe_config.json

# Test query
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT * FROM lineitem LIMIT 3;"
```

### Debug Commands
```bash
# Check server status
lsof -i :9031

# View logs
tail -f /tmp/rust-fe-debug.log | grep -E "(ERROR|WARN|TResultBatch)"

# Kill processes
pkill -f "doris-rust-fe"
```

## ⚠️ Critical Reminders

1. **NEVER use `--no-default-features`** - it breaks query execution
2. **ALWAYS populate metadata catalog** before registering DataFusion tables
3. **ALWAYS update tracking files** after completing work
4. **ALWAYS read tracking files** at start of new session
5. **BUILD WITH**: `cargo build --features real_be_proto` (correct)
6. **DON'T BUILD WITH**: `cargo build --no-default-features --features real_be_proto` (wrong!)

## 📊 Progress Tracking

Use TodoWrite tool to track progress within a session. Update markdown files at end of session for persistence across sessions.

**Session-level tracking**: TodoWrite tool
**Cross-session tracking**: todo.md, tools.md, current_impl.md

## 🤝 User Interaction

1. **Always review tracking files** before asking user what to work on
2. **Suggest continuing from last blocker** based on todo.md
3. **Provide context** from tracking files when explaining status
4. **Ask clarifying questions** if tracking files are unclear
5. **Update files before presenting results** to user

---

**Last Updated**: 2025-11-17
**Current Focus**: Fix empty row batch issue (TResultBatch has 0 rows)
**Next Milestone**: End-to-end query returning actual data
