#!/usr/bin/env bash
set -euo pipefail

# Simple MySQL CLI integration check against the Rust FE.
#
# Modes:
#   - Default: assume `doris-rust-fe` is already running and listening on
#     FE_QUERY_PORT (or 9030).
#   - Managed: with --start-fe, this script will:
#       * pick an ephemeral MySQL port
#       * start the FE binary on that port
#       * wait for readiness
#       * run the checks
#       * shut down the FE on exit

MANAGED_FE=false
FE_BIN_DEFAULT="./target/debug/doris-rust-fe"
FE_BIN="${FE_BIN:-$FE_BIN_DEFAULT}"
HOST="127.0.0.1"
USER="${MYSQL_USER:-root}"
DB="${MYSQL_DB:-tpch}"
TIMEOUT_BIN="timeout"
FE_PID=""

usage() {
  cat <<EOF
Usage: scripts/mysql_cli_integration.sh [--start-fe] [--fe-bin PATH]

Options:
  --start-fe       Start and stop the Rust FE for this check.
                   Uses an ephemeral MySQL port and FE_QUERY_PORT env.
  --fe-bin PATH    Path to the doris-rust-fe binary
                   (default: ${FE_BIN_DEFAULT}).
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --start-fe)
      MANAGED_FE=true
      shift
      ;;
    --fe-bin)
      if [[ $# -lt 2 ]]; then
        echo "[mysql-cli-integration] --fe-bin requires a path" >&2
        exit 1
      fi
      FE_BIN="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[mysql-cli-integration] Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if ! command -v mysql >/dev/null 2>&1; then
  echo "[mysql-cli-integration] mysql client not found in PATH" >&2
  exit 1
fi

if ! command -v "${TIMEOUT_BIN}" >/dev/null 2>&1; then
  echo "[mysql-cli-integration] 'timeout' command not found; please install coreutils/gnu timeout" >&2
  exit 1
fi

pick_free_port() {
  # Use python3 to ask the OS for an ephemeral TCP port.
  if command -v python3 >/dev/null 2>&1; then
    python3 - <<'EOF'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
port = s.getsockname()[1]
s.close()
print(port)
EOF
  else
    # Fallback: use default FE_QUERY_PORT or 9030.
    echo "${FE_QUERY_PORT:-9030}"
  fi
}

cleanup() {
  if [[ -n "${FE_PID}" ]]; then
    echo "[mysql-cli-integration] Stopping FE (pid=${FE_PID})" >&2
    kill "${FE_PID}" >/dev/null 2>&1 || true
    wait "${FE_PID}" >/dev/null 2>&1 || true
  fi
}

trap cleanup EXIT

start_fe_if_needed() {
  if [[ "${MANAGED_FE}" != "true" ]]; then
    return
  fi

  if [[ ! -x "${FE_BIN}" ]]; then
    echo "[mysql-cli-integration] FE binary not found or not executable at: ${FE_BIN}" >&2
    echo "[mysql-cli-integration] Build it with: cargo build --bin doris-rust-fe" >&2
    exit 1
  fi

  local port
  port=$(pick_free_port)
  export FE_QUERY_PORT="${port}"
  # HTTP port is not used in this script, but set to a nearby port for consistency.
  export FE_HTTP_PORT="$((port + 1000))"

  echo "[mysql-cli-integration] Starting FE: ${FE_BIN} (FE_QUERY_PORT=${FE_QUERY_PORT}, FE_HTTP_PORT=${FE_HTTP_PORT})" >&2
  "${FE_BIN}" > /tmp/mysql_cli_integration_fe.log 2>&1 &
  FE_PID=$!

  # Wait for FE to become ready by polling SELECT 1
  local max_tries=30
  local i
  for i in $(seq 1 "${max_tries}"); do
    if timeout 2s mysql -h"${HOST}" -P"${FE_QUERY_PORT}" -u"${USER}" -e "SELECT 1;" >/dev/null 2>&1; then
      echo "[mysql-cli-integration] FE is ready on port ${FE_QUERY_PORT}" >&2
      break
    fi
    sleep 1
  done

  if [[ "${i}" -ge "${max_tries}" ]]; then
    echo "[mysql-cli-integration] FE did not become ready in time (see /tmp/mysql_cli_integration_fe.log)" >&2
    exit 1
  fi
}

start_fe_if_needed

PORT="${FE_QUERY_PORT:-9030}"

echo "[mysql-cli-integration] Using host=${HOST} port=${PORT} user=${USER} db=${DB} managed_fe=${MANAGED_FE}" >&2

run_mysql() {
  local desc="$1"; shift
  echo "[mysql-cli-integration] ${desc}" >&2
  "${TIMEOUT_BIN}" 5s mysql -h"${HOST}" -P"${PORT}" -u"${USER}" "$@"
}

# 1. Basic connectivity: SELECT 1
run_mysql "SELECT 1" -D"${DB}" -e "SELECT 1;" >/tmp/mysql_cli_integration_select1.out

if ! grep -q "1" /tmp/mysql_cli_integration_select1.out; then
  echo "[mysql-cli-integration] SELECT 1 did not return expected result" >&2
  exit 1
fi

# 2. SHOW DATABASES should include tpch
run_mysql "SHOW DATABASES" -e "SHOW DATABASES;" >/tmp/mysql_cli_integration_dbs.out

if ! tail -n +2 /tmp/mysql_cli_integration_dbs.out | grep -qx "tpch"; then
  echo "[mysql-cli-integration] tpch database not found in SHOW DATABASES" >&2
  exit 1
fi

# 3. SHOW TABLES in tpch should include key TPC-H tables
run_mysql "SHOW TABLES in ${DB}" -D"${DB}" -e "SHOW TABLES;" >/tmp/mysql_cli_integration_tables.out

TABLES_BODY=$(tail -n +2 /tmp/mysql_cli_integration_tables.out || true)

for tbl in lineitem orders; do
  if ! printf '%s\\n' "${TABLES_BODY}" | grep -qx "${tbl}"; then
    echo "[mysql-cli-integration] table '${tbl}' not found in SHOW TABLES" >&2
    exit 1
  fi
done

# 4. DESCRIBE lineitem should match catalog-backed TPC-H schema
run_mysql "DESCRIBE ${DB}.lineitem" -D"${DB}" -B -N -e "DESC lineitem;" \
  >/tmp/mysql_cli_integration_desc_lineitem.out

mapfile -t DESC_LINES < /tmp/mysql_cli_integration_desc_lineitem.out || true

EXPECTED_FIELDS=(
  l_orderkey
  l_partkey
  l_suppkey
  l_linenumber
  l_quantity
  l_extendedprice
  l_discount
  l_tax
  l_returnflag
  l_linestatus
  l_shipdate
  l_commitdate
  l_receiptdate
  l_shipinstruct
  l_shipmode
  l_comment
)

EXPECTED_TYPES=(
  INT
  INT
  INT
  INT
  "DECIMAL(15, 2)"
  "DECIMAL(15, 2)"
  "DECIMAL(15, 2)"
  "DECIMAL(15, 2)"
  "VARCHAR(1)"
  "VARCHAR(1)"
  DATE
  DATE
  DATE
  "VARCHAR(25)"
  "VARCHAR(10)"
  "VARCHAR(44)"
)

if [[ ${#DESC_LINES[@]} -ne ${#EXPECTED_FIELDS[@]} ]]; then
  echo "[mysql-cli-integration] DESCRIBE lineitem returned ${#DESC_LINES[@]} columns, expected ${#EXPECTED_FIELDS[@]}" >&2
  exit 1
fi

for i in "${!EXPECTED_FIELDS[@]}"; do
  line="${DESC_LINES[$i]}"
  field=$(awk -F"\t" '{print $1}' <<< "${line}")
  type=$(awk -F"\t" '{print $2}' <<< "${line}")

  if [[ "${field}" != "${EXPECTED_FIELDS[$i]}" ]]; then
    echo "[mysql-cli-integration] DESCRIBE lineitem field mismatch at position $((i+1)): got='${field}', expected='${EXPECTED_FIELDS[$i]}'" >&2
    exit 1
  fi

  if [[ "${type}" != "${EXPECTED_TYPES[$i]}" ]]; then
    echo "[mysql-cli-integration] DESCRIBE lineitem type mismatch for field '${field}': got='${type}', expected='${EXPECTED_TYPES[$i]}'" >&2
    exit 1
  fi
done

echo "[mysql-cli-integration] All checks passed." >&2
