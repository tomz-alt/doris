# Circular Dependency Analysis - Doris Backend

This document analyzes the circular dependencies in the Doris backend and proposes resolution strategies for the Bazel migration.

---

## Executive Summary

The Doris backend has significant circular dependencies between its core libraries. This is a **blocking issue** for the Bazel migration because Bazel enforces acyclic dependency graphs.

**Key Findings:**
- **5 major components** involved in circular dependency chains
- **150+ source files** contribute to circular dependencies
- **Primary cycle**: `common ↔ util ↔ io ↔ runtime ↔ olap ↔ runtime`
- **Root cause**: Monolithic library design without clear layering

**Impact:**
- 🔴 **BLOCKS**: Bazel compilation will fail with circular dependency errors
- 🔴 **BLOCKS**: Cannot build any backend component until resolved
- 🟡 **AFFECTS**: Incremental build performance (unnecessary rebuilds)
- 🟡 **AFFECTS**: Test isolation and parallelization

**Recommended Approach:**
1. **Short-term**: Split libraries into smaller, layered targets (1-2 weeks)
2. **Medium-term**: Refactor headers to reduce coupling (2-4 weeks)
3. **Long-term**: Establish clear architectural layers (ongoing)

---

## Detailed Dependency Analysis

### 1. Common ↔ Util Cycle

**common → util dependencies (13 files):**
```
be/src/common/status.h
  ↳ #include "util/stack_util.h"

be/src/common/config.cpp
  ↳ #include "util/cpu_info.h"

be/src/common/daemon.cpp
  ↳ #include "util/..." (multiple util headers)

be/src/common/exception.cpp
  ↳ #include "util/stack_trace_utils.h"
```

**util → common dependencies (100+ files):**
```
be/src/util/block_compression.cpp
  ↳ #include "common/config.h"
  ↳ #include "common/factory_creator.h"

be/src/util/coding.cpp
  ↳ #include "common/status.h"

be/src/util/cpu_info.cpp
  ↳ #include "common/config.h"

... (90+ more files with similar patterns)
```

**Severity**: 🔴 **CRITICAL** - Most pervasive cycle

**Root Causes**:
- `common/status.h` is used everywhere but depends on `util/stack_util.h`
- `common/config.h` (gflags wrapper) is used everywhere but in common/
- No clear separation between "foundation" vs "utilities"

---

### 2. Common → IO/Runtime Dependencies

**common → io dependency (1 file):**
```
be/src/common/config.cpp
  ↳ #include "io/..." (IO configuration)
```

**common → runtime dependency (2 files):**
```
be/src/common/compare.h
  ↳ #include "runtime/..." (type comparisons)

be/src/common/daemon.cpp
  ↳ #include "runtime/exec_env.h"
```

**Severity**: 🟡 **MODERATE** - Limited scope but architectural violation

**Root Cause**:
- Common library should be foundation layer, but depends on higher-level subsystems
- Configuration system is too monolithic

---

### 3. Util ↔ IO Cycle

**util → io dependency (10 files):**
```
be/src/util/s3_util.cpp
  ↳ #include "io/fs/s3_file_system.h"

be/src/util/async_io.h
  ↳ #include "io/fs/file_reader.h"

be/src/util/disk_info.cpp
  ↳ #include "io/fs/local_file_system.h"
```

**io → util dependency (most io files):**
```
be/src/io/fs/local_file_reader.cpp
  ↳ #include "util/slice.h"
  ↳ #include "util/coding.h"

be/src/io/fs/s3_file_system.cpp
  ↳ #include "util/s3_util.h"  ← CIRCULAR!
```

**Severity**: 🔴 **CRITICAL** - Direct circular dependency

**Root Cause**:
- S3 utilities split between `util/s3_util.*` and `io/fs/s3_file_system.*`
- Lack of clear abstraction boundaries

---

### 4. Runtime ↔ OLAP Cycle

**runtime → olap dependency (20+ files):**
```
be/src/runtime/exec_env.cpp
  ↳ #include "olap/storage_engine.h"
  ↳ #include "olap/tablet_manager.h"

be/src/runtime/load_channel.cpp
  ↳ #include "olap/tablet.h"
  ↳ #include "olap/memtable.h"

be/src/runtime/query_context.cpp
  ↳ #include "olap/schema_cache.h"
```

