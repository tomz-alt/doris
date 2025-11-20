# Claude's Task Tracking - Doris CMake to Bazel Migration

This document tracks the current progress, finished tasks, and expected upcoming tasks for the build system migration.

---

## Session Information

- **Session ID**: claude/migrate-cmake-to-bazel-016aCAqvuWxkoyNukiH5BUSg
- **Branch**: claude/migrate-cmake-to-bazel-016aCAqvuWxkoyNukiH5BUSg
- **Start Date**: 2025-11-20
- **Current Phase**: Phase 1 - Analysis & Foundation

---

## Completed Tasks ✅

### Analysis Phase

1. **Build System Analysis** - COMPLETED
   - Analyzed build.sh:1-100 - Identified monolithic orchestration script
   - Analyzed be/CMakeLists.txt:1-100 - Identified CMake platform detection and options
   - Analyzed thirdparty/vars.sh:1-100 - Identified third-party dependency management
   - Analyzed gensrc/Makefile:1-37 - Identified proto/thrift/script generation pipeline
   - Explored BE source directory structure:
     - be/src/udf, exec, io, olap, util, vec (main components identified)
   - Explored root directory structure

2. **Documentation Creation** - COMPLETED
   - Created todos.md with comprehensive migration tracking
   - Created tools.md with Bazel tooling guide and best practices
   - Created CLAUDE.md (this file) for session tracking

### Key Findings from Analysis

**Build System Architecture**:
```
build.sh (root orchestrator)
├── env.sh (environment setup)
├── gensrc/Makefile (generated sources)
│   ├── script/ (custom generators)
│   ├── proto/ (protobuf files)
│   └── thrift/ (thrift files)
├── thirdparty/ (third-party builds)
│   ├── vars.sh (dependency definitions)
│   └── build-*.sh (per-dependency build scripts)
├── be/CMakeLists.txt (backend C++)
│   └── be/src/* (source tree)
├── fe/pom.xml (frontend Java/Maven)
└── ui/package.json (UI npm)
```

**Critical Dependencies Identified**:
- C++ Libraries: libevent, openssl, thrift, protobuf, gflags, glog, gtest, snappy, lz4, zlib, zstd, rocksdb, arrow, brpc, boost
- Build Tools: CMake 3.19.2+, Maven, npm, Make
- Compilers: GCC or Clang with C++17 support
- Platform Targets: Linux (x86_64, aarch64), macOS

**Build Complexity Metrics**:
- Backend source directories: ~30+ major components
- Third-party dependencies: ~30+ libraries
- Build flags: ~20+ configurable options
- Supported platforms: Linux x86_64, Linux aarch64, macOS

---

## Current Tasks 🔄

### In Progress

1. **Documentation & Planning**
   - Status: Final review of documentation files
   - Files created: todos.md, tools.md, CLAUDE.md
   - Next: Commit and push documentation

---

## Next Tasks (Priority Order) ⏳

### Immediate Next Steps (Phase 2)

1. **Set Up Initial Bazel Workspace**
   - Create WORKSPACE.bazel with initial dependencies
   - Create .bazelrc with platform configurations
   - Create root BUILD.bazel
   - Test basic Bazel setup with hello-world C++ target

2. **Import Third-party Dependencies**
   - Create bazel/third_party/BUILD.bazel
   - Add cc_import rules for essential libraries (glog, gflags, protobuf, gtest)
   - Test imports with minimal test binary
   - Document import pattern for remaining libraries

3. **Prototype Backend Build (Minimal)**
   - Create be/src/common/BUILD.bazel (smallest viable library)
   - Create be/src/util/BUILD.bazel (utility library)
   - Create simple test target to validate setup
   - Generate compile_commands.json for IDE support

### Upcoming Tasks (Phase 3)

