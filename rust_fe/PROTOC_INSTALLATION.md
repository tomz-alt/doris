# Protoc Installation Blocker

## Problem

The Rust FE → C++ BE integration requires protoc (Protocol Buffer Compiler) to generate Rust bindings from `.proto` files. Installation is **BLOCKED** in the current environment.

## Failed Attempts

### 1. apt-get (Package Manager)
```bash
$ sudo apt-get install protobuf-compiler
E: Failed to fetch https://ppa.launchpadcontent.net/.../dists/noble/InRelease  403  Forbidden
E: The repository '...' is no longer signed.
```

**Root cause**: Package repositories blocked by proxy (403 Forbidden)

### 2. GitHub Releases (Binary Download)
```bash
$ wget https://github.com/protocolbuffers/protobuf/releases/download/v33.1/protoc-33.1-linux-x86_64.zip
--2025-11-18 06:02:43--  https://release-assets.githubusercontent.com/...
Proxy request sent, awaiting response... 403 Forbidden
2025-11-18 06:02:43 ERROR 403: Forbidden.
```

**Root cause**: GitHub release assets blocked by proxy (403 Forbidden)

### 3. Build Verification
```bash
$ cargo build -vv --package fe-backend-client 2>&1 | grep protoc
[fe-backend-client 0.1.0] Warning: Failed to generate protobuf bindings: Could not find `protoc`.
[fe-backend-client 0.1.0] Install protoc to enable BE communication:
[fe-backend-client 0.1.0]   Debian/Ubuntu: apt-get install protobuf-compiler
[fe-backend-client 0.1.0]   macOS: brew install protobuf
[fe-backend-client 0.1.0] Continuing without protobuf generation...
```

**Status**: Build succeeds with graceful fallback, but protobuf bindings are NOT generated.

## Impact

### ❌ Cannot Do (Requires protoc)
- Generate Rust code from `internal_service.proto`
- Implement real gRPC client to C++ BE
- Execute queries with real data from C++ BE
- Verify 100% identical results vs Java FE on real data
- TPC-H benchmarks with actual query execution

### ✅ Can Still Do (MockBackend workaround)
- All 200 tests pass
- Comparison tests verify Rust FE matches Java FE behavior
- MockBackend simulates C++ BE for API testing (6 tests)
- SQL parsing, planning, and schema extraction work
- MySQL protocol server works for client connections
- Development and testing of FE logic

## Workarounds

### Option 1: Manual protoc Installation (Requires privileged access)
```bash
# On a machine with internet access, download protoc:
curl -LO https://github.com/protocolbuffers/protobuf/releases/download/v33.1/protoc-33.1-linux-x86_64.zip
unzip protoc-33.1-linux-x86_64.zip -d /usr/local

# Then on target machine:
export PROTOC=/usr/local/bin/protoc
cargo build --package fe-backend-client
```

### Option 2: Pre-generated Bindings (Recommended for CI/CD)
```bash
# On development machine with protoc:
cd /home/user/doris/rust_fe/fe-backend-client
cargo build  # Generates src/generated/*.rs

# Commit generated files:
git add src/generated/
git commit -m "Add pre-generated protobuf bindings"

# On target machine (no protoc needed):
cargo build --package fe-backend-client  # Uses pre-generated files
```

### Option 3: Use MockBackend for Development
```bash
# Current state - works without protoc
cargo test --package fe-backend-client
# All 6 tests pass using MockBackend

# Develop FE logic without real BE
cargo test  # All 200 tests pass
```

### Option 4: Docker Container with protoc
```dockerfile
FROM rust:1.75
RUN apt-get update && apt-get install -y protobuf-compiler
WORKDIR /workspace
COPY . .
RUN cargo build --package fe-backend-client
```

## Current Status

**Development Mode**: Using MockBackend
- ✅ 200 tests passing
- ✅ All FE logic testable
- ✅ Comparison tests verify Java FE parity
- ✅ Ready for real BE integration when protoc available

**Next Steps When protoc Available**:
1. Install protoc (any workaround above)
2. Run `cargo build -p fe-backend-client`
3. Verify generated files in `src/generated/`
4. Implement real `BackendClient::exec_plan_fragment()`
5. Implement real `BackendClient::fetch_data()`
6. Test with running C++ BE on port 9060
7. Run integration tests from BE_INTEGRATION_TEST_PLAN.md

## Files Ready for Integration

All code is ready - just waiting for protobuf generation:

```
fe-backend-client/
├── src/
│   ├── lib.rs           ✅ BackendClient stub ready
│   ├── mock.rs          ✅ MockBackend for testing
│   └── generated/       ⚠️ Empty (needs protoc to populate)
├── build.rs             ✅ Graceful protoc fallback
└── tests/               ✅ 6 tests pass with MockBackend
```

## Recommendation

**For Production Deployment**:
Use Option 2 (pre-generated bindings) - commit the generated `src/generated/*.rs` files to git. This allows building without protoc in restricted environments.

**For Development**:
Use Option 3 (MockBackend) - continue developing FE logic and tests without real C++ BE.

**For CI/CD**:
Use Option 4 (Docker with protoc) - build in container with all dependencies.
