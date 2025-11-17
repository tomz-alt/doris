#!/bin/bash
# Compare Java FE vs Rust FE Thrift payloads
# Usage: ./compare_payloads_simple.sh

set -e

CAPTURES_DIR="/tmp/tpch_captures"
JAVA_PCAP="$CAPTURES_DIR/java_simple_scan.pcap"
RUST_PCAP="$CAPTURES_DIR/rust_simple_scan.pcap"
JAVA_PAYLOAD="$CAPTURES_DIR/java_payload.bin"
RUST_PAYLOAD="$CAPTURES_DIR/rust_payload.bin"

echo "═══════════════════════════════════════════════════════════════"
echo "  Doris Thrift Payload Comparison: Java FE vs Rust FE"
echo "═══════════════════════════════════════════════════════════════"
echo

# Check if PCAP files exist
if [ ! -f "$JAVA_PCAP" ]; then
    echo "✗ Java FE capture not found: $JAVA_PCAP"
    echo "  Run capture first with clean Java FE query"
    exit 1
fi

if [ ! -f "$RUST_PCAP" ]; then
    echo "✗ Rust FE capture not found: $RUST_PCAP"
    echo "  Run capture first with clean Rust FE query"
    exit 1
fi

# Extract payloads
echo "📦 Extracting payloads from PCAP files..."
echo

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"$SCRIPT_DIR/extract_payload_simple.sh" "$JAVA_PCAP" "$JAVA_PAYLOAD"
"$SCRIPT_DIR/extract_payload_simple.sh" "$RUST_PCAP" "$RUST_PAYLOAD"

echo
echo "───────────────────────────────────────────────────────────────"
echo "  Size Comparison"
echo "───────────────────────────────────────────────────────────────"

JAVA_SIZE=$(wc -c < "$JAVA_PAYLOAD")
RUST_SIZE=$(wc -c < "$RUST_PAYLOAD")

printf "Java FE payload: %'d bytes\n" $JAVA_SIZE
printf "Rust FE payload: %'d bytes\n" $RUST_SIZE

if [ $RUST_SIZE -gt $JAVA_SIZE ]; then
    DIFF=$((RUST_SIZE - JAVA_SIZE))
    PERCENT=$(awk "BEGIN {printf \"%.1f\", ($RUST_SIZE - $JAVA_SIZE) * 100.0 / $JAVA_SIZE}")
    printf "Difference:      +%'d bytes (+%s%%) ⚠️  RUST IS LARGER\n" $DIFF "$PERCENT"
elif [ $RUST_SIZE -lt $JAVA_SIZE ]; then
    DIFF=$((JAVA_SIZE - RUST_SIZE))
    PERCENT=$(awk "BEGIN {printf \"%.1f\", ($JAVA_SIZE - $RUST_SIZE) * 100.0 / $JAVA_SIZE}")
    printf "Difference:      -%'d bytes (-%s%%) ⚠️  RUST IS SMALLER\n" $DIFF "$PERCENT"
else
    echo "Difference:      0 bytes (identical size) ✓"
fi

echo
echo "───────────────────────────────────────────────────────────────"
echo "  First 256 Bytes (Hex Dump)"
echo "───────────────────────────────────────────────────────────────"
echo
echo "Java FE:"
xxd -g 1 -l 256 "$JAVA_PAYLOAD" | head -16

echo
echo "Rust FE:"
xxd -g 1 -l 256 "$RUST_PAYLOAD" | head -16

echo
echo "───────────────────────────────────────────────────────────────"
echo "  Binary Difference (first 50 divergences)"
echo "───────────────────────────────────────────────────────────────"
echo

if cmp -s "$JAVA_PAYLOAD" "$RUST_PAYLOAD"; then
    echo "✓ Payloads are IDENTICAL"
else
    echo "Byte offset | Java (hex) | Rust (hex) | Java (oct) | Rust (oct)"
    echo "---------------------------------------------------------------"
    cmp -l "$JAVA_PAYLOAD" "$RUST_PAYLOAD" 2>/dev/null | head -50 | \
        awk '{printf "%11d | 0x%08x | 0x%08x | %10o | %10o\n", $1, strtonum("0"$2), strtonum("0"$3), strtonum("0"$2), strtonum("0"$3)}'

    TOTAL_DIFFS=$(cmp -l "$JAVA_PAYLOAD" "$RUST_PAYLOAD" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$TOTAL_DIFFS" -gt 50 ]; then
        echo "... (showing first 50 of $TOTAL_DIFFS total differences)"
    fi
fi

echo
echo "═══════════════════════════════════════════════════════════════"
echo "  Payload files saved:"
echo "  - Java: $JAVA_PAYLOAD"
echo "  - Rust: $RUST_PAYLOAD"
echo "═══════════════════════════════════════════════════════════════"
