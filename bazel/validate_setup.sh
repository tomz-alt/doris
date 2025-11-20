#!/usr/bin/env bash
# Apache Doris - Bazel Setup Validation Script
# Licensed to the Apache Software Foundation (ASF)
#
# This script validates that the Bazel build environment is properly configured

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DORIS_HOME="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=========================================="
echo "Doris Bazel Setup Validation"
echo "=========================================="
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

passed=0
failed=0
warnings=0

check_pass() {
    echo -e "${GREEN}✓${NC} $1"
    ((passed++))
}

check_fail() {
    echo -e "${RED}✗${NC} $1"
    ((failed++))
}

check_warn() {
    echo -e "${YELLOW}⚠${NC} $1"
    ((warnings++))
}

# Check 1: Bazel installation
echo "Checking Bazel installation..."
if command -v bazel &> /dev/null; then
    BAZEL_VERSION=$(bazel --version | awk '{print $2}')
    check_pass "Bazel is installed (version: $BAZEL_VERSION)"

    # Check version >= 6.0.0
    MAJOR_VERSION=$(echo "$BAZEL_VERSION" | cut -d. -f1)
    if [ "$MAJOR_VERSION" -ge 6 ]; then
        check_pass "Bazel version is compatible (>= 6.0.0)"
    else
        check_warn "Bazel version may be too old (found $BAZEL_VERSION, recommend >= 6.5.0)"
    fi
else
    check_fail "Bazel is not installed or not in PATH"
    echo "  Install with: npm install -g @bazel/bazelisk"
    echo "  Or see: https://bazel.build/install"
fi
echo ""

# Check 2: Third-party dependencies
echo "Checking third-party dependencies..."
THIRDPARTY_DIR="${DORIS_HOME}/thirdparty/installed"
if [ -d "$THIRDPARTY_DIR" ]; then
    check_pass "Third-party directory exists"

    # Check for key libraries
    if [ -d "${THIRDPARTY_DIR}/lib" ]; then
        check_pass "Third-party lib directory exists"

        # Count libraries
        LIB_COUNT=$(find "${THIRDPARTY_DIR}/lib" -name "*.a" -o -name "*.so" 2>/dev/null | wc -l)
        if [ "$LIB_COUNT" -gt 10 ]; then
            check_pass "Third-party libraries built ($LIB_COUNT files found)"
        else
            check_fail "Third-party libraries not fully built ($LIB_COUNT files found, expected 50+)"
            echo "  Run: cd thirdparty && ./build-thirdparty.sh"
        fi
    else
        check_fail "Third-party lib directory missing"
        echo "  Run: cd thirdparty && ./build-thirdparty.sh"
    fi

    if [ -d "${THIRDPARTY_DIR}/include" ]; then
        check_pass "Third-party include directory exists"
    else
        check_fail "Third-party include directory missing"
    fi
else
    check_fail "Third-party directory not found at $THIRDPARTY_DIR"
    echo "  Run: cd thirdparty && ./build-thirdparty.sh"
fi
echo ""

# Check 3: Generated sources
echo "Checking generated sources..."
GENSRC_DIR="${DORIS_HOME}/gensrc/build"
if [ -d "$GENSRC_DIR" ]; then
    check_pass "Generated sources directory exists"

    # Check for proto files
    PROTO_COUNT=$(find "${GENSRC_DIR}" -name "*.pb.h" -o -name "*.pb.cc" 2>/dev/null | wc -l)
    if [ "$PROTO_COUNT" -gt 5 ]; then
        check_pass "Protobuf sources generated ($PROTO_COUNT files)"
    else
        check_warn "Protobuf sources may not be generated ($PROTO_COUNT files found)"
        echo "  Run: cd gensrc && make"
    fi

    # Check for thrift files
    THRIFT_COUNT=$(find "${GENSRC_DIR}" -name "*_types.h" -o -name "*_types.cpp" 2>/dev/null | wc -l)
    if [ "$THRIFT_COUNT" -gt 5 ]; then
        check_pass "Thrift sources generated ($THRIFT_COUNT files)"
    else
        check_warn "Thrift sources may not be generated ($THRIFT_COUNT files found)"
        echo "  Run: cd gensrc && make"
    fi
