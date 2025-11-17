# Quick Reference Tools

**Last Updated**: 2025-11-17

## 🚨 Critical: Process Management

```bash
# ALWAYS kill processes before starting new server
pkill -f "doris-rust-fe" && sleep 2

# Verify ports are free
lsof -i :9031  # Should return nothing (exit code 1)
```

## 🏗️ Build and Run

**CRITICAL**: Build WITH default features (do NOT use `--no-default-features`)

```bash
# Build
cargo build --features real_be_proto

# Run server
RUST_LOG=info cargo run --features real_be_proto -- --config fe_config.json

# Run in background with logging
RUST_LOG=info cargo run --bin doris-rust-fe --features real_be_proto \
  -- --config fe_config.json 2>&1 > /tmp/rust-fe.log &
sleep 15  # Wait for startup
```

## 🧪 Testing

```bash
# Test query (Rust FE)
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT * FROM tpch_sf1.lineitem LIMIT 3"

# Baseline query (Java FE)
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT * FROM tpch_sf1.lineitem LIMIT 3"

# Show databases/tables
mysql -h 127.0.0.1 -P 9031 -u root -e "SHOW DATABASES"
mysql -h 127.0.0.1 -P 9031 -u root -e "USE tpch_sf1; SHOW TABLES"
```

## 🔍 Debugging

```bash
# Check server logs
tail -f /tmp/rust-fe.log | grep -E "(ERROR|WARN|INFO)"

# Check tablet metadata
mysql -h 127.0.0.1 -P 9030 -u root -e "SHOW TABLETS FROM tpch_sf1.lineitem LIMIT 5"

# Verify Docker containers
docker ps --filter "name=doris"

# Check BE connectivity
docker exec doris-be ps aux | grep doris_be
```

## 📦 Payload Capture (Java FE vs Rust FE)

```bash
# 1. Capture Java FE baseline
docker exec -d doris-be tcpdump -i any -w /tmp/java.pcap "tcp port 8060"
sleep 2
docker exec doris-fe-java mysql -uroot -h127.0.0.1 -P9030 \
  -e "USE tpch_sf1; SELECT * FROM lineitem LIMIT 3"
docker exec doris-be pkill tcpdump && sleep 1
docker cp doris-be:/tmp/java.pcap /tmp/

# 2. Capture Rust FE
pkill -f "doris-rust-fe" && sleep 2  # Clean slate
docker exec -d doris-be tcpdump -i any -w /tmp/rust.pcap "tcp port 8060"
sleep 2
RUST_LOG=info cargo run --features real_be_proto -- --config fe_config.json &
sleep 15
mysql -h 127.0.0.1 -P 9031 -u root -e "SELECT * FROM tpch_sf1.lineitem LIMIT 3"
docker exec doris-be pkill tcpdump && sleep 1
docker cp doris-be:/tmp/rust.pcap /tmp/

# 3. Compare
ls -lh /tmp/*.pcap
/opt/homebrew/bin/tshark -r /tmp/java.pcap -qz io,phs
/opt/homebrew/bin/tshark -r /tmp/rust.pcap -qz io,phs
```

## 📊 Thrift Payload Decoder

```bash
# Decode Rust FE payload
cargo run --features real_be_proto --example decode_scan_ranges -- /tmp/rust-payload.bin

# Decode Java FE payload
cargo run --features real_be_proto --example decode_scan_ranges -- /tmp/java-payload.bin
```

## ⚠️ Current Issue

**Status**: BE returns 0 rows despite correct scan ranges

**Symptoms**:
- Scan ranges loaded: 3 tablets, correct IDs/version/schema_hash
- Fragment executes on BE without errors
- TResultBatch received but contains 0 rows

**Investigation**:
- Java FE payload: ~6KB, 3 fragments, 17 backends
- Rust FE payload: ~1.4KB, 1 fragment, 1 backend
- Missing: 2 fragments, proper plan nodes, replica distribution

**Next Steps**:
1. Implement proper EXCHANGE_NODE for fragment aggregation
2. Implement result node for fragment 2
3. Add tablet replica support (3 replicas per tablet)
4. Add multi-backend distribution

## 🗂️ Configuration

**fe_config.json**: MySQL port 9031, backend at 172.20.80.3:8060
**scan_ranges.json**: Tablet IDs [1763227464383, 1763227464385, 1763227464387], version 3

## 📝 Key Files

- `src/be/thrift_pipeline.rs` - Fragment/plan node generation
- `src/be/client.rs` - BE gRPC communication
- `src/planner/fragment_executor.rs` - Query execution
- `src/catalog/be_table.rs` - BE-backed tables
- `src/metadata/types.rs` - Type system

---

**See also**: SUCCESS_REPORT.md, TYPE_CASTING_FIX_REPORT.md, todo.md
