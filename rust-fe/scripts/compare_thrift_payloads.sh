#!/usr/bin/env bash
#
# compare_thrift_payloads.sh
#
# Compares Thrift payloads from Java FE and Rust FE for TPC-H lineitem scan.
# This script generates the Rust FE payload, decodes both Java and Rust payloads,
# and produces a side-by-side diff for field-by-field comparison.
#
# Prerequisites:
# - Rust FE codebase built with --features real_be_proto
# - Java FE payload captured at /tmp/java-fe-thrift.bin
#   (use tcpdump or BE logging to capture this)
#
# Usage:
#   ./scripts/compare_thrift_payloads.sh
#
# Output:
#   - /tmp/rust-fe-thrift.bin - Raw Rust FE Thrift payload
#   - /tmp/rust-thrift-decoded.txt - Decoded Rust payload structure
#   - /tmp/java-thrift-decoded.txt - Decoded Java payload structure (if Java payload exists)
#   - Diff output showing structural differences

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

RUST_PAYLOAD="/tmp/rust-fe-thrift.bin"
JAVA_PAYLOAD="/tmp/java-fe-thrift.bin"
RUST_DECODED="/tmp/rust-thrift-decoded.txt"
JAVA_DECODED="/tmp/java-thrift-decoded.txt"

cd "$PROJECT_ROOT"

echo "=========================================="
echo "Thrift Payload Comparison Tool"
echo "=========================================="
echo ""

# Step 1: Generate Rust FE payload
echo "[1/4] Generating Rust FE payload..."
cargo run --no-default-features --features real_be_proto \
    --example generate_payload 2>&1 | grep -E "^(✓|===)" || true

if [ ! -f "$RUST_PAYLOAD" ]; then
    echo "ERROR: Failed to generate Rust FE payload at $RUST_PAYLOAD"
    exit 1
fi

RUST_SIZE=$(wc -c < "$RUST_PAYLOAD" | tr -d ' ')
echo "  Rust FE payload: $RUST_SIZE bytes"
echo ""

# Step 2: Decode Rust FE payload
echo "[2/4] Decoding Rust FE payload..."
cargo run --no-default-features --features real_be_proto \
    --example decode_thrift -- "$RUST_PAYLOAD" 2>/dev/null > "$RUST_DECODED"

RUST_LINES=$(wc -l < "$RUST_DECODED" | tr -d ' ')
echo "  Decoded to: $RUST_DECODED ($RUST_LINES lines)"
echo ""

# Step 3: Check for Java FE payload and decode if present
if [ -f "$JAVA_PAYLOAD" ]; then
    JAVA_SIZE=$(wc -c < "$JAVA_PAYLOAD" | tr -d ' ')
    echo "[3/4] Decoding Java FE payload..."
    echo "  Java FE payload: $JAVA_SIZE bytes"

    cargo run --no-default-features --features real_be_proto \
        --example decode_thrift -- "$JAVA_PAYLOAD" 2>/dev/null > "$JAVA_DECODED"

    JAVA_LINES=$(wc -l < "$JAVA_DECODED" | tr -d ' ')
    echo "  Decoded to: $JAVA_DECODED ($JAVA_LINES lines)"
    echo ""

    # Step 4: Compare
    echo "[4/4] Comparing payloads..."
    echo ""
    echo "=========================================="
    echo "Size Comparison:"
    echo "=========================================="
    echo "  Rust FE: $RUST_SIZE bytes, $RUST_LINES decoded lines"
    echo "  Java FE: $JAVA_SIZE bytes, $JAVA_LINES decoded lines"
    echo ""
    echo "=========================================="
    echo "Structural Diff (Rust vs Java):"
    echo "=========================================="
    echo ""

    if diff -u "$JAVA_DECODED" "$RUST_DECODED" > /tmp/thrift-diff.txt; then
        echo "✓ Payloads are structurally IDENTICAL"
    else
        echo "✗ Payloads have structural differences:"
        echo ""
        head -100 /tmp/thrift-diff.txt
        echo ""
        echo "(Full diff saved to /tmp/thrift-diff.txt)"
    fi

    echo ""
    echo "=========================================="
    echo "Field-by-Field Analysis:"
    echo "=========================================="
    echo ""
    echo "Top-level TPipelineFragmentParamsList fields:"
    echo "  - Field 1: params_list (List)"
    echo "  - Field 9: is_nereids (Bool)"
    echo "  - Field 11: query_id (Struct with hi/lo)"
    echo ""
    echo "Check TPipelineFragmentParams (first element of field 1):"
    grep -E "field (1|2|3|4|5|21|23|28|40):" "$RUST_DECODED" | head -20 || true
    echo ""
    echo "For detailed field mapping, see:"
    echo "  - $RUST_DECODED"
    echo "  - $JAVA_DECODED"
    echo "  - /tmp/thrift-diff.txt"

else
    echo "[3/4] Java FE payload not found"
    echo ""
    echo "To capture Java FE payload:"
    echo "  1. Start docker compose with Java FE and BE:"
    echo "       ./docker/quickstart.sh"
    echo ""
    echo "  2. In doris-be container, capture traffic:"
    echo "       docker exec -it doris-be bash"
    echo "       tcpdump -i any -w /tmp/be-capture.pcap tcp port 8060"
    echo ""
    echo "  3. Run a query from Java FE:"
    echo "       docker exec -it doris-mysql-client mysql -h doris-fe-java -P 9030 -u root \\"
    echo "         -e \"USE tpch; SELECT * FROM lineitem LIMIT 1;\""
    echo ""
    echo "  4. Stop tcpdump, extract payload from pcap, save as:"
    echo "       $JAVA_PAYLOAD"
    echo ""
    echo "  5. Re-run this script to compare"
    echo ""
    echo "[4/4] Skipping comparison (no Java payload)"
fi

echo ""
echo "=========================================="
echo "Summary"
echo "=========================================="
echo "Rust FE payload: $RUST_PAYLOAD ($RUST_SIZE bytes)"
echo "Rust decoded:    $RUST_DECODED ($RUST_LINES lines)"
if [ -f "$JAVA_PAYLOAD" ]; then
    echo "Java FE payload: $JAVA_PAYLOAD ($JAVA_SIZE bytes)"
    echo "Java decoded:    $JAVA_DECODED ($JAVA_LINES lines)"
    echo "Diff:            /tmp/thrift-diff.txt"
fi
echo ""
echo "Done!"
