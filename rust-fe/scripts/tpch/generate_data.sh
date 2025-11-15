#!/bin/bash
# TPC-H Data Generation Script
# Generates TPC-H data using dbgen and loads into Doris

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Default values
SCALE_FACTOR=${SCALE_FACTOR:-1}
DBGEN_DIR=${DBGEN_DIR:-"/tmp/tpch-dbgen"}
DATA_DIR=${DATA_DIR:-"/tmp/tpch-data"}
FE_HOST=${FE_HOST:-"127.0.0.1"}
FE_PORT=${FE_PORT:-9030}
DATABASE=${DATABASE:-"tpch_sf${SCALE_FACTOR}"}

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

step() {
    echo -e "${BLUE}==>${NC} $1"
}

success() {
    echo -e "${GREEN}✓${NC} $1"
}

warn() {
    echo -e "${YELLOW}!${NC} $1"
}

error() {
    echo -e "${RED}✗${NC} $1"
}

usage() {
    cat <<EOF
TPC-H Data Generation Script

Usage: $0 [OPTIONS]

Options:
    -s, --scale FACTOR       Scale factor (default: 1)
                             SF1=1GB, SF10=10GB, SF100=100GB
    -d, --dbgen-dir DIR      Directory for TPC-H dbgen tool
    -o, --output-dir DIR     Output directory for generated data
    -h, --host HOST          FE host (default: 127.0.0.1)
    -p, --port PORT          FE MySQL port (default: 9030)
    --help                   Show this help

Examples:
    # Generate SF1 (1GB) data
    $0 -s 1

    # Generate SF10 data to custom directory
    $0 -s 10 -o /data/tpch-sf10

    # Generate and load into Rust FE
    $0 -s 1 -p 9031

Environment Variables:
    SCALE_FACTOR    Scale factor (same as -s)
    DBGEN_DIR       dbgen directory
    DATA_DIR        Output data directory
    FE_HOST         FE host
    FE_PORT         FE MySQL port

EOF
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -s|--scale)
            SCALE_FACTOR="$2"
            shift 2
            ;;
        -d|--dbgen-dir)
            DBGEN_DIR="$2"
            shift 2
            ;;
        -o|--output-dir)
            DATA_DIR="$2"
            shift 2
            ;;
        -h|--host)
            FE_HOST="$2"
            shift 2
            ;;
        -p|--port)
            FE_PORT="$2"
            shift 2
            ;;
        --help)
            usage
            ;;
        *)
            error "Unknown option: $1"
            usage
            ;;
    esac
done

DATABASE="tpch_sf${SCALE_FACTOR}"

echo "======================================"
echo "TPC-H Data Generation"
echo "======================================"
echo "Scale Factor:    SF${SCALE_FACTOR}"
echo "Database:        ${DATABASE}"
echo "Data Directory:  ${DATA_DIR}"
echo "FE Endpoint:     ${FE_HOST}:${FE_PORT}"
echo "======================================"
echo ""

# Step 1: Install/compile dbgen
step "Setting up TPC-H dbgen..."

if [ ! -d "$DBGEN_DIR" ]; then
    mkdir -p "$DBGEN_DIR"
    step "Cloning TPC-H dbgen repository..."

    if ! command -v git &> /dev/null; then
        error "git is not installed. Please install git first."
        exit 1
    fi

    git clone https://github.com/electrum/tpch-dbgen.git "$DBGEN_DIR"
    cd "$DBGEN_DIR"

    step "Compiling dbgen..."
    if ! command -v gcc &> /dev/null; then
        error "gcc is not installed. Please install gcc first."
        exit 1
    fi

    make
    success "dbgen compiled successfully"
else
    success "dbgen already exists at ${DBGEN_DIR}"
    cd "$DBGEN_DIR"
fi

# Step 2: Generate data
step "Generating TPC-H data (SF${SCALE_FACTOR})..."

mkdir -p "$DATA_DIR"