**olap → runtime dependency (30+ files):**
```
be/src/olap/memtable.cpp
  ↳ #include "runtime/exec_env.h"
  ↳ #include "runtime/memory/memory_tracker.h"

be/src/olap/compaction.cpp
  ↳ #include "runtime/thread_context.h"

be/src/olap/tablet.cpp
  ↳ #include "runtime/load_channel_mgr.h"
```

**Severity**: 🔴 **CRITICAL** - Deep integration between layers

**Root Cause**:
- Runtime needs to manage OLAP storage (tablets, memtables)
- OLAP needs runtime services (memory tracking, thread context, exec env)
- Bidirectional dependency is architectural, not accidental

---

### 5. IO ↔ OLAP Cycle

**io → olap dependency (10 files):**
```
be/src/io/fs/local_file_system.cpp
  ↳ #include "olap/tablet_schema.h"

be/src/io/cache/block_file_cache_downloader.cpp
  ↳ #include "olap/rowset/rowset_meta.h"
```

**olap → io dependency (most olap files):**
```
be/src/olap/rowset/beta_rowset_reader.cpp
  ↳ #include "io/fs/file_reader.h"
  ↳ #include "io/cache/block_file_cache.h"

be/src/olap/tablet.cpp
  ↳ #include "io/fs/file_writer.h"
```

**Severity**: 🟡 **MODERATE** - IO should be foundational, but knows about OLAP types

**Root Cause**:
- File cache implementation needs OLAP metadata (rowset, tablet schema)
- Should use abstract types instead

---

## Dependency Graph Visualization

```
Current (Circular):

    ┌─────────┐
    │ common  │◄──────────────────┐
    └────┬────┘                   │
         │                        │
         ↓                        │
    ┌─────────┐              ┌────┴────┐
    │  util   │──────────────►  olap   │
    └────┬────┘              └────┬────┘
         │                        │
         ↓                        ↓
    ┌─────────┐              ┌─────────┐
    │   io    │◄─────────────┤ runtime │
    └─────────┘              └─────────┘
         ▲                        │
         └────────────────────────┘

Legend: A → B means "A depends on B"
        Arrows form cycles!
```

---

## Resolution Strategies

### Strategy 1: Library Splitting (RECOMMENDED)

Split each large library into smaller, layered sub-libraries.

**Example: Split `common` into:**
```
common/
├── foundation/          # No dependencies on other BE libs
│   ├── status_core      # Status without stack traces
│   ├── compiler_util    # Compiler utilities
│   └── expected         # Expected<T, E> type
├── core/                # Depends on foundation + util/foundation
│   ├── status           # Full Status with stack traces
│   ├── config           # Configuration management
│   └── daemon           # Daemon utilities
└── testing/             # Test utilities
    └── be_mock_util
```

**Example: Split `util` into:**
```
util/
├── foundation/          # No dependencies on other BE libs
│   ├── slice            # Basic data types
│   ├── coding           # Encoding/decoding
│   └── bit_util         # Bit manipulation
├── compression/         # Depends on foundation
│   └── block_compression
├── network/             # Depends on foundation
│   └── brpc_client_cache
├── arrow/               # Depends on foundation + arrow lib
├── io_util/             # Depends on foundation + io (OK, higher layer)
│   └── s3_util
└── testing/
    └── test_util
```

**Benefits:**
- ✅ Breaks cycles by creating dependency layers
- ✅ Enables incremental builds (only rebuild affected layers)
- ✅ Improves test isolation
- ✅ Clear architectural boundaries

**Effort:** 1-2 weeks (refactoring BUILD files, minimal code changes)

---

### Strategy 2: Interface Extraction

Extract interfaces to break circular dependencies.

**Example: Runtime ↔ OLAP cycle:**

Create `runtime/interfaces/storage_interface.h`:
```cpp
namespace doris::runtime {
// Abstract interface - no dependency on olap/
class IStorageEngine {
public:
    virtual ~IStorageEngine() = default;
    virtual Status createTablet(...) = 0;
    // ...
};
}
```

