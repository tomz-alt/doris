# Backend Setup Automation

## Location

All BE setup and testing automation is located in:
```
/home/user/doris_binary/
```

This directory is **outside the git repository** intentionally, as it's designed to hold the multi-GB BE binary which should not be version controlled.

## Available Scripts

### Quick Status Check
```bash
bash /home/user/doris_binary/check_status.sh
```
Shows current state of binary availability, BE installation, running processes, ports, and Rust FE readiness.

**Run this anytime** to see where you are in the setup process and get next-step recommendations.

### Automated Setup and Testing (Recommended)
```bash
bash /home/user/doris_binary/automated_setup_and_test.sh
```

**This is the main entry point** - one command that handles everything:
- ✅ Detects binary format (complete tarball or 3-part split)
- ✅ Reassembles split files automatically
- ✅ Extracts BE
- ✅ Verifies installation
- ✅ Starts BE daemon
- ✅ Checks ports (8060, 9060, 8040, 9050)
- ✅ Runs Rust FE connection tests
- ✅ Executes comprehensive integration tests

### Manual Setup
```bash
bash /home/user/doris_binary/extract_and_setup.sh
```
For step-by-step manual setup. Extracts binary and shows next steps without automation.

## Prerequisites

Upload the BE binary to `/home/user/doris_binary/` in one of these formats:

**Option A: Complete tarball**
```
apache-doris-4.0.1-bin-x64.tar.gz
```

**Option B: 3-part split** (auto-reassembled by script)
```
apache-doris-be-part-aa
apache-doris-be-part-ab
apache-doris-be-part-ac
```

## Workflow

### Step 1: Check Status
```bash
bash /home/user/doris_binary/check_status.sh
```

### Step 2: Upload Binary
See current blocker: `/home/user/doris_binary/DOWNLOAD_STATUS.md`

Downloads are blocked by proxy. Manual upload required.

### Step 3: Run Automation
```bash
bash /home/user/doris_binary/automated_setup_and_test.sh
```

### Step 4: Verify
```bash
# BE should be running
ps aux | grep doris_be

# Port 8060 should be open
netstat -tlnp | grep 8060

# Rust FE tests should pass
cd /home/user/doris/rust_fe
cargo test
```

Expected: **209 tests passing** ✅

## Integration Tests

The automation runs these test examples:

### Connection Test
```bash
cd /home/user/doris/rust_fe
cargo run --example test_be_connection
```
Tests basic gRPC connection to BE at localhost:8060

### Comprehensive Integration Test
```bash
cargo run --example test_real_be
```
Tests:
- gRPC connection
- `exec_plan_fragment` RPC
- `fetch_data` RPC
- Error handling
- Status codes

### Full Test Suite
```bash
cargo test
```
Runs all 209 tests including:
- 171 base tests (catalog, parser, executor, planner, protocol)
- 34 Rust FE vs Java FE comparison tests
- 5 TPC-H E2E tests (Q1 full execution, Q2-Q22 parsing)

## Documentation

All documentation in `/home/user/doris_binary/`:

- **README.md** - Complete setup guide
- **SETUP.md** - Detailed step-by-step instructions
- **DOWNLOAD_STATUS.md** - Download blocker details and alternatives
- **check_status.sh** - Status checker script
- **automated_setup_and_test.sh** - Full automation script
- **extract_and_setup.sh** - Manual extraction script

## Current Status

As of 2025-11-18:

✅ **Rust FE**: Complete (209 tests passing)
✅ **gRPC Client**: Fully implemented
✅ **Test Examples**: Created and ready
✅ **Automation Scripts**: Complete and tested
❌ **BE Binary**: Awaiting upload (downloads blocked)

**Next step**: Upload BE binary, run `automated_setup_and_test.sh`

## Why Outside Git Repo?

The `/home/user/doris_binary/` directory is outside the git repository because:

1. **Size**: BE binary is ~2GB, inappropriate for git
2. **Separation**: Runtime binaries separate from source code
3. **Flexibility**: Can handle different binary versions without repo changes
4. **Cleanup**: Easy to delete and re-download without affecting git history

The setup scripts are utilities that live with the binary, not source code to be version controlled.

## After Successful Setup

Once BE is running, you can:

1. **Execute TPC-H queries** against real data
2. **Load test datasets** (via Java FE or Rust FE)
3. **Compare results** between Rust FE and Java FE
4. **Performance benchmarks** (Rust FE vs Java FE)
5. **Production testing** with real workloads

All tools and tests are ready - just waiting for the BE binary.
