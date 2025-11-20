# Doris Build System Migration: CMake to Bazel

## Migration Strategy

This document tracks the gradual migration from CMake to Bazel for comprehensive and ultrafast building experiences.

---

## Phase 1: Analysis & Foundation (Current Phase)

### 1.1 Current Build System Analysis ✅ COMPLETED

**Current State:**
- **Orchestrator**: `build.sh` - monolithic shell script that coordinates all builds
- **Backend**: CMake-based (be/CMakeLists.txt) - handles C++ compilation
- **Frontend**: Maven-based (fe/pom.xml) - handles Java compilation
- **UI**: npm-based - handles JavaScript/TypeScript compilation
- **Generated Sources**: Makefile-based (gensrc/Makefile) - proto/thrift/script generation
- **Third-party**: Bash scripts (thirdparty/vars.sh) - source-based compilation with patches

**Key Issues:**
- No cross-component incrementalism
- No distributed caching
- Long rebuild times for small changes
- Difficult dependency tracking
- Limited parallelism across components

### 1.2 Documentation Setup 🔄 IN PROGRESS

- [x] Create todos.md for build system migration tracking
- [x] Create tools.md for Bazel tooling experiences
- [x] Create CLAUDE.md for task tracking
- [ ] Document current build times and bottlenecks
- [ ] Create migration risk assessment

---

## Phase 2: Bazel Foundation

### 2.1 Initial Workspace Setup ⏳ PENDING

- [ ] Create root WORKSPACE.bazel file
- [ ] Create .bazelrc with platform-specific configurations
- [ ] Create root BUILD.bazel file
- [ ] Configure Bazel version and toolchains
- [ ] Set up hermetic C++ toolchain
- [ ] Set up hermetic Java toolchain
- [ ] Configure remote caching (optional)
- [ ] Configure build event service (optional)

### 2.2 Tooling Integration ⏳ PENDING

- [ ] Configure compile_commands.json generation for IDE support
- [ ] Set up clang-format integration
- [ ] Set up clang-tidy integration
- [ ] Configure test output formatting
- [ ] Set up build performance monitoring

---

## Phase 3: Backend (BE) Migration - PRIORITY

### 3.1 Third-party Dependencies ⏳ PENDING

**Strategy**: Import existing third-party installations rather than rebuilding

- [ ] Create bazel/third_party/BUILD.bazel
- [ ] Map cc_import rules for each library:
  - [ ] libevent
  - [ ] openssl
  - [ ] thrift
  - [ ] protobuf
  - [ ] gflags
  - [ ] glog
  - [ ] gtest/gmock
  - [ ] snappy
  - [ ] lz4
  - [ ] zlib
  - [ ] zstd
  - [ ] rocksdb
  - [ ] arrow
  - [ ] brpc
  - [ ] bzip2
  - [ ] boost
  - [ ] (add others as discovered)
- [ ] Create filegroup rules for headers
- [ ] Test third-party imports with simple test binary

### 3.2 Generated Sources (gensrc) ⏳ PENDING

**Strategy**: Convert Makefile steps to Bazel genrule/proto_library/thrift_library

- [ ] Analyze gensrc/script dependencies
- [ ] Analyze gensrc/proto dependencies
- [ ] Analyze gensrc/thrift dependencies
- [ ] Create proto_library rules for all .proto files
- [ ] Create genrule for thrift generation
- [ ] Create genrule for script-generated sources
- [ ] Verify generated headers match CMake output
- [ ] Create visibility rules for generated headers

### 3.3 Core Backend Sources ⏳ PENDING

**Strategy**: Create cc_library targets mirroring the CMake library structure

**High Priority Libraries** (foundational):
- [ ] be/src/glibc-compatibility - cc_library
- [ ] be/src/common - cc_library
- [ ] be/src/util - cc_library (break into smaller libs)
- [ ] be/src/runtime - cc_library
- [ ] be/src/exprs - cc_library

**Medium Priority Libraries**:
- [ ] be/src/olap - cc_library
- [ ] be/src/exec - cc_library
- [ ] be/src/vec - cc_library (vectorized execution)
- [ ] be/src/io - cc_library

**Backend Binaries**:
- [ ] be/src/service/doris_be - cc_binary (main backend binary)
- [ ] be/src/tools - cc_binary (various tools)

**Platform-Specific**:
- [ ] Add select() statements for x86_64 vs aarch64
- [ ] Add select() statements for Linux vs macOS
- [ ] Handle AVX2 instruction set flags
- [ ] Handle ARM architecture flags (armv8-a+crc)

**Compiler Options**:
- [ ] Port CMAKE_CXX_FLAGS to Bazel copts
- [ ] Port preprocessor definitions (-D flags)
- [ ] Port include directories
- [ ] Port linker flags (USE_LIBCPP, USE_JEMALLOC, etc.)
- [ ] Handle PCH (precompiled headers) if enabled