Then `olap/storage_engine.h`:
```cpp
#include "runtime/interfaces/storage_interface.h"

namespace doris {
class StorageEngine : public runtime::IStorageEngine {
    // Implementation
};
}
```

**Benefits:**
- ✅ Breaks tight coupling
- ✅ Enables dependency injection
- ✅ Improves testability

**Drawbacks:**
- ⚠️ Runtime overhead (virtual calls)
- ⚠️ More complex code structure

**Effort:** 2-4 weeks (design interfaces, refactor code)

---

### Strategy 3: Forward Declarations + PIMPL

Use forward declarations and PIMPL idiom to reduce header dependencies.

**Example: `common/status.h` depending on `util/stack_util.h`:**

**Before:**
```cpp
// status.h
#include "util/stack_util.h"

class Status {
    std::string stack_trace_;  // Captures stack on error
};
```

**After:**
```cpp
// status.h - NO #include "util/stack_util.h"
class Status {
private:
    struct Impl;  // Forward declaration
    std::unique_ptr<Impl> pimpl_;
};

// status.cpp
#include "common/status.h"
#include "util/stack_util.h"  // Only in .cpp file

struct Status::Impl {
    std::string stack_trace_;
};
```

**Benefits:**
- ✅ Breaks header-level dependencies
- ✅ Faster compilation (fewer includes)
- ✅ Better encapsulation

**Drawbacks:**
- ⚠️ Slight runtime overhead (heap allocation)
- ⚠️ More verbose code

**Effort:** 1-2 weeks (refactor key headers)

---

### Strategy 4: Configuration as a Separate Library

Move configuration to a standalone library that everything can depend on.

**Create `config_foundation` library:**
```
config_foundation/
├── BUILD.bazel
├── config.h
├── config.cpp
└── gflags_wrapper.h
```

All other libraries depend on `config_foundation` (no circular dependency).

**Benefits:**
- ✅ Config is needed everywhere, so this is acceptable
- ✅ Simple to implement
- ✅ No code changes

**Effort:** 1 day (create new BUILD target, update deps)

---

## Recommended Implementation Plan

### Phase 1: Quick Wins (Week 1)

**Goal:** Enable basic compilation

1. **Extract `config_foundation` library**
   - Move `common/config.*` to `common/foundation/config.*`
   - Create `//be/src/common/foundation:config` target
   - Update all BUILD files to depend on it

2. **Split `common/status`**
   - Create `common/foundation/status_core` (no stack traces)
   - Keep `common/status` (with stack traces, depends on util)
   - Most code uses `StatusCore`, breaking the cycle

3. **Split `util` into `util/foundation` and `util/advanced`**
   - `util/foundation`: slice, coding, bit_util (no BE deps)
   - `util/advanced`: everything else (can depend on other BE libs)

**Expected Result:** `common/foundation` and `util/foundation` have no circular dependencies

---

### Phase 2: Larger Refactoring (Week 2-3)

**Goal:** Resolve runtime ↔ olap cycle

4. **Extract storage interfaces**
   - Create `runtime/interfaces/storage_interface.h`
   - Create `runtime/interfaces/tablet_interface.h`
   - Update `runtime/exec_env` to use interfaces
   - Update `olap/storage_engine` to implement interfaces

5. **Split IO utilities**
   - Move `util/s3_util.*` → `io/s3/s3_util.*`
   - Move `util/async_io.*` → `io/async_io.*`
   - Update BUILD dependencies

**Expected Result:** Clean layering: `foundation → io → runtime → olap`

---

### Phase 3: Validation (Week 3-4)

6. **Build with Bazel**
   - Run `bazel build //be/src/common/foundation:all`
   - Run `bazel build //be/src/util/foundation:all`
   - Fix any remaining circular dependency errors

7. **Test compilation**
   - Run `bazel test //be/test/common:all`
   - Run `bazel test //be/test/util:all`
   - Verify tests pass

8. **Update documentation**
   - Update all BUILD.bazel files with new targets
   - Update be/README.bazel.md with new structure
   - Document dependency rules for future development

---

## Dependency Rules for Future Development

To prevent circular dependencies from reoccurring, establish these rules:

### Layer Definitions

