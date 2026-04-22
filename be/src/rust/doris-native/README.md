# doris-native: Rust Native Readers for Apache Doris

This workspace contains the Rust-based format readers for Doris BE, starting with Lance support.

## Architecture

```
C++ (Doris BE)                    Rust (doris-native)
┌─────────────────┐               ┌──────────────────┐
│ LanceRustReader  │──JSON config─>│ lance_reader_open │
│ (GenericReader)  │               │ lance_reader_next │──> lance-rs
│                  │<─Arrow C ABI──│ lance_reader_close│    Dataset::scan()
└─────────────────┘               └──────────────────┘
```

Data exchange uses the Arrow C Data Interface (zero-copy between Rust and C++).
Each reader owns a single-threaded tokio runtime (`block_on()` on the scanner thread).

## Prerequisites

- Rust stable toolchain (see `rust-toolchain.toml`)
- For BE integration: `BUILD_RUST_READERS=ON` in CMake

## Quick Start

### Run Rust tests

```bash
cd be/src/rust/doris-native
cargo test
```

Expected output: 24 tests passing (error handling, lance reader, FFI bridge).

### Build release library

```bash
cargo build --release
# Output: target/release/libdoris_ffi.a (linked into doris_be)
```

### Build with Doris BE

```bash
# From repo root:
export DORIS_HOME=$PWD
export DORIS_THIRDPARTY=/path/to/thirdparty
export BUILD_RUST_READERS=ON

# Via build.sh:
./build.sh --be

# Or via cmake directly:
cd be/build_Release
cmake -DBUILD_RUST_READERS=ON ...
make -j$(nproc) doris_be
```

## Crate Structure

```
doris-native/
├── Cargo.toml                    # Workspace root
├── rust-toolchain.toml           # Rust version pin
└── crates/
    └── doris-ffi/                # Static library linked into doris_be
        ├── Cargo.toml
        └── src/
            ├── lib.rs            # Module root + rust_echo FFI
            ├── error.rs          # Thread-local error handling (FFI_OK, FFI_ERR_*)
            ├── lance_reader.rs   # LanceReader + LanceReaderConfig
            └── ffi.rs            # extern "C" functions (lance_reader_open, etc.)
```

## FFI Functions

| Function | Purpose |
|----------|---------|
| `lance_reader_open(uri, columns, batch_size, handle_out)` | Open dataset (simple API) |
| `lance_reader_open_json(config_json, len, handle_out)` | Open with full config (S3 creds, version, vector search) |
| `lance_reader_next_batch(handle, schema, array, eof, bytes)` | Read next Arrow batch |
| `lance_reader_get_schema(handle, schema_out)` | Get dataset schema |
| `lance_reader_close(handle)` | Free resources |
| `lance_reader_last_error(buf, len)` | Get error message |
| `lance_test_create_dataset(path, len)` | Create 5-row test dataset |
| `lance_test_create_multi_fragment_dataset(path, len)` | Create 15-row, 3-fragment test dataset |

## JSON Config

The `lance_reader_open_json` accepts a JSON config string:

```json
{
  "uri": "s3://bucket/data.lance",
  "columns": ["id", "name"],
  "batch_size": 4096,
  "version": 0,
  "storage_options": {
    "AWS_ACCESS_KEY_ID": "...",
    "AWS_SECRET_ACCESS_KEY": "..."
  },
  "filter": "category = 'shoes'",
  "vector_search": {
    "column": "embedding",
    "query": [0.1, 0.2, 0.3],
    "k": 10,
    "metric": "cosine",
    "nprobes": 20,
    "ef": 100
  },
  "full_text_search": "machine learning",
  "limit": 100,
  "offset": 0,
  "fragment_ids": [0, 1, 2]
}
```

## Running E2E Tests

### Standalone C++ test (no Doris cluster needed)

```bash
# Build test binary:
RUST_LIB=be/src/rust/doris-native/target/release/libdoris_ffi.a
ARROW_LIB=/path/to/thirdparty/installed/lib64
clang++ -std=c++20 -O2 \
  -I/path/to/thirdparty/installed/include \
  be/test/format/lance/standalone_lance_test.cpp \
  $RUST_LIB -Wl,--start-group $ARROW_LIB/libarrow.a ... -Wl,--end-group \
  -lpthread -ldl -lm -lrt -o lance_test

./lance_test
# All 8 tests PASSED!
```

### Live Doris cluster test

```bash
# 1. Create test datasets on BE:
./lance_create single /opt/apache-doris/be/lance_test_data/single.lance
./lance_create multi  /opt/apache-doris/be/lance_test_data/multi.lance

# 2. Query via MySQL client:
mysql -h 127.0.0.1 -P 9030 -u root -e "
SELECT * FROM local(
    \"file_path\" = \"lance_test_data/single.lance/data/\",
    \"backend_id\" = \"<BE_ID>\",
    \"format\" = \"lance\"
) ORDER BY id;"

# Expected:
# id  name    score
# 1   alice   90.5
# 2   bob     85.0
# 3   carol   92.3
# 4   dave    78.1
# 5   eve     88.7
```

### Regression test

```bash
# Run the lance TVF regression test suite:
./run-regression-test.sh --run -s test_lance_tvf

# Run by file:
./run-regression-test.sh --run \
  -f regression-test/suites/external_table_p0/tvf/lance/test_lance_tvf.groovy

# Generate expected output (first time):
./run-regression-test.sh --run -s test_lance_tvf -genOut
```