### 3.4 Backend Testing ⏳ PENDING

- [ ] Create cc_test rules for unit tests
- [ ] Configure googletest integration
- [ ] Set up test data dependencies
- [ ] Configure test environment variables
- [ ] Create test suites for different components
- [ ] Validate test coverage matches CMake build

---

## Phase 4: Frontend (FE) Integration

### 4.1 Maven Wrapper Approach ⏳ PENDING

**Strategy**: Keep Maven for now, wrap with Bazel genrule

- [ ] Create bazel/java/BUILD.bazel
- [ ] Create genrule that invokes Maven build
- [ ] Export JAR artifacts as filegroup
- [ ] Test Maven wrapper with clean build
- [ ] Test incremental Maven builds

### 4.2 Native Bazel Java (Future) ⏳ PENDING

**Note**: Only pursue if Maven wrapper proves insufficient

- [ ] Analyze fe/pom.xml dependencies
- [ ] Create java_library rules for fe-common
- [ ] Create java_library rules for fe-core
- [ ] Port Maven dependency resolution
- [ ] Handle shading and packaging
- [ ] Handle Maven plugins

---

## Phase 5: UI Integration

### 5.1 npm Wrapper Approach ⏳ PENDING

- [ ] Create bazel/ui/BUILD.bazel
- [ ] Create genrule for npm build
- [ ] Export UI artifacts
- [ ] Test UI build integration

---

## Phase 6: Integration & Testing

### 6.1 End-to-End Build ⏳ PENDING

- [ ] Create //:all target that builds everything
- [ ] Create //:backend target for BE-only builds
- [ ] Create //:frontend target for FE-only builds
- [ ] Test clean build from scratch
- [ ] Test incremental build (change one .cc file)
- [ ] Test incremental build (change one .java file)
- [ ] Benchmark build times vs CMake

### 6.2 Packaging & Distribution ⏳ PENDING

- [ ] Create pkg_tar rules for output/ directory structure
- [ ] Replicate build.sh packaging logic
- [ ] Create release build configuration
- [ ] Test packaging with stripped debug info
- [ ] Create installer targets

### 6.3 CI/CD Integration ⏳ PENDING

- [ ] Update GitHub Actions to use Bazel
- [ ] Configure remote caching for CI
- [ ] Set up build result monitoring
- [ ] Update documentation

---

## Phase 7: Optimization & Refinement

### 7.1 Performance Tuning ⏳ PENDING

- [ ] Enable remote caching
- [ ] Enable remote execution (if applicable)
- [ ] Tune parallel build settings
- [ ] Optimize dependency graph (reduce over-linking)
- [ ] Enable Bazel's experimental features as appropriate

### 7.2 Developer Experience ⏳ PENDING

- [ ] Create helper scripts (bazel-build-be, etc.)
- [ ] Document common Bazel commands
- [ ] Create IDE integration guides
- [ ] Set up query commands for debugging builds
- [ ] Create troubleshooting guide

---

## Phase 8: Deprecation of CMake (Future)

### 8.1 Transition Period ⏳ PENDING

- [ ] Run Bazel and CMake builds in parallel
- [ ] Validate output binary equivalence
- [ ] Migrate all developers to Bazel
- [ ] Migrate CI/CD fully to Bazel
- [ ] Update all documentation

### 8.2 CMake Removal ⏳ PENDING

- [ ] Remove CMakeLists.txt files
- [ ] Remove build.sh script
- [ ] Archive CMake documentation
- [ ] Celebrate! 🎉

---

## Known Risks & Mitigation

### Risk 1: Third-party Complexity
**Mitigation**: Start with cc_import of existing installations, don't rebuild from source initially

### Risk 2: Generated Sources
**Mitigation**: Keep exact parity with current gensrc output, validate with diff

### Risk 3: Maven/FE Migration
**Mitigation**: Use wrapper approach initially, defer native Bazel Java rules

### Risk 4: Toolchain Compatibility
**Mitigation**: Use hermetic toolchains, test on all target platforms early

### Risk 5: Build Time Regression
**Mitigation**: Benchmark continuously, use remote caching, optimize dependency graph

---

## Success Metrics

- [ ] Clean build time < 50% of CMake clean build
- [ ] Incremental build time < 25% of CMake incremental build
- [ ] Remote cache hit rate > 80%
- [ ] Developer satisfaction score > 4/5
- [ ] CI/CD build time reduction > 40%

---

## Resources

- Bazel Documentation: https://bazel.build/
- Bazel C++ Rules: https://bazel.build/reference/be/c-cpp
- Bazel Java Rules: https://bazel.build/reference/be/java
- Bazel Best Practices: https://bazel.build/basics/best-practices

---

**Last Updated**: 2025-11-20
**Status**: Phase 1 (Analysis & Foundation)
