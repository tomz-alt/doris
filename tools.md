# Bazel Tooling Guide for Doris

This document captures viable tool use experiences and best practices for the Bazel migration.

---

## Essential Bazel Commands

### Building

```bash
# Build everything
bazel build //...

# Build backend only
bazel build //be:doris_be

# Build with specific configuration
bazel build --config=release //be:doris_be

# Build with remote cache
bazel build --remote_cache=grpc://cache-server:9092 //...

# Build specific target
bazel build //be/src/olap:olap_lib
```

### Testing

```bash
# Run all tests
bazel test //...

# Run backend tests only
bazel test //be/...

# Run specific test
bazel test //be/src/util:util_test

# Run tests with output
bazel test --test_output=all //be/src/util:util_test

# Run tests with specific filter
bazel test --test_filter=MyTestCase.MyTest //be/...
```

### Querying

```bash
# Find all dependencies of a target
bazel query 'deps(//be:doris_be)'

# Find reverse dependencies (what depends on X)
bazel query 'rdeps(//..., //be/src/util:util_lib)'

# Find all cc_library targets
bazel query 'kind(cc_library, //...)'

# Find path between two targets
bazel query 'somepath(//be:doris_be, //be/src/util:util_lib)'

# Visualize dependency graph
bazel query 'deps(//be:doris_be)' --output=graph > graph.dot
dot -Tpng graph.dot -o graph.png
```

### Cleaning

```bash
# Clean build outputs
bazel clean

# Clean everything including repository cache
bazel clean --expunge

# Clean and remove analysis cache
bazel clean --expunge_async
```

### Performance Analysis

```bash
# Generate build profile
bazel build --profile=profile.json //...

# Analyze profile (requires bazel analyze-profile)
bazel analyze-profile profile.json

# Show build time breakdown
bazel build --experimental_profile_include_target_label //...
```

---

## IDE Integration

### CLion / IntelliJ IDEA

**Install Bazel Plugin**:
1. Settings → Plugins → Search "Bazel"
2. Install "Bazel" by Google
3. Restart IDE

**Configure Project**:
1. File → Import Bazel Project
2. Select workspace root (/home/user/doris)
3. Import project view (or create new)

**Sample .bazelproject file**:
```
directories:
  be/src
  fe/src

targets:
  //be:doris_be
  //be/...

derive_targets_from_directories: true

additional_languages:
  c
  c++
  java
```

**Generate compile_commands.json** (for other IDEs):
```bash
# Using bazel-compile-commands-extractor
bazel run @hedron_compile_commands//:refresh_all
```

### VS Code

**Install Extensions**:
- Bazel (by The Bazel Authors)
- C/C++ (by Microsoft)
- clangd (recommended over C/C++)

**Configure .vscode/settings.json**:
```json
{
  "bazel.executable": "bazel",
  "bazel.buildifierExecutable": "buildifier",
  "clangd.arguments": [
    "--compile-commands-dir=${workspaceFolder}",
    "--background-index",
    "--clang-tidy"
  ]
}
```

---

## Common Patterns

### Creating a cc_library

```python
# be/src/util/BUILD.bazel
cc_library(
    name = "util_lib",
    srcs = glob([
        "*.cpp",
    ], exclude = [
        "*_test.cpp",
    ]),
    hdrs = glob([
        "*.h",
    ]),
    deps = [
        "//be/src/common:common_lib",
        "@com_google_glog//:glog",
    ],
    copts = [
        "-std=c++17",
        "-Wall",
        "-Wextra",
    ],
    visibility = ["//be:__subpackages__"],
)
```

### Creating a cc_test

```python
cc_test(
    name = "util_test",
    srcs = glob([
        "*_test.cpp",
    ]),
    deps = [
        ":util_lib",
        "@com_google_googletest//:gtest",
        "@com_google_googletest//:gtest_main",
    ],
    data = [
        "//be/test/data:test_files",
    ],
    copts = [
        "-std=c++17",
    ],
)
```

### Creating a cc_binary

```python
cc_binary(
    name = "doris_be",
    srcs = [
        "main.cpp",
    ],
    deps = [
        "//be/src/service:service_lib",
        "//be/src/runtime:runtime_lib",
        "//be/src/olap:olap_lib",
        # ... all other deps
    ],
    linkopts = [
        "-lpthread",
        "-ldl",
        "-lrt",
    ] + select({
        "//bazel/config:use_jemalloc": ["-ljemalloc"],
        "//conditions:default": [],
    }),
    visibility = ["//visibility:public"],
)
```

