# Doris Backend (BE) - Bazel Build Guide

This document describes the Bazel build setup for the Doris backend.

## Current Status

**Phase 3 - Initial Implementation** (In Progress)

### Completed ✅

- Core library BUILD files:
  - `be/src/common/BUILD.bazel` - Common utilities, configuration, base types
  - `be/src/util/BUILD.bazel` - Utility functions (compression, encoding, networking)
  - `be/test/common/BUILD.bazel` - Common library tests
  - `be/test/util/BUILD.bazel` - Util library tests
  - `be/BUILD.bazel` - Top-level backend targets

### In Progress 🔄

- Third-party library integration (requires building thirdparty dependencies first)
- Resolving circular dependencies between libraries
- Adding remaining component libraries (olap, exec, runtime, vec, io, service)

### Pending ⏳

- Generated sources integration (gensrc)
- Complete dependency graph
- doris_be binary target
- Performance benchmarks

## Prerequisites

### 1. Install Bazel

See [bazel/README.md](../bazel/README.md) for installation instructions.

### 2. Build Third-party Dependencies (REQUIRED)

The backend depends on 30+ third-party libraries that must be built first:

```bash
cd /home/user/doris/thirdparty
./build-thirdparty.sh
```

This will:
- Download and extract all third-party sources
- Apply Doris-specific patches
- Build and install to `thirdparty/installed/`

**Note**: This step takes significant time (30-60 minutes depending on CPU).

### 3. Generate Sources (REQUIRED)

Generate protobuf, thrift, and other auto-generated code:

```bash
cd /home/user/doris/gensrc
make
```

## Building the Backend

### Build Core Libraries

```bash
# Build common library
bazel build //be/src/common:common

# Build util library
bazel build //be/src/util:util

# Build I/O layer
bazel build //be/src/io:io

# Build runtime environment
bazel build //be/src/runtime:runtime

# Build OLAP storage engine
bazel build //be/src/olap:olap

# Build all backend libraries
bazel build //be:backend_libs
```

### Build with Different Configurations

```bash
# Debug build
bazel build --config=debug //be/src/common:common

# Release build with optimizations
bazel build --config=release //be/src/common:common

# Release with debug symbols (for profiling)
bazel build --config=relwithdebinfo //be/src/common:common

# Build without AVX2 (for older CPUs)
bazel build --config=noavx2 //be/src/common:common
```

### Platform-Specific Builds

```bash
# Linux x86_64 (default)
bazel build --config=linux_x86_64 //be/src/common:common

# Linux aarch64 (ARM)
bazel build --config=linux_aarch64 //be/src/common:common

# macOS
bazel build --config=macos //be/src/common:common
```

## Running Tests

### Run Specific Tests

```bash
# Run common library tests
bazel test //be/test/common:compare_test
bazel test //be/test/common:config_test

# Run util library tests
bazel test //be/test/util:bitmap_test
bazel test //be/test/util:coding_test
```

### Run All Tests

```bash
# Run all backend tests
bazel test //be/test/...

# Quick test suite (fast tests only)
bazel test //be:quick_tests
```

### Test with Verbose Output

```bash
# Show all test output
bazel test --test_output=all //be/test/common:compare_test

# Show only failing tests
bazel test --test_output=errors //be/test/...
```

### Test with Sanitizers

```bash
# Address Sanitizer (memory errors)
bazel test --config=asan //be/test/...

# Thread Sanitizer (race conditions)
bazel test --config=tsan //be/test/...

# Undefined Behavior Sanitizer
bazel test --config=ubsan //be/test/...
```

## IDE Integration

### Generate compile_commands.json

For CLion, VS Code, or any IDE that uses clangd:

```bash
bazel run @hedron_compile_commands//:refresh_all
```

This creates `compile_commands.json` at the repository root.

### CLion Configuration

1. Install Bazel plugin: Settings → Plugins → Search "Bazel"
2. Import Bazel project: File → Import Bazel Project
3. Select workspace root: `/home/user/doris`
4. Add project view targets:
   ```
   directories:
     be/src
     be/test

   targets:
     //be/...
   ```

### VS Code Configuration

1. Install extensions:
   - Bazel (by The Bazel Authors)
   - clangd

2. Configure `.vscode/settings.json`:
   ```json
   {
     "bazel.executable": "bazel",
     "clangd.arguments": [
       "--compile-commands-dir=${workspaceFolder}",
       "--background-index"
     ]
   }
   ```

## Library Structure

### Common Library (`be/src/common`)

Core utilities and configuration:
- `config.cpp/h` - Configuration management (gflags)
- `daemon.cpp/h` - Daemon process utilities
- `exception.cpp/h` - Exception handling
- `status.h` - Status/Result types
- `logging.h` - Logging infrastructure

Dependencies:
- gflags, glog, lz4
- cloud, io, util (circular - being resolved)

### Util Library (`be/src/util`)