4. **Migrate Generated Sources (gensrc)**
   - Create proto_library rules for gensrc/proto/*.proto
   - Create genrule for thrift generation
   - Create genrule for script-based generation
   - Validate generated headers match CMake output

5. **Expand Backend Coverage**
   - Create BUILD.bazel files for major components:
     - be/src/olap
     - be/src/exec
     - be/src/runtime
     - be/src/vec
   - Build doris_be binary target
   - Test binary execution

6. **Testing & Validation**
   - Create cc_test rules for existing unit tests
   - Run tests with Bazel
   - Compare test results with CMake build
   - Benchmark build times

### Future Tasks (Phase 4+)

7. **Frontend Integration**
   - Create Maven wrapper genrule
   - Export FE artifacts
   - Test FE build integration

8. **Full Integration**
   - Create end-to-end build targets
   - Implement packaging/distribution
   - Set up CI/CD integration
   - Performance optimization

---

## Decision Log

### 2025-11-20

**Decision**: Use cc_import for third-party libraries initially
- **Rationale**: Importing existing third-party/installed libraries avoids rebuilding 30+ dependencies from source, significantly reducing migration complexity
- **Alternative considered**: Rebuild all third-party deps with Bazel (http_archive + custom rules)
- **Trade-off**: Less hermetic initially, but faster migration path

**Decision**: Start with backend (BE) migration, defer frontend (FE)
- **Rationale**: BE is pure C++ and more straightforward to migrate; FE involves complex Maven setup
- **Alternative considered**: Migrate FE first or in parallel
- **Trade-off**: Sequential approach is slower but reduces risk

**Decision**: Keep gensrc as genrule initially
- **Rationale**: Preserves existing generation logic, validates output parity
- **Alternative considered**: Rewrite generators in Starlark
- **Trade-off**: Less "Bazel-native" but lower risk of generation bugs

**Decision**: Use wrapper approach for Maven/npm
- **Rationale**: Minimal changes to FE/UI, focus migration effort on BE
- **Alternative considered**: Native Bazel Java rules
- **Trade-off**: Less incremental caching for Java, but much faster to implement

---

## Blockers & Risks

### Current Blockers
None - analysis phase complete, ready to begin implementation

### Identified Risks

1. **Third-party Complexity** - MEDIUM RISK
   - Risk: 30+ third-party libraries with custom patches
   - Mitigation: Use cc_import of existing builds initially
   - Status: Mitigated by decision to import rather than rebuild

2. **Generated Source Parity** - MEDIUM RISK
   - Risk: Bazel-generated sources might differ from CMake
   - Mitigation: Extensive validation with diff, byte-for-byte comparison
   - Status: Will validate in Phase 3

3. **Platform Variations** - LOW RISK
   - Risk: Different behavior on x86_64 vs aarch64, Linux vs macOS
   - Mitigation: Use select() extensively, test on all platforms
   - Status: Will address in Phase 2 with .bazelrc configuration

4. **Build Time Regression** - MEDIUM RISK
   - Risk: Initial Bazel builds might be slower than optimized CMake
   - Mitigation: Profile builds, enable remote caching, optimize dependency graph
   - Status: Will benchmark continuously starting Phase 3

5. **Developer Adoption** - LOW RISK
   - Risk: Team might resist new build system
   - Mitigation: Excellent documentation (tools.md), parallel CMake/Bazel period
   - Status: Documentation created, will support parallel builds

---

## Performance Targets

### Build Time Goals (vs current CMake build)

- **Clean Build**: < 50% of CMake clean build time
- **Incremental Build (1 .cc change)**: < 25% of CMake incremental build
- **Incremental Build (1 .java change)**: < 50% of Maven incremental build
- **CI/CD Build**: > 40% time reduction with remote caching

### Caching Goals

- **Remote Cache Hit Rate**: > 80%
- **Local Disk Cache**: Configured and enabled
- **Action Cache**: Enabled with appropriate cache keys

### Developer Experience Goals

- **IDE Integration**: Working compile_commands.json, Bazel plugin support
- **Query Performance**: < 2 seconds for common queries
- **Build Comprehension**: Clear error messages, good documentation

---

## Open Questions

1. **Remote Cache Infrastructure**
   - Question: Where should we host the remote cache? Self-hosted or cloud?
   - Status: Deferred to Phase 7
   - Impact: Medium - affects build performance for distributed teams

2. **Toolchain Hermiticity**
   - Question: Should we use fully hermetic C++ toolchain or rely on system toolchain?
   - Status: Open - will decide in Phase 2
   - Impact: High - affects reproducibility and cross-platform consistency

3. **Java Rules Strategy**
   - Question: Keep Maven wrapper permanently or migrate to native Bazel Java rules?
   - Status: Open - will evaluate after Phase 4
   - Impact: Medium - affects FE build performance and maintainability

4. **Release Workflow**
   - Question: How do we replicate current build.sh release packaging with Bazel?
   - Status: Deferred to Phase 6
   - Impact: High - affects production releases

---

## Resources & References

### Documentation Created
- [todos.md](todos.md) - Comprehensive migration task breakdown
- [tools.md](tools.md) - Bazel tooling guide and best practices
- [CLAUDE.md](CLAUDE.md) - This file, session tracking

### External Resources
- [Bazel Documentation](https://bazel.build/)
- [Bazel C++ Tutorial](https://bazel.build/tutorials/cpp)
- [Bazel Best Practices](https://bazel.build/basics/best-practices)
- [Migrating to Bazel](https://bazel.build/migrate)

### Repository Structure Reference
```
/home/user/doris/
├── build.sh              # Current monolithic build script
├── env.sh                # Environment setup
├── be/                   # Backend (C++)
│   ├── CMakeLists.txt    # Current CMake config
│   └── src/              # C++ source tree
├── fe/                   # Frontend (Java)
│   └── pom.xml           # Maven config
├── ui/                   # UI (JavaScript/TypeScript)
│   └── package.json      # npm config
├── gensrc/               # Generated sources
│   └── Makefile          # Current generation logic
├── thirdparty/           # Third-party dependencies
│   └── vars.sh           # Dependency definitions
└── [future] bazel/       # Bazel-specific configs
    ├── third_party/      # Third-party BUILD files
    ├── java/             # Java/Maven integration
    └── ui/               # UI/npm integration
```

---

## Commit History (This Session)

_No commits yet - documentation phase complete, implementation starts next_

### Planned Commits

1. **docs: Add Bazel migration documentation and tracking**
   - Add todos.md, tools.md, CLAUDE.md
   - Document migration strategy and tooling

2. **build: Initialize Bazel workspace**
   - Add WORKSPACE.bazel
   - Add .bazelrc
   - Add root BUILD.bazel

3. **build: Add third-party Bazel imports**
   - Add bazel/third_party/BUILD.bazel
   - Import essential C++ libraries

4. **build: Add backend common and util libraries**
   - Add be/src/common/BUILD.bazel
   - Add be/src/util/BUILD.bazel
   - Add test targets

_(More commits will be added as work progresses)_

---

## Notes for Future Sessions

### Things to Remember

1. **Always validate generated sources**: Use `diff` to compare gensrc output between CMake and Bazel
2. **Test on multiple platforms**: Don't assume x86_64 behavior matches aarch64
3. **Profile builds regularly**: Use `bazel build --profile=profile.json` to catch regressions
4. **Keep CMake working**: Parallel builds during transition period
5. **Document decisions**: Update this file with rationale for major choices

### Quick Start Commands for Next Session

```bash
# Check current branch
git status

# Continue from where we left off
cat CLAUDE.md  # Review this file

# Start Phase 2 work
# 1. Create WORKSPACE.bazel
# 2. Create .bazelrc
# 3. Create root BUILD.bazel
# 4. Test with bazel info
```

---

## Success Criteria

The migration will be considered successful when:

- [ ] All BE targets build with Bazel
- [ ] All BE tests pass with Bazel
- [ ] FE/UI build via Bazel wrappers
- [ ] Build times meet performance targets
- [ ] Remote caching operational with >80% hit rate
- [ ] IDE integration working (compile_commands.json)
- [ ] Documentation complete and team trained
- [ ] CI/CD migrated to Bazel
- [ ] CMake build deprecated and removed

---

**Last Updated**: 2025-11-20
**Current Status**: Phase 1 complete, ready for Phase 2 implementation
**Next Session Goal**: Set up initial Bazel workspace and test basic functionality
