# Doris Bazel Build System

This directory contains Bazel-specific configuration for the Doris project migration from CMake to Bazel.

## Prerequisites

### Install Bazel

Bazel is required to use the new build system. Install it using one of these methods:

#### Using Bazelisk (Recommended)

Bazelisk automatically downloads and uses the correct Bazel version:

```bash
# Linux/macOS
npm install -g @bazel/bazelisk
# OR
wget https://github.com/bazelbuild/bazelisk/releases/download/v1.19.0/bazelisk-linux-amd64
chmod +x bazelisk-linux-amd64
sudo mv bazelisk-linux-amd64 /usr/local/bin/bazel
```

#### Using Package Managers

**Ubuntu/Debian:**
```bash
sudo apt install apt-transport-https curl gnupg
curl -fsSL https://bazel.build/bazel-release.pub.gpg | gpg --dearmor > bazel.gpg
sudo mv bazel.gpg /etc/apt/trusted.gpg.d/
echo "deb [arch=amd64] https://storage.googleapis.com/bazel-apt stable jdk1.8" | sudo tee /etc/apt/sources.list.d/bazel.list
sudo apt update && sudo apt install bazel
```

**macOS:**
```bash
brew install bazel
```

**Verify Installation:**
```bash
bazel --version
# Should show: bazel 6.x.x or higher
```

## Quick Start

### 1. Build the Test Target

Verify your Bazel setup is working:

```bash
cd /home/user/doris
bazel build //bazel/test:hello_bazel
bazel run //bazel/test:hello_bazel
```

Expected output:
```
Hello from Bazel!
Doris build system migration is working!
```

### 2. Generate compile_commands.json for IDE

For IDE support (CLion, VS Code, etc.):

```bash
bazel run @hedron_compile_commands//:refresh_all
```

This creates `compile_commands.json` at the workspace root for LSP/clangd.

### 3. Build Backend (when ready)

```bash
# Build all backend targets
bazel build //be:doris_be

# Build with release optimization
bazel build --config=release //be:doris_be

# Build specific component
bazel build //be/src/olap:olap_lib
```

### 4. Run Tests

```bash
# Run all tests
bazel test //...

# Run backend tests only
bazel test //be/...

# Run specific test
bazel test //be/src/util:util_test

# Run with verbose output
bazel test --test_output=all //be/src/util:util_test
```

## Configuration Files

- **WORKSPACE.bazel**: Defines external dependencies and repository setup
- **.bazelrc**: Configures build options, compiler flags, and optimization levels
- **BUILD.bazel**: Root build file with top-level targets
- **platforms/BUILD.bazel**: Platform definitions for cross-compilation
- **third_party/BUILD.bazel**: Third-party library imports

## Common Commands

### Building

```bash
# Build everything
bazel build //...

# Clean build
bazel clean && bazel build //...

# Build with specific config
bazel build --config=debug //be:doris_be
bazel build --config=release //be:doris_be
```

### Testing

```bash
# Run all tests
bazel test //...

# Run with coverage
bazel coverage --combined_report=lcov //be/...
```

### Querying

```bash
# Find all dependencies
bazel query 'deps(//be:doris_be)'

# Find reverse dependencies
bazel query 'rdeps(//..., //be/src/util:util_lib)'

# Visualize dependency graph
bazel query 'deps(//be:doris_be)' --output=graph > graph.dot
```

### Performance

```bash
# Build with profiling
bazel build --profile=profile.json //...
bazel analyze-profile profile.json

# Use remote cache (if configured)
bazel build --config=remote //...
```

## Build Configurations

Available in `.bazelrc`:

- `--config=debug` - Debug build with -O0 and full debug info
- `--config=release` - Release build with -O3 and stripped binaries
- `--config=relwithdebinfo` - Release with debug info for profiling
- `--config=avx2` - Enable AVX2 instructions (default on x86_64)
- `--config=noavx2` - Disable AVX2 for older CPUs
- `--config=asan` - Address Sanitizer for memory debugging
- `--config=tsan` - Thread Sanitizer for concurrency debugging
- `--config=ubsan` - Undefined Behavior Sanitizer
- `--config=remote` - Use remote caching

## Migration Status

See [../todos.md](../todos.md) for detailed migration progress.

**Current Phase**: Phase 2 - Bazel Foundation

**Completed**:
- ✅ WORKSPACE.bazel with core dependencies
- ✅ .bazelrc with compiler flags and platform configs
- ✅ Platform definitions for Linux (x86_64, aarch64) and macOS
- ✅ Third-party library stub imports
- ✅ Hello world test target

**Next Steps**:
- [ ] Validate Bazel setup with test build
- [ ] Create cc_import rules for essential third-party libraries
- [ ] Prototype BE common/util libraries
- [ ] Add generated source rules (gensrc)

## Troubleshooting

### "Target not found" error
```bash
# Check if target exists
bazel query //path/to:target
```

### Slow builds
```bash
# Enable disk cache
bazel build --disk_cache=~/.cache/bazel/doris //...

# Increase parallelism
bazel build --jobs=auto //...
```

### Include errors
```bash
# Rebuild with verbose output
bazel build --verbose_failures //...

# Show all compilation commands
bazel build --subcommands //...
```

### Cache issues
```bash
# Clear cache and rebuild
bazel clean --expunge
bazel build //...
```

## Resources

- [Doris Bazel Tools Guide](../tools.md) - Comprehensive tooling guide
- [Doris Bazel Migration TODOs](../todos.md) - Detailed migration plan
- [Bazel Documentation](https://bazel.build/)
- [Bazel C++ Tutorial](https://bazel.build/tutorials/cpp)

## Support

For issues or questions:
1. Check [../tools.md](../tools.md) for common patterns
2. Review [../todos.md](../todos.md) for known issues
3. Consult Bazel documentation: https://bazel.build/

---

**Last Updated**: 2025-11-20