Utility functions organized by category:
- Compression: `block_compression.cpp`, `snappy`, `lz4`, `zstd`
- Encoding: `coding.cpp`, `crc32c.cpp`, `bit_util.h`
- Networking: `brpc_client_cache.cpp`, `cidr.cpp`
- Data structures: `bitmap.cpp`, `bitmap_value.h`
- Performance: `cpu_info.cpp`, `stopwatch.h`
- Arrow integration: `util/arrow/` subdirectory
- SIMD optimizations: `util/simd/` subdirectory

Dependencies:
- gflags, glog, gtest
- snappy, lz4, zlib, zstd, bzip2
- brpc, protobuf, arrow

## Known Issues & Workarounds

### Issue 1: Circular Dependencies

**Problem**: Common depends on util, util depends on common.

**Status**: Being resolved by splitting libraries into smaller, more granular targets.

**Workaround**: Some deps commented out temporarily.

### Issue 2: Third-party Headers Not Found

**Problem**: Compilation fails with "glog/logging.h: No such file"

**Cause**: Third-party libraries not built yet.

**Solution**:
```bash
cd thirdparty && ./build-thirdparty.sh
```

### Issue 3: Generated Headers Not Found

**Problem**: Compilation fails with "gen_cpp/something.pb.h: No such file"

**Cause**: Generated sources not created yet.

**Solution**:
```bash
cd gensrc && make
```

### Issue 4: Bazel Not Installed

**Problem**: `bash: bazel: command not found`

**Solution**: Install Bazel following [bazel/README.md](../bazel/README.md)

## Migration Progress

### Phase 3: Backend Prototype (Current)

- [x] be/src/common BUILD file (13 .cpp files)
- [x] be/src/util BUILD file (71 .cpp files, 5 subpackages)
- [x] be/src/io BUILD file (48 .cpp files, fs/cache subsystems)
- [x] be/src/runtime BUILD file (73 .cpp files, 6 subpackages)
- [x] be/src/olap BUILD file (211 .cpp files, rowset/task/wal subsystems)
- [x] Test targets for common (4+ tests)
- [x] Test targets for util (4+ tests)
- [x] gensrc integration (proto/thrift generated sources)
- [ ] Resolve circular dependencies
- [ ] Validate compilation with thirdparty deps
- [ ] Add remaining libraries:
  - [ ] be/src/exec (query execution)
  - [ ] be/src/vec (vectorized execution)
  - [ ] be/src/service (RPC services)
  - [ ] be/src/exprs (expressions)
  - [ ] be/src/http (HTTP handlers)
  - [ ] be/src/geo (geospatial functions)

### Phase 4: Generated Sources

- [ ] gensrc proto integration
- [ ] gensrc thrift integration
- [ ] gensrc script integration

### Phase 5: Full Backend Build

- [ ] Complete dependency graph
- [ ] doris_be binary
- [ ] All tests passing
- [ ] Benchmark build times vs CMake

## Performance Tips

### Use Remote Caching

```bash
# Configure in .bazelrc
build --config=remote
```

### Parallel Builds

```bash
# Use all CPU cores
bazel build --jobs=auto //be/...

# Limit to specific number of jobs
bazel build --jobs=8 //be/...
```

### Incremental Builds

Bazel automatically does incremental builds. Only changed files and their dependents are rebuilt.

### Build Profiling

```bash
# Generate build profile
bazel build --profile=profile.json //be/...

# Analyze profile
bazel analyze-profile profile.json
```

## Troubleshooting

### Clean Build

```bash
# Clean build outputs
bazel clean

# Clean everything including cache
bazel clean --expunge
```

### Verbose Build

```bash
# Show all compilation commands
bazel build --subcommands //be/src/common:common

# Show why targets are being rebuilt
bazel build --explain=explain.log //be/src/common:common
```

### Query Dependencies

```bash
# Show all dependencies of a target
bazel query 'deps(//be/src/common:common)'

# Show reverse dependencies (what depends on X)
bazel query 'rdeps(//be/..., //be/src/common:common)'

# Find circular dependencies
bazel query 'somepath(//be/src/common:common, //be/src/util:util)'
```

## Next Steps

1. **Validate Setup**: Run `bazel build //bazel/test:hello_bazel` to ensure Bazel is working
2. **Build Thirdparty**: Run `cd thirdparty && ./build-thirdparty.sh`
3. **Generate Sources**: Run `cd gensrc && make`
4. **Build Common**: Run `bazel build //be/src/common:common`
5. **Run Tests**: Run `bazel test //be/test/common:compare_test`

## Resources

- Main migration docs: [../todos.md](../todos.md)
- Bazel tooling guide: [../tools.md](../tools.md)
- Session tracking: [../CLAUDE.md](../CLAUDE.md)
- Bazel documentation: https://bazel.build/

---

**Last Updated**: 2025-11-20
**Phase**: 3 (Backend Prototype)
**Status**: Initial BUILD files created, awaiting thirdparty dependencies
