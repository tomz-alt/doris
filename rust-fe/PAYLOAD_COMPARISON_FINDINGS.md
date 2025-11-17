# Payload Comparison Investigation - Session Findings

**Date**: 2025-11-17
**Issue**: BE returns 0 rows for Rust FE despite scan ranges being loaded

## Key Discovery: Multiple Background Processes Contaminated Capture

### Initial Observation
- **Java FE payload**: 7,863 bytes (from pcap)
- **Rust FE payload**: 33,927 bytes (+331.5% larger!)
- **Rust FE** sending **4.3x more data** than Java FE

### Root Cause Analysis

**Packet count comparison**:
```
Java FE:  59 packets to BE (port 8060)
Rust FE: 214 packets to BE (port 8060)  ← 3.6x MORE!
```

**Conclusion**: The Rust FE capture included traffic from **multiple background rust-fe instances** that were still running from previous debugging sessions. This explains:
1. Why Rust payload is 4x larger
2. Why packet count is 3.6x higher
3. The oversized pcap files

### Corrective Action Required

**MUST do before any payload capture**:
```bash
# Kill ALL rust-fe and cargo processes
pkill -f "doris-rust-fe" && pkill -f "cargo run"
sleep 3

# Verify ports are free
lsof -i :9031  # Should exit with code 1 (nothing listening)
lsof -i :8031  # Should exit with code 1 (nothing listening)
```

## Working Payload Extraction & Comparison Scripts

### Created New Scripts ✅

**Location**: `scripts/`

1. **`extract_payload_simple.sh`**
   - Extracts TCP payloads from pcap files
   - Uses only tcpdump (no tshark dependency)
   - Filters for TCP traffic to BE port 8060

2. **`compare_payloads_simple.sh`**
   - Automated comparison of Java vs Rust payloads
   - Shows size comparison with percentage difference
   - Displays hex dumps of first 256 bytes
   - Lists byte-level differences
   - Works on macOS without GNU extensions

### Usage

```bash
# After clean capture of both Java and Rust FE:
cd /Users/zhhanz/Documents/velodb/doris/rust-fe
./scripts/compare_payloads_simple.sh
```

**Expected output**:
```
Java FE payload: ~7,863 bytes
Rust FE payload: Should be similar size if clean capture
Difference:      Should be < 20% if both are single queries
```

## Next Steps for Clean Comparison

### Step 1: Environment Cleanup (CRITICAL)
```bash
# Change to rust-fe directory
cd /Users/zhhanz/Documents/velodb/doris/rust-fe

# Kill ALL processes
pkill -f "doris-rust-fe"
pkill -f "cargo run"
sleep 3

# Verify clean state
ps aux | grep -E "(doris-rust-fe|cargo run)" | grep -v grep
# ↑ Should return NOTHING

lsof -i :9031 -i :8031
# ↑ Should exit with code 1 (no output)
```

### Step 2: Clean Java FE Capture
```bash
# Start tcpdump in BE container
docker exec -d doris-be tcpdump -i any -w /tmp/capture_java_clean.pcap "tcp port 8060"
sleep 2

# Execute ONE query from Java FE
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT * FROM tpch_sf1.lineitem LIMIT 3"

# Stop tcpdump
docker exec doris-be pkill tcpdump
sleep 2

# Copy to host
docker cp doris-be:/tmp/capture_java_clean.pcap /tmp/tpch_captures/
docker exec doris-be rm -f /tmp/capture_java_clean.pcap
```

### Step 3: Clean Rust FE Capture
```bash
# Start tcpdump in BE container
docker exec -d doris-be tcpdump -i any -w /tmp/capture_rust_clean.pcap "tcp port 8060"
sleep 2

# Start SINGLE Rust FE instance
RUST_LOG=info cargo run --features real_be_proto --bin doris-rust-fe 2>&1 | tee /tmp/rust-fe-clean.log &
RUST_PID=$!
sleep 15

# Execute ONE query from Rust FE
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT * FROM lineitem LIMIT 3"

# Stop Rust FE
kill $RUST_PID

# Stop tcpdump
docker exec doris-be pkill tcpdump
sleep 2

# Copy to host
docker cp doris-be:/tmp/capture_rust_clean.pcap /tmp/tpch_captures/
docker exec doris-be rm -f /tmp/capture_rust_clean.pcap
```

### Step 4: Compare Clean Payloads
```bash
# Rename old captures for backup
mv /tmp/tpch_captures/java_simple_scan.pcap /tmp/tpch_captures/java_simple_scan_OLD.pcap 2>/dev/null
mv /tmp/tpch_captures/rust_simple_scan.pcap /tmp/tpch_captures/rust_simple_scan_OLD.pcap 2>/dev/null

# Use new clean captures
cp /tmp/tpch_captures/capture_java_clean.pcap /tmp/tpch_captures/java_simple_scan.pcap
cp /tmp/tpch_captures/capture_rust_clean.pcap /tmp/tpch_captures/rust_simple_scan.pcap

# Run comparison
./scripts/compare_payloads_simple.sh
```

## Expected Results from Clean Comparison

If Rust FE payload is still significantly larger than Java FE:
- **+10-30%**: Likely due to extra fields, different Thrift encoding, or TDescriptorTable differences
- **+50-100%**: Possible duplicate scan ranges or extra fragment parameters
- **+200%+**: Still capturing multiple instances - repeat cleanup

If Rust FE payload is similar size (+/- 20%):
- Proceed to hex-level comparison to find specific field differences
- Look for missing or incorrect values in scan ranges
- Check TDescriptorTable structure

## Tools Documentation Updated

**File**: `tools.md`
- Added section on working payload extraction scripts
- Emphasized importance of killing all background processes
- Documented clean capture procedure

## Current Blocker Status

**Problem**: BE returns TResultBatch with 0 rows
**Progress**:
- ✅ Identified contaminated capture issue
- ✅ Created working extraction/comparison scripts
- ⏳ Need clean payload comparison to identify actual differences
- ⏳ Then fix whatever field(s) are incorrect in Rust FE

**Hypothesis**: Once we do a clean comparison, we'll likely find:
1. Incorrect table_id (hardcoded as `1`, may need real ID)
2. Missing or incorrect fields in TPaloScanRange
3. TDescriptorTable differences
4. Query options/globals differences
