#!/bin/bash
# Extract TCP payload from pcap using tcpdump hex output

PCAP_FILE="$1"
OUTPUT_FILE="$2"
DST_PORT="${3:-8060}"

if [ -z "$PCAP_FILE" ] || [ -z "$OUTPUT_FILE" ]; then
    echo "Usage: $0 <pcap_file> <output_file> [dst_port]"
    exit 1
fi

echo "Extracting TCP payloads to port $DST_PORT from $PCAP_FILE..."

# Use tcpdump to get hex output, filter for destination port
# Extract only the hex bytes (remove addresses and ASCII)
tcpdump -r "$PCAP_FILE" -nn -x "tcp dst port $DST_PORT and greater 100" 2>/dev/null | \
    awk '
    BEGIN { in_packet = 0; }
    /^[0-9]{2}:[0-9]{2}:[0-9]{2}/ {
        # New packet header
        in_packet = 1;
        next;
    }
    /^\t0x[0-9a-f]{4}:/ {
        # Hex line
        if (in_packet) {
            # Extract hex bytes (columns 2-9)
            for (i=2; i<=9; i++) {
                if ($i ~ /^[0-9a-f]{4}$/) {
                    print $i;
                }
            }
        }
    }
    /^$/ {
        in_packet = 0;
    }
    ' | \
    tr -d '\n' | \
    xxd -r -p > "$OUTPUT_FILE"

SIZE=$(wc -c < "$OUTPUT_FILE")
echo "✓ Extracted $SIZE bytes to $OUTPUT_FILE"

if [ $SIZE -gt 0 ]; then
    echo ""
    echo "First 128 bytes (hex):"
    xxd -g 1 -l 128 "$OUTPUT_FILE"
fi
