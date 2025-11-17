#!/usr/bin/env bash
#
# Smoke test wiring between Rust FE PBlock encoding and a running Doris BE.
# This script:
#   - Assumes BE has been built via ./scripts/build_be_and_rust_fe.sh
#   - Starts a BE instance (using existing Doris scripts/config)
#   - Builds and starts the Rust FE (real_be_proto)
#   - Uses mysql CLI to create a simple test table and insert a few rows
#     via Rust FE, exercising the PBlock path end-to-end.
#
# NOTE:
#   - This script is intentionally minimal and may require environment-
#     specific tuning (ports, paths, configs).
#   - It does not modify any BE code; it only calls existing scripts.
#
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DORIS_ROOT="${ROOT_DIR}/.."

FE_HOST="${FE_HOST:-127.0.0.1}"
FE_QUERY_PORT="${FE_QUERY_PORT:-9030}"
RUST_FE_BIN="${RUST_FE_BIN:-${ROOT_DIR}/target/debug/doris-rust-fe}"
RUST_FE_CONFIG="${RUST_FE_CONFIG:-${ROOT_DIR}/fe_config.json}"

echo "==> Starting Doris BE (if not already running)"
(
  cd "${DORIS_ROOT}"
  if [[ -x "./bin/start_be.sh" ]]; then
    ./bin/start_be.sh --daemon || true
  else
    echo "WARN: ./bin/start_be.sh not found; ensure BE is running manually."
  fi
)

echo "==> Starting Rust FE (real_be_proto) in background"
(
  cd "${ROOT_DIR}"
  if [[ ! -x "${RUST_FE_BIN}" ]]; then
    echo "Rust FE binary not found at ${RUST_FE_BIN}, building..."
    cargo build --features real_be_proto
  fi
  # Run FE in the background; caller is responsible for stopping it.
  "${RUST_FE_BIN}" --config "${RUST_FE_CONFIG}" &
  RUST_FE_PID=$!
  echo "${RUST_FE_PID}" > "${ROOT_DIR}/target/rust_fe.pid"
)

echo "==> Waiting a few seconds for Rust FE to accept MySQL connections..."
sleep 5

echo "==> Creating test database/table via Rust FE (MySQL protocol)"
mysql -h "${FE_HOST}" -P "${FE_QUERY_PORT}" -uroot -e "
  CREATE DATABASE IF NOT EXISTS pblock_test;
  DROP TABLE IF EXISTS pblock_test.simple;
  CREATE TABLE pblock_test.simple (
    id INT NOT NULL,
    name VARCHAR(10) NOT NULL,
    price DECIMAL(15,2) NOT NULL,
    shipdate DATE NOT NULL,
    shipts DATETIME NOT NULL
  ) ENGINE=OLAP
  DUPLICATE KEY(id)
  DISTRIBUTED BY HASH(id) BUCKETS 1
  PROPERTIES('replication_num' = '1');
"

echo "==> Inserting rows via Rust FE (PBlock path)"
mysql -h "${FE_HOST}" -P "${FE_QUERY_PORT}" -uroot -e "
  INSERT INTO pblock_test.simple (id, name, price, shipdate, shipts) VALUES
    (1, 'alice', 12.34, '2024-11-05', '2024-11-05 01:02:03'),
    (2, 'bob',   -0.56, '2024-11-06', '2024-11-06 04:05:06');
  SELECT * FROM pblock_test.simple ORDER BY id;
"

echo "==> Smoke test complete. Rust FE PID is recorded in target/rust_fe.pid (stop it when done)."

