# Quick Start Guide

Get the Doris Rust FE running in 5 minutes!

## 1. Install Dependencies

### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install -y build-essential protobuf-compiler pkg-config libssl-dev
```

### macOS
```bash
brew install protobuf openssl
```

### Install Rust (if not already installed)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## 2. Build

```bash
cd rust-fe
cargo build --release
```

Expected output:
```
   Compiling doris-rust-fe v0.1.0
    Finished release [optimized] target(s) in 2m 15s
```

## 3. Run

```bash
cargo run --release
```

You should see:
```
Doris Rust FE started successfully
MySQL server listening on port 9030
HTTP server listening on port 8030
```

## 4. Verify - MySQL Protocol

Open a new terminal and test MySQL connection:

```bash
# Simple connection test
mysql -h 127.0.0.1 -P 9030 -u root -e "SELECT 1"

# Interactive session
mysql -h 127.0.0.1 -P 9030 -u root
```

Try some queries:
```sql
SELECT 1;
SHOW DATABASES;
USE test;
SHOW TABLES;
SELECT * FROM example_table;
```

## 5. Verify - Stream Load

Test stream load with sample data:

```bash
curl -X PUT \
  -H "label: quickstart_load" \
  -H "format: csv" \
  --data-binary @tests/test_data.csv \
  "http://127.0.0.1:8030/api/test/products/_stream_load" | jq .
```

Expected response:
```json
{
  "TxnId": "...",
  "Label": "quickstart_load",
  "Status": "Success",
  "NumberLoadedRows": 5,
  "LoadBytes": 102
}
```

## 6. Check Status

```bash
# Health check
curl http://127.0.0.1:8030/api/health | jq .

# Service status
curl http://127.0.0.1:8030/api/status | jq .
```

## Common Issues

### Issue: "protobuf compiler not found"
**Solution**: Install protoc
```bash
# Ubuntu/Debian
sudo apt-get install protobuf-compiler

# macOS
brew install protobuf
```

### Issue: "could not find libssl"
**Solution**: Install OpenSSL development files
```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# macOS
brew install openssl
export PKG_CONFIG_PATH="/usr/local/opt/openssl/lib/pkgconfig"
```

### Issue: "Address already in use"
**Solution**: Change ports in `fe_config.json`
```json
{
  "mysql_port": 9031,
  "http_port": 8031,
  ...
}
```

### Issue: "Connection refused" when connecting to BE
**Solution**: This is expected for PoC without running BE nodes. The FE will return mock data.

## Next Steps

1. **Run automated tests**:
   ```bash
   ./tests/verify_mysql.sh
   ./tests/verify_stream_load.sh
   ./tests/load_test.sh
   ```

2. **Configure backend nodes** (if you have Doris BE running):
   Edit `fe_config.json`:
   ```json
   {
     "backend_nodes": [
       {
         "host": "your-be-host",
         "port": 9060,
         "grpc_port": 9070
       }
     ]
   }
   ```

3. **Explore the code**:
   - MySQL protocol: `src/mysql/`
   - Query execution: `src/query/`
   - Stream load: `src/http/`
   - BE communication: `src/be/`

4. **Check the full README** for advanced features and migration guides

## Performance Testing

Test query queuing with concurrent connections:

```bash
# 10 concurrent connections, 100 queries each
CONCURRENT=10 QUERIES=100 ./tests/load_test.sh
```

## Stopping the Service

Press `Ctrl+C` in the terminal running the service.

## Need Help?

- Check the full [README.md](README.md)
- Review [OPENSRV_MIGRATION.md](OPENSRV_MIGRATION.md) for production deployment
- Review [POSTGRES_EXTENSION.md](POSTGRES_EXTENSION.md) for PostgreSQL integration

Happy querying! 🚀
