#!/usr/bin/env bash
# Apache Doris - Bazel Build Helper Script
# Licensed to the Apache Software Foundation (ASF)
#
# This script provides convenient commands for common Bazel build operations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DORIS_HOME="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "$DORIS_HOME"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${YELLOW}ℹ${NC} $1"
}

usage() {
    cat << EOF
Doris Bazel Build Helper

USAGE:
    $0 <command> [options]

COMMANDS:
    validate            Run setup validation
    build-all           Build all backend components
    build-common        Build common library
    build-util          Build util library
    build-io            Build I/O layer
    build-runtime       Build runtime environment
    build-olap          Build OLAP storage engine
    test-all            Run all tests
    test-common         Run common tests
    test-util           Run util tests
    test-quick          Run quick test suite
    clean               Clean build outputs
    clean-all           Clean everything (including cache)
    deps <target>       Show dependencies of a target
    rdeps <target>      Show reverse dependencies of a target
    graph <target>      Generate dependency graph
    compile-commands    Generate compile_commands.json for IDE
    help                Show this help message

OPTIONS:
    --config=<config>   Build configuration: debug, release, relwithdebinfo
    --jobs=<n>          Number of parallel jobs
    --verbose           Show verbose output

EXAMPLES:
    $0 validate                              # Run setup validation
    $0 build-all                            # Build all BE components
    $0 build-common --config=debug          # Build common in debug mode
    $0 build-all --jobs=16                  # Build with 16 parallel jobs
    $0 test-all                              # Run all tests
    $0 deps //be/src/common:common           # Show dependencies
    $0 compile-commands                      # Generate IDE support

QUICK START:
    1. $0 validate                           # Check setup
    2. $0 build-common                       # Try building common
    3. $0 test-common                        # Run common tests
    4. $0 compile-commands                   # Setup IDE

EOF
}

# Parse common options
CONFIG=""
JOBS=""
VERBOSE=""

parse_options() {
    for arg in "$@"; do
        case $arg in
            --config=*)
                CONFIG="--config=${arg#*=}"
                ;;
            --jobs=*)
                JOBS="--jobs=${arg#*=}"
                ;;
            --verbose)
                VERBOSE="--verbose_failures --subcommands"
                ;;
        esac
    done
}

# Commands
cmd_validate() {
    print_header "Running Setup Validation"
    "${SCRIPT_DIR}/validate_setup.sh"
}

cmd_build_all() {
    parse_options "$@"
    print_header "Building All Backend Components"

    components=(
        "//be/src/common:common"
        "//be/src/util:util"
        "//be/src/io:io"
        "//be/src/runtime:runtime"
        "//be/src/olap:olap"
    )

    for component in "${components[@]}"; do
        print_info "Building $component..."
        if bazel build $CONFIG $JOBS $VERBOSE "$component"; then
            print_success "Built $component"
        else
            print_error "Failed to build $component"
            return 1
        fi
    done

    print_success "All components built successfully!"
}

cmd_build_common() {
    parse_options "$@"
    print_header "Building Common Library"
    bazel build $CONFIG $JOBS $VERBOSE //be/src/common:common
}

cmd_build_util() {
    parse_options "$@"
    print_header "Building Util Library"
    bazel build $CONFIG $JOBS $VERBOSE //be/src/util:util
}

cmd_build_io() {
    parse_options "$@"
    print_header "Building I/O Layer"
    bazel build $CONFIG $JOBS $VERBOSE //be/src/io:io
}

cmd_build_runtime() {
    parse_options "$@"
    print_header "Building Runtime Environment"
    bazel build $CONFIG $JOBS $VERBOSE //be/src/runtime:runtime
}

cmd_build_olap() {
    parse_options "$@"
    print_header "Building OLAP Storage Engine"
    bazel build $CONFIG $JOBS $VERBOSE //be/src/olap:olap
}

