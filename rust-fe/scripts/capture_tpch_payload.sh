#!/bin/bash
#
# TPC-H Payload Capture Script
# Captures Thrift payloads from FE → BE communication for comparison
#

set -e

CAPTURE_DIR="/tmp/tpch_captures"
mkdir -p "$CAPTURE_DIR"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

usage() {
    echo "Usage: $0 <java|rust> <query_name> <sql_query>"
    echo ""
    echo "Example:"
    echo "  $0 java simple_scan \"SELECT * FROM tpch_sf1.lineitem LIMIT 3\""
    echo "  $0 rust simple_scan \"SELECT * FROM tpch_sf1.lineitem LIMIT 3\""
    exit 1
}

if [ $# -lt 3 ]; then
    usage
fi

FE_TYPE="$1"
QUERY_NAME="$2"
SQL_QUERY="$3"

PCAP_FILE="$CAPTURE_DIR/${FE_TYPE}_${QUERY_NAME}.pcap"
PAYLOAD_FILE="$CAPTURE_DIR/${FE_TYPE}_${QUERY_NAME}_payload.bin"

echo -e "${BLUE}==> Capturing ${FE_TYPE} FE payload for: ${QUERY_NAME}${NC}"
echo -e "${BLUE}    SQL: ${SQL_QUERY}${NC}"

# Stop any existing tcpdump
echo -e "${GREEN}1. Stopping any existing tcpdump...${NC}"
docker exec doris-be pkill tcpdump 2>/dev/null || true
sleep 1

# Start fresh tcpdump
echo -e "${GREEN}2. Starting tcpdump on BE container...${NC}"
docker exec -d doris-be tcpdump -i any -w /tmp/capture.pcap "tcp port 8060" 2>&1
sleep 2

# Execute query based on FE type
echo -e "${GREEN}3. Executing query from ${FE_TYPE} FE...${NC}"
if [ "$FE_TYPE" = "java" ]; then
    # Connect to Java FE MySQL interface and run query
    # Assuming Java FE is running on port 9030
    mysql -h 127.0.0.1 -P 9030 -u root --protocol=tcp -e "$SQL_QUERY" 2>&1 | head -20
elif [ "$FE_TYPE" = "rust" ]; then
    # Run Rust FE with the query
    # This will need to be adjusted based on how you run Rust FE
    echo -e "${RED}    Note: Rust FE execution needs to be configured${NC}"
    echo -e "${RED}    Please run your Rust FE client with: ${SQL_QUERY}${NC}"
    read -p "    Press Enter after query completes..."
else
    echo -e "${RED}Invalid FE type: ${FE_TYPE}${NC}"
    usage
fi

# Stop tcpdump
echo -e "${GREEN}4. Stopping tcpdump...${NC}"
docker exec doris-be pkill tcpdump 2>/dev/null || true
sleep 1

# Copy pcap from container
echo -e "${GREEN}5. Copying pcap from BE container...${NC}"
docker cp doris-be:/tmp/capture.pcap "$PCAP_FILE"
docker exec doris-be rm -f /tmp/capture.pcap

# Extract Thrift payload using tshark
echo -e "${GREEN}6. Extracting Thrift payload...${NC}"
if command -v tshark &> /dev/null; then
    # Extract TCP payload from BE port 8060 (FE → BE communication)
    tshark -r "$PCAP_FILE" -Y "tcp.dstport==8060 && tcp.len>0" -T fields -e tcp.payload | \
        xxd -r -p > "$PAYLOAD_FILE" 2>/dev/null || \
        echo -e "${RED}    Warning: Could not extract payload with tshark${NC}"
else
    echo -e "${RED}    tshark not found. Please install Wireshark or extract manually.${NC}"
fi

# Summary
echo -e "${BLUE}==> Capture complete!${NC}"
echo -e "    PCAP:    ${PCAP_FILE}"
if [ -f "$PAYLOAD_FILE" ]; then
    PAYLOAD_SIZE=$(wc -c < "$PAYLOAD_FILE")
    echo -e "    Payload: ${PAYLOAD_FILE} (${PAYLOAD_SIZE} bytes)"
    echo -e "${GREEN}    First 64 bytes (hex):${NC}"
    xxd -g 1 -l 64 "$PAYLOAD_FILE"
else
    echo -e "${RED}    Payload extraction failed - manual extraction needed${NC}"
fi