### Platform-Specific Configuration

```python
# BUILD.bazel
config_setting(
    name = "linux_x86_64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:x86_64",
    ],
)

config_setting(
    name = "linux_aarch64",
    constraint_values = [
        "@platforms//os:linux",
        "@platforms//cpu:aarch64",
    ],
)

cc_library(
    name = "platform_lib",
    srcs = select({
        ":linux_x86_64": ["x86_64_impl.cpp"],
        ":linux_aarch64": ["aarch64_impl.cpp"],
    }),
    copts = select({
        ":linux_x86_64": ["-mavx2"],
        ":linux_aarch64": ["-march=armv8-a+crc"],
    }),
)
```

### Third-party cc_import

```python
# bazel/third_party/BUILD.bazel
cc_import(
    name = "glog",
    hdrs = glob(["installed/include/glog/**/*.h"]),
    shared_library = "installed/lib/libglog.so",
    visibility = ["//visibility:public"],
)

cc_import(
    name = "protobuf",
    hdrs = glob(["installed/include/google/protobuf/**/*.h"]),
    static_library = "installed/lib/libprotobuf.a",
    visibility = ["//visibility:public"],
)
```

### Proto Library

```python
# gensrc/proto/BUILD.bazel
load("@rules_proto//proto:defs.bzl", "proto_library")
load("@com_google_protobuf//:protobuf.bzl", "cc_proto_library")

proto_library(
    name = "doris_proto",
    srcs = glob(["*.proto"]),
    deps = [
        "@com_google_protobuf//:any_proto",
    ],
    visibility = ["//visibility:public"],
)

cc_proto_library(
    name = "doris_cc_proto",
    deps = [":doris_proto"],
    visibility = ["//visibility:public"],
)
```

### Genrule for Custom Generation

```python
genrule(
    name = "gen_version",
    srcs = ["version.sh"],
    outs = ["version.h"],
    cmd = "$(location version.sh) > $@",
    visibility = ["//be:__subpackages__"],
)
```

---

## Configuration Files

### .bazelrc

Essential configuration for the Doris project:

```bash
# Common build options
build --cxxopt=-std=c++17
build --host_cxxopt=-std=c++17
build --copt=-Wall
build --copt=-Wextra

# Platform-specific
build:linux --copt=-DLINUX
build:macos --copt=-DMACOS

# Optimization levels
build:debug --compilation_mode=dbg
build:release --compilation_mode=opt
build:release --strip=always

# Remote caching
build:remote --remote_cache=grpc://localhost:9092
build:remote --remote_upload_local_results=true

# Performance
build --jobs=auto
build --local_ram_resources=HOST_RAM*.8
build --experimental_worker_max_memory=4096

# Testing
test --test_output=errors
test --test_summary=detailed

# C++ specific
build --cxxopt=-Wno-deprecated-declarations
build --cxxopt=-Wno-unused-parameter

# Java specific (when we migrate FE)
build --java_language_version=11
build --java_runtime_version=11
build --tool_java_language_version=11
build --tool_java_runtime_version=11

# Disk cache
build --disk_cache=~/.cache/bazel
```

### WORKSPACE.bazel

```python
workspace(name = "doris")

# Load http_archive rule
load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

# C++ Rules
http_archive(
    name = "rules_cc",
    urls = ["https://github.com/bazelbuild/rules_cc/releases/download/0.0.9/rules_cc-0.0.9.tar.gz"],
    sha256 = "2037875b9a4456dce4a79d112a8ae885bbc4aad968e6587dca6e64f3a0900cdf",
    strip_prefix = "rules_cc-0.0.9",
)

# Proto Rules
http_archive(
    name = "rules_proto",
    sha256 = "66bfdf8782796239d3875d37e7de19b1d94301e8972b3cbd2446b332429b4df1",
    strip_prefix = "rules_proto-4.0.0",
    urls = [
        "https://github.com/bazelbuild/rules_proto/archive/refs/tags/4.0.0.tar.gz",
    ],
)
load("@rules_proto//proto:repositories.bzl", "rules_proto_dependencies", "rules_proto_toolchains")
rules_proto_dependencies()
rules_proto_toolchains()

# Google Test
http_archive(
    name = "com_google_googletest",
    urls = ["https://github.com/google/googletest/archive/refs/tags/release-1.12.1.tar.gz"],
    strip_prefix = "googletest-release-1.12.1",
    sha256 = "81964fe578e9bd7c94dfdb09c8e4d6e6759e19967e397dbea48d1c10e45d0df2",
)

# Protobuf
http_archive(
    name = "com_google_protobuf",
    urls = ["https://github.com/protocolbuffers/protobuf/releases/download/v21.11/protobuf-all-21.11.tar.gz"],
    strip_prefix = "protobuf-21.11",
    sha256 = "b3b104f0374802e1add5d5d7a5a845ac",
)

# Third-party local repository (for existing third-party builds)
local_repository(
    name = "doris_thirdparty",
    path = "thirdparty",
)
```