if [ -f "${DATA_DIR}/lineitem.tbl" ] && [ -s "${DATA_DIR}/lineitem.tbl" ]; then
    warn "Data files already exist in ${DATA_DIR}"
    read -p "Regenerate data? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        success "Using existing data files"
    else
        rm -f "${DATA_DIR}"/*.tbl
        ./dbgen -s "$SCALE_FACTOR" -T a -f
        mv *.tbl "$DATA_DIR/"
        success "Data generated in ${DATA_DIR}"
    fi
else
    ./dbgen -s "$SCALE_FACTOR" -T a -f
    mv *.tbl "$DATA_DIR/"
    success "Data generated in ${DATA_DIR}"
fi

# Show data file sizes
echo ""
step "Generated data files:"
du -sh "${DATA_DIR}"/*.tbl | sed 's/^/  /'
echo ""

# Step 3: Create database and tables
step "Creating database and tables..."

mysql -h "$FE_HOST" -P "$FE_PORT" -u root -e "DROP DATABASE IF EXISTS ${DATABASE};" 2>/dev/null || true
mysql -h "$FE_HOST" -P "$FE_PORT" -u root -e "CREATE DATABASE ${DATABASE};"

mysql -h "$FE_HOST" -P "$FE_PORT" -u root "$DATABASE" <<'EOF'
-- Customer table
CREATE TABLE customer (
    c_custkey INT NOT NULL,
    c_name VARCHAR(25) NOT NULL,
    c_address VARCHAR(40) NOT NULL,
    c_nationkey INT NOT NULL,
    c_phone CHAR(15) NOT NULL,
    c_acctbal DECIMAL(15,2) NOT NULL,
    c_mktsegment CHAR(10) NOT NULL,
    c_comment VARCHAR(117) NOT NULL
)
DUPLICATE KEY(c_custkey)
DISTRIBUTED BY HASH(c_custkey) BUCKETS 12
PROPERTIES ("replication_num" = "1");

-- Lineitem table
CREATE TABLE lineitem (
    l_orderkey INT NOT NULL,
    l_partkey INT NOT NULL,
    l_suppkey INT NOT NULL,
    l_linenumber INT NOT NULL,
    l_quantity DECIMAL(15,2) NOT NULL,
    l_extendedprice DECIMAL(15,2) NOT NULL,
    l_discount DECIMAL(15,2) NOT NULL,
    l_tax DECIMAL(15,2) NOT NULL,
    l_returnflag CHAR(1) NOT NULL,
    l_linestatus CHAR(1) NOT NULL,
    l_shipdate DATE NOT NULL,
    l_commitdate DATE NOT NULL,
    l_receiptdate DATE NOT NULL,
    l_shipinstruct CHAR(25) NOT NULL,
    l_shipmode CHAR(10) NOT NULL,
    l_comment VARCHAR(44) NOT NULL
)
DUPLICATE KEY(l_orderkey, l_partkey, l_suppkey, l_linenumber)
PARTITION BY RANGE(l_shipdate) (
    PARTITION p1992 VALUES LESS THAN ("1993-01-01"),
    PARTITION p1993 VALUES LESS THAN ("1994-01-01"),
    PARTITION p1994 VALUES LESS THAN ("1995-01-01"),
    PARTITION p1995 VALUES LESS THAN ("1996-01-01"),
    PARTITION p1996 VALUES LESS THAN ("1997-01-01"),
    PARTITION p1997 VALUES LESS THAN ("1998-01-01"),
    PARTITION p1998 VALUES LESS THAN ("1999-01-01")
)
DISTRIBUTED BY HASH(l_orderkey) BUCKETS 32
PROPERTIES ("replication_num" = "1");

-- Nation table
CREATE TABLE nation (
    n_nationkey INT NOT NULL,
    n_name CHAR(25) NOT NULL,
    n_regionkey INT NOT NULL,
    n_comment VARCHAR(152) NOT NULL
)
DUPLICATE KEY(n_nationkey)
DISTRIBUTED BY HASH(n_nationkey) BUCKETS 1
PROPERTIES ("replication_num" = "1");

-- Orders table
CREATE TABLE orders (
    o_orderkey INT NOT NULL,
    o_custkey INT NOT NULL,
    o_orderstatus CHAR(1) NOT NULL,
    o_totalprice DECIMAL(15,2) NOT NULL,
    o_orderdate DATE NOT NULL,
    o_orderpriority CHAR(15) NOT NULL,
    o_clerk CHAR(15) NOT NULL,
    o_shippriority INT NOT NULL,
    o_comment VARCHAR(79) NOT NULL
)
DUPLICATE KEY(o_orderkey)
PARTITION BY RANGE(o_orderdate) (
    PARTITION p1992 VALUES LESS THAN ("1993-01-01"),
    PARTITION p1993 VALUES LESS THAN ("1994-01-01"),
    PARTITION p1994 VALUES LESS THAN ("1995-01-01"),
    PARTITION p1995 VALUES LESS THAN ("1996-01-01"),
    PARTITION p1996 VALUES LESS THAN ("1997-01-01"),
    PARTITION p1997 VALUES LESS THAN ("1998-01-01"),
    PARTITION p1998 VALUES LESS THAN ("1999-01-01")
)
DISTRIBUTED BY HASH(o_orderkey) BUCKETS 32
PROPERTIES ("replication_num" = "1");

-- Part table
CREATE TABLE part (
    p_partkey INT NOT NULL,
    p_name VARCHAR(55) NOT NULL,
    p_mfgr CHAR(25) NOT NULL,
    p_brand CHAR(10) NOT NULL,
    p_type VARCHAR(25) NOT NULL,
    p_size INT NOT NULL,
    p_container CHAR(10) NOT NULL,
    p_retailprice DECIMAL(15,2) NOT NULL,
    p_comment VARCHAR(23) NOT NULL
)
DUPLICATE KEY(p_partkey)
DISTRIBUTED BY HASH(p_partkey) BUCKETS 12
PROPERTIES ("replication_num" = "1");

-- Partsupp table
CREATE TABLE partsupp (
    ps_partkey INT NOT NULL,
    ps_suppkey INT NOT NULL,
    ps_availqty INT NOT NULL,
    ps_supplycost DECIMAL(15,2) NOT NULL,
    ps_comment VARCHAR(199) NOT NULL
)
DUPLICATE KEY(ps_partkey, ps_suppkey)
DISTRIBUTED BY HASH(ps_partkey) BUCKETS 12
PROPERTIES ("replication_num" = "1");

-- Region table
CREATE TABLE region (
    r_regionkey INT NOT NULL,
    r_name CHAR(25) NOT NULL,
    r_comment VARCHAR(152) NOT NULL
)
DUPLICATE KEY(r_regionkey)
DISTRIBUTED BY HASH(r_regionkey) BUCKETS 1
PROPERTIES ("replication_num" = "1");

-- Supplier table
CREATE TABLE supplier (
    s_suppkey INT NOT NULL,
    s_name CHAR(25) NOT NULL,
    s_address VARCHAR(40) NOT NULL,
    s_nationkey INT NOT NULL,
    s_phone CHAR(15) NOT NULL,
    s_acctbal DECIMAL(15,2) NOT NULL,
    s_comment VARCHAR(101) NOT NULL
)
DUPLICATE KEY(s_suppkey)
DISTRIBUTED BY HASH(s_suppkey) BUCKETS 12
PROPERTIES ("replication_num" = "1");
EOF

success "Tables created in database ${DATABASE}"

# Step 4: Load data using Stream Load
step "Loading data into tables..."

load_table() {
    local table=$1
    local file="${DATA_DIR}/${table}.tbl"

    if [ ! -f "$file" ]; then
        error "Data file not found: $file"
        return 1
    fi

    step "Loading ${table}..."

    # Use Stream Load API
    local http_port=$((FE_PORT - 1000))  # 9030 -> 8030, 9031 -> 8031

    curl --location-trusted \
        -u root: \
        -H "column_separator:|" \
        -H "columns: $(get_columns $table)" \
        -T "$file" \
        "http://${FE_HOST}:${http_port}/api/${DATABASE}/${table}/_stream_load" \
        2>/dev/null | python3 -m json.tool || true

    # Verify row count
    local count=$(mysql -h "$FE_HOST" -P "$FE_PORT" -u root -N -e "SELECT COUNT(*) FROM ${DATABASE}.${table};")
    success "${table}: ${count} rows loaded"
}

get_columns() {
    case $1 in
        customer)
            echo "c_custkey,c_name,c_address,c_nationkey,c_phone,c_acctbal,c_mktsegment,c_comment"
            ;;
        lineitem)
            echo "l_orderkey,l_partkey,l_suppkey,l_linenumber,l_quantity,l_extendedprice,l_discount,l_tax,l_returnflag,l_linestatus,l_shipdate,l_commitdate,l_receiptdate,l_shipinstruct,l_shipmode,l_comment"
            ;;
        nation)
            echo "n_nationkey,n_name,n_regionkey,n_comment"
            ;;
        orders)
            echo "o_orderkey,o_custkey,o_orderstatus,o_totalprice,o_orderdate,o_orderpriority,o_clerk,o_shippriority,o_comment"
            ;;
        part)
            echo "p_partkey,p_name,p_mfgr,p_brand,p_type,p_size,p_container,p_retailprice,p_comment"
            ;;
        partsupp)
            echo "ps_partkey,ps_suppkey,ps_availqty,ps_supplycost,ps_comment"
            ;;
        region)
            echo "r_regionkey,r_name,r_comment"
            ;;
        supplier)
            echo "s_suppkey,s_name,s_address,s_nationkey,s_phone,s_acctbal,s_comment"
            ;;
    esac
}

# Load all tables
for table in region nation customer supplier part partsupp orders lineitem; do
    load_table "$table"
done

echo ""
success "Data loading complete!"

# Step 5: Verify data
step "Verifying loaded data..."
echo ""

mysql -h "$FE_HOST" -P "$FE_PORT" -u root "$DATABASE" <<'EOF'
SELECT
    'customer' as table_name, COUNT(*) as row_count FROM customer
UNION ALL
SELECT 'lineitem', COUNT(*) FROM lineitem
UNION ALL
SELECT 'nation', COUNT(*) FROM nation
UNION ALL
SELECT 'orders', COUNT(*) FROM orders
UNION ALL
SELECT 'part', COUNT(*) FROM part
UNION ALL
SELECT 'partsupp', COUNT(*) FROM partsupp
UNION ALL
SELECT 'region', COUNT(*) FROM region
UNION ALL
SELECT 'supplier', COUNT(*) FROM supplier
ORDER BY table_name;
EOF

echo ""
success "TPC-H data generation complete!"
echo ""
echo "======================================"
echo "Summary"
echo "======================================"
echo "Database:        ${DATABASE}"
echo "Scale Factor:    SF${SCALE_FACTOR}"
echo "Data Location:   ${DATA_DIR}"
echo "FE Endpoint:     ${FE_HOST}:${FE_PORT}"
echo ""
echo "Next steps:"
echo "  1. Run benchmarks:"
echo "     python3 scripts/benchmark_tpch.py --scale ${SCALE_FACTOR}"
echo ""
echo "  2. Query the data:"
echo "     mysql -h ${FE_HOST} -P ${FE_PORT} -u root ${DATABASE}"
echo "======================================"