cmd_test_all() {
    parse_options "$@"
    print_header "Running All Tests"
    bazel test $CONFIG $JOBS --test_output=errors //be/test/...
}

cmd_test_common() {
    parse_options "$@"
    print_header "Running Common Tests"
    bazel test $CONFIG $JOBS --test_output=errors //be/test/common:...
}

cmd_test_util() {
    parse_options "$@"
    print_header "Running Util Tests"
    bazel test $CONFIG $JOBS --test_output=errors //be/test/util:...
}

cmd_test_quick() {
    parse_options "$@"
    print_header "Running Quick Test Suite"
    bazel test $CONFIG $JOBS --test_output=errors //be:quick_tests
}

cmd_clean() {
    print_header "Cleaning Build Outputs"
    bazel clean
    print_success "Build outputs cleaned"
}

cmd_clean_all() {
    print_header "Cleaning Everything (including cache)"
    bazel clean --expunge
    print_success "Everything cleaned"
}

cmd_deps() {
    if [ -z "$2" ]; then
        print_error "Target required. Usage: $0 deps <target>"
        echo "Example: $0 deps //be/src/common:common"
        return 1
    fi
    print_header "Dependencies of $2"
    bazel query "deps($2)"
}

cmd_rdeps() {
    if [ -z "$2" ]; then
        print_error "Target required. Usage: $0 rdeps <target>"
        echo "Example: $0 rdeps //be/src/common:common"
        return 1
    fi
    print_header "Reverse Dependencies of $2"
    bazel query "rdeps(//..., $2)"
}

cmd_graph() {
    if [ -z "$2" ]; then
        print_error "Target required. Usage: $0 graph <target>"
        echo "Example: $0 graph //be/src/common:common"
        return 1
    fi

    OUTPUT_FILE="dependency_graph.dot"
    PNG_FILE="dependency_graph.png"

    print_header "Generating Dependency Graph for $2"
    bazel query "deps($2)" --output=graph > "$OUTPUT_FILE"
    print_success "Graph written to $OUTPUT_FILE"

    if command -v dot &> /dev/null; then
        dot -Tpng "$OUTPUT_FILE" -o "$PNG_FILE"
        print_success "PNG generated: $PNG_FILE"
    else
        print_info "Install graphviz to generate PNG: sudo apt install graphviz"
    fi
}

cmd_compile_commands() {
    print_header "Generating compile_commands.json for IDE"
    bazel run @hedron_compile_commands//:refresh_all
    print_success "compile_commands.json generated"
    print_info "You can now use this with clangd, CLion, VS Code, etc."
}

# Main command dispatch
case "${1:-help}" in
    validate)
        cmd_validate
        ;;
    build-all)
        shift
        cmd_build_all "$@"
        ;;
    build-common)
        shift
        cmd_build_common "$@"
        ;;
    build-util)
        shift
        cmd_build_util "$@"
        ;;
    build-io)
        shift
        cmd_build_io "$@"
        ;;
    build-runtime)
        shift
        cmd_build_runtime "$@"
        ;;
    build-olap)
        shift
        cmd_build_olap "$@"
        ;;
    test-all)
        shift
        cmd_test_all "$@"
        ;;
    test-common)
        shift
        cmd_test_common "$@"
        ;;
    test-util)
        shift
        cmd_test_util "$@"
        ;;
    test-quick)
        shift
        cmd_test_quick "$@"
        ;;
    clean)
        cmd_clean
        ;;
    clean-all)
        cmd_clean_all
        ;;
    deps)
        cmd_deps "$@"
        ;;
    rdeps)
        cmd_rdeps "$@"
        ;;
    graph)
        cmd_graph "$@"
        ;;
    compile-commands)
        cmd_compile_commands
        ;;
    help|--help|-h)
        usage
        ;;
    *)
        print_error "Unknown command: $1"
        echo ""
        usage
        exit 1
        ;;
esac