## Test Summary

| Layer | Tests | What's verified |
|-------|-------|----------------|
| Rust unit (24) | `cargo test` | Error handling, LanceReader open/read/close, FFI lifecycle, JSON config |
| C++ standalone (8) | `lance_test` binary | FFI bridge, Arrow import, schema inference, data verification, multi-fragment |
| Live cluster (8) | MySQL queries | Full TVF: SELECT *, projection, COUNT, WHERE, LIMIT, multi-fragment, aggregation |
| Regression (9) | `test_lance_tvf.groovy` | Automated CI-ready version of live cluster tests |

## CI build: cross-compile to old glibc with cargo-zigbuild

The production `libdoris_ffi.a` vendors AWS-LC (via `aws-lc-rs`), which on a
modern host (glibc ≥ 2.38) pulls in C23 symbol variants such as
`__isoc23_sscanf` / `__isoc23_strtol`. Doris BE links statically against the
ldb-toolchain runtime (glibc 2.17–2.27), which does not expose those names,
so a native `cargo build` on Ubuntu 24.04 produces a `.a` the BE cannot link.

CI avoids this by **cross-compiling the Rust side against glibc 2.17** with
[`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild) *before* the
BE C++ build runs. CMake auto-detects the resulting `.a` and turns
`BUILD_RUST_READERS` on automatically — no extra flag needed.

### Host prerequisites (CI image)

| Tool | Version used | Install |
|------|--------------|---------|
| Rust (stable) | 1.95+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh -s -- -y --default-toolchain stable --profile minimal` |
| Zig | 0.13.0 | `wget https://ziglang.org/download/0.13.0/zig-linux-x86_64-0.13.0.tar.xz && tar -xf zig-*.tar.xz -C /usr/local/` then `export PATH=/usr/local/zig-linux-x86_64-0.13.0:$PATH` |
| cargo-zigbuild | latest | `cargo install --locked cargo-zigbuild` |
| protoc | from thirdparty | Already in `/var/local/thirdparty/installed/bin/protoc` |
| libc6-dev | host glibc headers | `apt-get install -y libc6-dev` |
| sccache (optional) | latest | `cargo install sccache` — set `SCCACHE_DIR=/tmp/sccache` |

### The pre-C++ Rust build step

```bash
# Run this before build.sh --be in the CI pipeline.
export PATH=$HOME/.cargo/bin:/usr/local/zig-linux-x86_64-0.13.0:$PATH
export TMPDIR=/tmp

cd "${DORIS_HOME}/be/src/rust/doris-native"
cargo zigbuild --target x86_64-unknown-linux-gnu.2.17 --release

# Output (~500 MB, static archive, glibc-2.17-safe):
ls -lh target/x86_64-unknown-linux-gnu/release/libdoris_ffi.a
```

The `.2.17` suffix on the target tells zig to emit only symbols available in
glibc 2.17. `build.rs` detects the low-glibc target and compiles a small
stubs translation unit (`glibc_compat.c`) to patch up the handful of
functions (`fcntl64`, `statx`, ...) that AWS-LC expects.

### How CMake picks it up

`be/CMakeLists.txt` auto-enables `BUILD_RUST_READERS` when it finds a
pre-built archive at either of:

```
be/src/rust/doris-native/target/x86_64-unknown-linux-gnu/release/libdoris_ffi.a   ← zigbuild output
be/src/rust/doris-native/target/release/libdoris_ffi.a                            ← native cargo output
```

So the CI flow is:

```bash
# 1. Pre-step: build the Rust archive against old glibc
cd be/src/rust/doris-native
cargo zigbuild --target x86_64-unknown-linux-gnu.2.17 --release
cd -

# 2. Normal BE build — CMake auto-detects the .a and links it in
./build.sh --be -j "$(nproc)"
```

### Running the regression test on CI

After the build, the groovy suite `test_lance_tvf` uploads the dataset
fixtures under `regression-test/data/external_table_p0/tvf/lance/` to the
BE's filesystem via `scpFiles` and issues the TVF queries. It gates the
feature on the session variable:

```sql
SET enable_rust_lance_reader = true;
```

Invoke it directly with:

```bash
./run-regression-test.sh --run -s test_lance_tvf
```

The suite covers SELECT *, column projection, COUNT, WHERE, LIMIT, the
multi-fragment scan path, and a negative case on a non-existent path.

### Troubleshooting

**`ld.lld: error: undefined symbol: __isoc23_sscanf`** — the `.a` was built
with native cargo against modern glibc. Rebuild with
`cargo zigbuild --target x86_64-unknown-linux-gnu.2.17 --release`.

**`mold: error: undefined symbol: log` / `pow`** — the system `mold` linker
(≤2.30) mis-handles libm resolution across the static-archive group.
Force LLD: `EXTRA_CXX_FLAGS=-fuse-ld=lld` or hide mold from PATH so CMake
falls through to lld.

**`CMake Error: Unable to resolve full path of PCH-header`** — stale
`build_Release/CMakeCache.txt` after toggling `BUILD_RUST_READERS`. Wipe
the build directory: `rm -rf be/build_Release` and retry.