```
Layer 0 (Foundation):
  - common/foundation (status_core, compiler_util, expected)
  - util/foundation (slice, coding, bit_util)
  - No dependencies on other BE libraries

Layer 1 (Core):
  - common/core (status, config, daemon)
  - util/core (compression, encoding, networking)
  - gensrc (generated proto/thrift)
  - May depend on: Layer 0 + third-party libs

Layer 2 (Infrastructure):
  - io (filesystems, caching)
  - util/advanced (arrow, simd, io utilities)
  - May depend on: Layer 0, Layer 1

Layer 3 (Storage):
  - olap (tablets, rowsets, compaction)
  - May depend on: Layer 0, Layer 1, Layer 2

Layer 4 (Execution):
  - runtime (exec env, memory, load management)
  - May depend on: Layer 0-3 (via interfaces)

Layer 5 (Query Processing):
  - exec (operators, scanners)
  - vec (vectorized execution)
  - exprs (expressions)
  - May depend on: Layer 0-4

Layer 6 (Services):
  - service (RPC services)
  - http (HTTP handlers)
  - May depend on: All lower layers
```

### Enforcement Rules

1. **Lower layers CANNOT depend on higher layers**
   - ✅ runtime can depend on olap (lower layer)
   - ❌ olap cannot depend on runtime (higher layer)

2. **Use interfaces for upward dependencies**
   - If Layer N needs something from Layer N+1, define interface in Layer N
   - Layer N+1 implements the interface

3. **Common types in foundation layers**
   - Status, Config, basic utilities → Layer 0
   - Generated proto/thrift → Layer 1

4. **Bazel visibility enforcement**
   ```python
   # In common/foundation/BUILD.bazel
   package(default_visibility = ["//be:__subpackages__"])

   # In olap/BUILD.bazel
   cc_library(
       name = "olap",
       visibility = ["//be/src/runtime:__pkg__"],  # Only runtime can use
   )
   ```

---

## Tracking Progress

### Circular Dependency Elimination Checklist

- [ ] **Phase 1: Quick Wins**
  - [ ] Extract `config_foundation` library
  - [ ] Split `common/status` into `status_core` and `status`
  - [ ] Split `util` into `foundation` and `advanced`
  - [ ] Verify: `bazel build //be/src/common/foundation:all`

- [ ] **Phase 2: Larger Refactoring**
  - [ ] Extract runtime storage interfaces
  - [ ] Move IO utilities from util to io
  - [ ] Verify: `bazel build //be/src/io:all`
  - [ ] Verify: `bazel build //be/src/runtime:all`

- [ ] **Phase 3: Validation**
  - [ ] Build all backend components with Bazel
  - [ ] Run all tests with Bazel
  - [ ] Update documentation
  - [ ] Verify: `bazel build //be:backend_libs`

---

## Metrics for Success

Track these metrics before/after refactoring:

| Metric | Before | Target |
|--------|--------|--------|
| Circular dependency chains | 5+ | 0 |
| Files in circular deps | 150+ | 0 |
| Library count | 5 large | 15-20 small |
| Average rebuild time (1 file change) | N/A | < 30s |
| Bazel build success | ❌ Fails | ✅ Passes |

---

## References

### Bazel Documentation
- [Dealing with Circular Dependencies](https://bazel.build/concepts/dependencies#cycles)
- [Dependency Management Best Practices](https://bazel.build/concepts/build-files#managing-dependencies)

### Design Patterns
- [PIMPL Idiom](https://en.cppreference.com/w/cpp/language/pimpl)
- [Dependency Inversion Principle](https://en.wikipedia.org/wiki/Dependency_inversion_principle)
- [Interface Segregation](https://en.wikipedia.org/wiki/Interface_segregation_principle)

### Doris Documentation
- [Doris Backend Architecture](../be/README.md)
- [Bazel Migration Guide](../README.Bazel.md)
- [Migration TODO](../todos.md)

---

**Last Updated:** 2025-11-20
**Status:** Analysis complete, awaiting implementation
**Next Steps:** Begin Phase 1 refactoring (extract foundation libraries)
**Estimated Total Effort:** 3-4 weeks
**Priority:** 🔴 **CRITICAL** - Blocks Bazel migration