---

## Debugging Tips

### Build Failures

```bash
# Verbose build output
bazel build --verbose_failures //be:doris_be

# Show all compilation commands
bazel build --subcommands //be:doris_be

# Show why a target is being rebuilt
bazel build --explain=explain.log //be:doris_be
```

### Dependency Issues

```bash
# Find what's pulling in a dependency
bazel query 'somepath(//be:doris_be, @problematic_dep//...)'

# Show all external dependencies
bazel query 'kind(http_archive, //external:*)'

# Check for duplicate dependencies
bazel query 'filter("^@", deps(//be:doris_be))' --output=build
```

### Cache Debugging

```bash
# Disable all caching
bazel build --noremote_accept_cached --noremote_upload_local_results //...

# Check cache stats
bazel info | grep cache
```

---

## Performance Optimization

### Remote Caching Setup

**Using Bazel Remote Cache (bazel-remote)**:

```bash
# Run cache server
docker run -v /path/to/cache:/data \
  -p 9092:9092 \
  buchgr/bazel-remote-cache:latest \
  --max_size 50G

# Configure in .bazelrc
build --remote_cache=grpc://localhost:9092
build --remote_upload_local_results=true
```

### Build Without the Bytes (BwoB)

```bash
# Download only what's needed
build --remote_download_minimal

# Or download only outputs
build --remote_download_outputs=minimal
```

### Parallel Execution Tuning

```bash
# Increase parallel jobs
build --jobs=50

# Limit RAM usage
build --local_ram_resources=HOST_RAM*0.8

# Limit CPU usage
build --local_cpu_resources=HOST_CPUS*0.8
```

---

## Migration Helpers

### Comparing Build Outputs

```bash
# Build with CMake
mkdir cmake_build && cd cmake_build
cmake ../be
make -j$(nproc)
cd ..

# Build with Bazel
bazel build //be:doris_be

# Compare binaries
diff <(objdump -d cmake_build/be/doris_be) \
     <(objdump -d bazel-bin/be/doris_be)
```

### Extracting Compile Flags from CMake

```bash
# Generate compile_commands.json with CMake
cmake -DCMAKE_EXPORT_COMPILE_COMMANDS=ON ../be

# Extract flags for conversion to Bazel
cat compile_commands.json | jq '.[0].command'
```

---

## Troubleshooting

### Common Issues

**Issue**: "Target not found"
```bash
# Solution: Check BUILD.bazel file exists and target is defined
bazel query //path/to:target
```

**Issue**: "Undeclared inclusion(s)"
```bash
# Solution: Add missing dependencies or hdrs
# Use --verbose_failures to see which headers are missing
```

**Issue**: "Multiple definition of symbol"
```bash
# Solution: Check for duplicate sources or libraries in deps
bazel query 'somepath(//be:doris_be, //problematic:lib)'
```

**Issue**: Slow builds
```bash
# Solution: Enable remote caching, increase --jobs, profile build
bazel build --profile=profile.json //...
bazel analyze-profile profile.json
```

---

## Best Practices

1. **Keep BUILD files near sources**: Each directory should have its own BUILD.bazel
2. **Use fine-grained targets**: Create separate libraries for logical components
3. **Minimize visibility**: Use `visibility` to control dependency access
4. **Leverage glob carefully**: Exclude tests and benchmarks from main libraries
5. **Use select() for platform differences**: Keep platform logic declarative
6. **Document complex rules**: Add comments for non-obvious configurations
7. **Test incrementally**: Ensure each new target builds and tests pass
8. **Profile regularly**: Use --profile to catch performance regressions

---

**Last Updated**: 2025-11-20