else
    check_fail "Generated sources not found at $GENSRC_DIR"
    echo "  Run: cd gensrc && make"
fi
echo ""

# Check 4: Bazel workspace files
echo "Checking Bazel workspace files..."
if [ -f "${DORIS_HOME}/WORKSPACE.bazel" ]; then
    check_pass "WORKSPACE.bazel exists"
else
    check_fail "WORKSPACE.bazel missing"
fi

if [ -f "${DORIS_HOME}/.bazelrc" ]; then
    check_pass ".bazelrc exists"
else
    check_fail ".bazelrc missing"
fi

if [ -f "${DORIS_HOME}/.bazelversion" ]; then
    check_pass ".bazelversion exists"
else
    check_warn ".bazelversion missing (recommended for consistency)"
fi

if [ -f "${DORIS_HOME}/BUILD.bazel" ]; then
    check_pass "Root BUILD.bazel exists"
else
    check_fail "Root BUILD.bazel missing"
fi
echo ""

# Check 5: Backend BUILD files
echo "Checking backend BUILD files..."
BE_BUILD_FILES=(
    "be/BUILD.bazel"
    "be/src/common/BUILD.bazel"
    "be/src/util/BUILD.bazel"
    "be/src/io/BUILD.bazel"
    "be/src/runtime/BUILD.bazel"
    "be/src/olap/BUILD.bazel"
)

for build_file in "${BE_BUILD_FILES[@]}"; do
    if [ -f "${DORIS_HOME}/${build_file}" ]; then
        check_pass "$build_file exists"
    else
        check_warn "$build_file missing (optional)"
    fi
done
echo ""

# Check 6: Try a simple Bazel command
echo "Testing Bazel workspace..."
cd "$DORIS_HOME"
if bazel info workspace &> /dev/null; then
    check_pass "Bazel workspace is valid"
    WORKSPACE_PATH=$(bazel info workspace 2>/dev/null)
    echo "  Workspace: $WORKSPACE_PATH"
else
    check_fail "Bazel workspace has errors"
    echo "  Run: bazel info workspace (to see errors)"
fi
echo ""

# Check 7: Test hello_bazel target
if [ -f "${DORIS_HOME}/bazel/test/BUILD.bazel" ]; then
    echo "Testing hello_bazel target..."
    if bazel build //bazel/test:hello_bazel &> /dev/null; then
        check_pass "hello_bazel builds successfully"

        # Try to run it
        if bazel run //bazel/test:hello_bazel 2>&1 | grep -q "Hello from Bazel"; then
            check_pass "hello_bazel runs successfully"
        else
            check_warn "hello_bazel built but output unexpected"
        fi
    else
        check_warn "hello_bazel failed to build (this is OK if thirdparty not built yet)"
    fi
    echo ""
fi

# Summary
echo "=========================================="
echo "Validation Summary"
echo "=========================================="
echo -e "${GREEN}Passed:${NC}   $passed"
echo -e "${YELLOW}Warnings:${NC} $warnings"
echo -e "${RED}Failed:${NC}   $failed"
echo ""

if [ $failed -eq 0 ] && [ $warnings -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed! Your Bazel setup is ready.${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Build backend: bazel build //be/src/common:common"
    echo "  2. Run tests: bazel test //be/test/common:compare_test"
    echo "  3. Generate IDE support: bazel run @hedron_compile_commands//:refresh_all"
    exit 0
elif [ $failed -eq 0 ]; then
    echo -e "${YELLOW}⚠ Setup is mostly ready, but some warnings exist.${NC}"
    echo ""
    echo "Recommended actions:"
    if [ $warnings -gt 0 ]; then
        echo "  - Review warnings above and address as needed"
    fi
    echo "  - Try building: bazel build //bazel/test:hello_bazel"
    exit 0
else
    echo -e "${RED}✗ Setup has issues that need to be fixed.${NC}"
    echo ""
    echo "Required actions:"
    echo "  1. Install Bazel (if not installed)"
    echo "  2. Build third-party dependencies: cd thirdparty && ./build-thirdparty.sh"
    echo "  3. Generate sources: cd gensrc && make"
    echo ""
    echo "See documentation: bazel/README.md or be/README.bazel.md"
    exit 1
fi
