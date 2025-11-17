#!/bin/bash
# Simple payload extractor using tcpdump (no tshark needed)
# Usage: ./extract_payload_simple.sh <pcap_file> <output_file>

set -e

if [ $# -ne 2 ]; then
    echo "Usage: $0 <pcap_file> <output_file>"
    echo "Example: $0 /tmp/capture.pcap /tmp/payload.bin"
    exit 1
fi

PCAP_FILE="$1"
OUTPUT_FILE="$2"

if [ ! -f "$PCAP_FILE" ]; then
    echo "Error: PCAP file not found: $PCAP_FILE"
    exit 1
fi

echo "Extracting TCP payload from $PCAP_FILE..."
echo "Target: port 8060 (BE gRPC port)"

# Use tcpdump to extract hex payload, filter for port 8060
# -r: read from file
# -A: print packet in ASCII (we'll extract hex separately)
# -x: print packet in hex
# tcp dst port 8060: filter for packets going TO the BE

tcpdump -r "$PCAP_FILE" -x -n 'tcp dst port 8060 and (tcp[tcpflags] & tcp-push != 0)' 2>/dev/null | \
    grep -E '^\s+0x[0-9a-f]{4}:' | \
    sed 's/^\s*0x[0-9a-f]\{4\}:\s*//' | \
    tr -d ' ' | \
    tr -d '\n' | \
    xxd -r -p > "$OUTPUT_FILE"

if [ -s "$OUTPUT_FILE" ]; then
    SIZE=$(wc -c < "$OUTPUT_FILE")
    echo "✓ Extracted $SIZE bytes to $OUTPUT_FILE"
else
    echo "✗ Failed to extract payload (output file is empty)"
    exit 1
fi
