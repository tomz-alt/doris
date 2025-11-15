#!/bin/bash
# TPC-DS Data Generation Script
# Generates TPC-DS data using dsdgen and loads into Doris

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Default values
SCALE_FACTOR=${SCALE_FACTOR:-1}
DSDGEN_DIR=${DSDGEN_DIR:-"/tmp/tpcds-kit"}
DATA_DIR=${DATA_DIR:-"/tmp/tpcds-data"}
FE_HOST=${FE_HOST:-"127.0.0.1"}
FE_PORT=${FE_PORT:-9030}
DATABASE=${DATABASE:-"tpcds_sf${SCALE_FACTOR}"}

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
TPC-DS Data Generation Script

Usage: $0 [OPTIONS]

Options:
    -s, --scale FACTOR       Scale factor (default: 1)
                             SF1=~3GB, SF10=~30GB, SF100=~300GB
    -d, --dsdgen-dir DIR     Directory for TPC-DS dsdgen tool
    -o, --output-dir DIR     Output directory for generated data
    -h, --host HOST          FE host (default: 127.0.0.1)
    -p, --port PORT          FE MySQL port (default: 9030)
    --help                   Show this help

Examples:
    # Generate SF1 (~3GB) data
    $0 -s 1

    # Generate SF10 data to custom directory
    $0 -s 10 -o /data/tpcds-sf10

    # Generate and load into Rust FE
    $0 -s 1 -p 9031

Environment Variables:
    SCALE_FACTOR    Scale factor (same as -s)
    DSDGEN_DIR      dsdgen directory
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
        -d|--dsdgen-dir)
            DSDGEN_DIR="$2"
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

DATABASE="tpcds_sf${SCALE_FACTOR}"

echo "======================================"
echo "TPC-DS Data Generation"
echo "======================================"
echo "Scale Factor:    SF${SCALE_FACTOR}"
echo "Database:        ${DATABASE}"
echo "Data Directory:  ${DATA_DIR}"
echo "FE Endpoint:     ${FE_HOST}:${FE_PORT}"
echo "======================================"
echo ""

# Step 1: Install/compile dsdgen
step "Setting up TPC-DS dsdgen..."

if [ ! -d "$DSDGEN_DIR" ]; then
    mkdir -p "$DSDGEN_DIR"
    step "Cloning TPC-DS kit repository..."

    if ! command -v git &> /dev/null; then
        error "git is not installed. Please install git first."
        exit 1
    fi

    git clone https://github.com/gregrahn/tpcds-kit.git "$DSDGEN_DIR"
    cd "$DSDGEN_DIR/tools"

    step "Compiling dsdgen..."
    if ! command -v gcc &> /dev/null; then
        error "gcc is not installed. Please install gcc first."
        exit 1
    fi

    make -f Makefile.suite
    success "dsdgen compiled successfully"
else
    success "dsdgen already exists at ${DSDGEN_DIR}"
    cd "$DSDGEN_DIR/tools"
fi

# Step 2: Generate data
step "Generating TPC-DS data (SF${SCALE_FACTOR})..."

mkdir -p "$DATA_DIR"

if [ -f "${DATA_DIR}/store_sales.dat" ] && [ -s "${DATA_DIR}/store_sales.dat" ]; then
    warn "Data files already exist in ${DATA_DIR}"
    read -p "Regenerate data? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        success "Using existing data files"
    else
        rm -f "${DATA_DIR}"/*.dat
        ./dsdgen -SCALE "$SCALE_FACTOR" -DIR "$DATA_DIR" -FORCE Y
        success "Data generated in ${DATA_DIR}"
    fi
else
    ./dsdgen -SCALE "$SCALE_FACTOR" -DIR "$DATA_DIR" -FORCE Y
    success "Data generated in ${DATA_DIR}"
fi

# Show data file sizes
echo ""
step "Generated data files:"
du -sh "${DATA_DIR}"/*.dat 2>/dev/null | head -10 || echo "  (files generated)"
echo "  ... (24 tables total)"
echo ""

# Step 3: Create database and tables
step "Creating database and tables..."

mysql -h "$FE_HOST" -P "$FE_PORT" -u root -e "DROP DATABASE IF EXISTS ${DATABASE};" 2>/dev/null || true
mysql -h "$FE_HOST" -P "$FE_PORT" -u root -e "CREATE DATABASE ${DATABASE};"

# Create TPC-DS tables
mysql -h "$FE_HOST" -P "$FE_PORT" -u root "$DATABASE" <<'EOF'
-- Call center
CREATE TABLE call_center (
    cc_call_center_sk BIGINT NOT NULL,
    cc_call_center_id CHAR(16) NOT NULL,
    cc_rec_start_date DATE,
    cc_rec_end_date DATE,
    cc_closed_date_sk INT,
    cc_open_date_sk INT,
    cc_name VARCHAR(50),
    cc_class VARCHAR(50),
    cc_employees INT,
    cc_sq_ft INT,
    cc_hours CHAR(20),
    cc_manager VARCHAR(40),
    cc_mkt_id INT,
    cc_mkt_class CHAR(50),
    cc_mkt_desc VARCHAR(100),
    cc_market_manager VARCHAR(40),
    cc_division INT,
    cc_division_name VARCHAR(50),
    cc_company INT,
    cc_company_name CHAR(50),
    cc_street_number CHAR(10),
    cc_street_name VARCHAR(60),
    cc_street_type CHAR(15),
    cc_suite_number CHAR(10),
    cc_city VARCHAR(60),
    cc_county VARCHAR(30),
    cc_state CHAR(2),
    cc_zip CHAR(10),
    cc_country VARCHAR(20),
    cc_gmt_offset DECIMAL(5,2),
    cc_tax_percentage DECIMAL(5,2)
)
DUPLICATE KEY(cc_call_center_sk)
DISTRIBUTED BY HASH(cc_call_center_sk) BUCKETS 1
PROPERTIES ("replication_num" = "1");

-- Store sales (largest table)
CREATE TABLE store_sales (
    ss_sold_date_sk INT,
    ss_sold_time_sk INT,
    ss_item_sk INT NOT NULL,
    ss_customer_sk INT,
    ss_cdemo_sk INT,
    ss_hdemo_sk INT,
    ss_addr_sk INT,
    ss_store_sk INT,
    ss_promo_sk INT,
    ss_ticket_number BIGINT NOT NULL,
    ss_quantity INT,
    ss_wholesale_cost DECIMAL(7,2),
    ss_list_price DECIMAL(7,2),
    ss_sales_price DECIMAL(7,2),
    ss_ext_discount_amt DECIMAL(7,2),
    ss_ext_sales_price DECIMAL(7,2),
    ss_ext_wholesale_cost DECIMAL(7,2),
    ss_ext_list_price DECIMAL(7,2),
    ss_ext_tax DECIMAL(7,2),
    ss_coupon_amt DECIMAL(7,2),
    ss_net_paid DECIMAL(7,2),
    ss_net_paid_inc_tax DECIMAL(7,2),
    ss_net_profit DECIMAL(7,2)
)
DUPLICATE KEY(ss_sold_date_sk, ss_sold_time_sk, ss_item_sk)
PARTITION BY RANGE(ss_sold_date_sk) (
    PARTITION p1 VALUES LESS THAN ("2451180"),
    PARTITION p2 VALUES LESS THAN ("2451545"),
    PARTITION p3 VALUES LESS THAN ("2451910"),
    PARTITION p4 VALUES LESS THAN ("2452275"),
    PARTITION p5 VALUES LESS THAN ("2452640")
)
DISTRIBUTED BY HASH(ss_item_sk) BUCKETS 32
PROPERTIES ("replication_num" = "1");

-- Date dimension
CREATE TABLE date_dim (
    d_date_sk INT NOT NULL,
    d_date_id CHAR(16) NOT NULL,
    d_date DATE,
    d_month_seq INT,
    d_week_seq INT,
    d_quarter_seq INT,
    d_year INT,
    d_dow INT,
    d_moy INT,
    d_dom INT,
    d_qoy INT,
    d_fy_year INT,
    d_fy_quarter_seq INT,
    d_fy_week_seq INT,
    d_day_name CHAR(9),
    d_quarter_name CHAR(6),
    d_holiday CHAR(1),
    d_weekend CHAR(1),
    d_following_holiday CHAR(1),
    d_first_dom INT,
    d_last_dom INT,
    d_same_day_ly INT,
    d_same_day_lq INT,
    d_current_day CHAR(1),
    d_current_week CHAR(1),
    d_current_month CHAR(1),
    d_current_quarter CHAR(1),
    d_current_year CHAR(1)
)
DUPLICATE KEY(d_date_sk)
DISTRIBUTED BY HASH(d_date_sk) BUCKETS 1
PROPERTIES ("replication_num" = "1");

-- Item
CREATE TABLE item (
    i_item_sk INT NOT NULL,
    i_item_id CHAR(16) NOT NULL,
    i_rec_start_date DATE,
    i_rec_end_date DATE,
    i_item_desc VARCHAR(200),
    i_current_price DECIMAL(7,2),
    i_wholesale_cost DECIMAL(7,2),
    i_brand_id INT,
    i_brand CHAR(50),
    i_class_id INT,
    i_class CHAR(50),
    i_category_id INT,
    i_category CHAR(50),
    i_manufact_id INT,
    i_manufact CHAR(50),
    i_size CHAR(20),
    i_formulation CHAR(20),
    i_color CHAR(20),
    i_units CHAR(10),
    i_container CHAR(10),
    i_manager_id INT,
    i_product_name CHAR(50)
)
DUPLICATE KEY(i_item_sk)
DISTRIBUTED BY HASH(i_item_sk) BUCKETS 12
PROPERTIES ("replication_num" = "1");

-- Store
CREATE TABLE store (
    s_store_sk INT NOT NULL,
    s_store_id CHAR(16) NOT NULL,
    s_rec_start_date DATE,
    s_rec_end_date DATE,
    s_closed_date_sk INT,
    s_store_name VARCHAR(50),
    s_number_employees INT,
    s_floor_space INT,
    s_hours CHAR(20),
    s_manager VARCHAR(40),
    s_market_id INT,
    s_geography_class VARCHAR(100),
    s_market_desc VARCHAR(100),
    s_market_manager VARCHAR(40),
    s_division_id INT,
    s_division_name VARCHAR(50),
    s_company_id INT,
    s_company_name VARCHAR(50),
    s_street_number VARCHAR(10),
    s_street_name VARCHAR(60),
    s_street_type CHAR(15),
    s_suite_number CHAR(10),
    s_city VARCHAR(60),
    s_county VARCHAR(30),
    s_state CHAR(2),
    s_zip CHAR(10),
    s_country VARCHAR(20),
    s_gmt_offset DECIMAL(5,2),
    s_tax_precentage DECIMAL(5,2)
)
DUPLICATE KEY(s_store_sk)
DISTRIBUTED BY HASH(s_store_sk) BUCKETS 1
PROPERTIES ("replication_num" = "1");

-- Customer
CREATE TABLE customer (
    c_customer_sk INT NOT NULL,
    c_customer_id CHAR(16) NOT NULL,
    c_current_cdemo_sk INT,
    c_current_hdemo_sk INT,
    c_current_addr_sk INT,
    c_first_shipto_date_sk INT,
    c_first_sales_date_sk INT,
    c_salutation CHAR(10),
    c_first_name CHAR(20),
    c_last_name CHAR(30),
    c_preferred_cust_flag CHAR(1),
    c_birth_day INT,
    c_birth_month INT,
    c_birth_year INT,
    c_birth_country VARCHAR(20),
    c_login CHAR(13),
    c_email_address CHAR(50),
    c_last_review_date_sk INT
)
DUPLICATE KEY(c_customer_sk)
DISTRIBUTED BY HASH(c_customer_sk) BUCKETS 12
PROPERTIES ("replication_num" = "1");

-- Customer demographics
CREATE TABLE customer_demographics (
    cd_demo_sk INT NOT NULL,
    cd_gender CHAR(1),
    cd_marital_status CHAR(1),
    cd_education_status CHAR(20),
    cd_purchase_estimate INT,
    cd_credit_rating CHAR(10),
    cd_dep_count INT,
    cd_dep_employed_count INT,
    cd_dep_college_count INT
)
DUPLICATE KEY(cd_demo_sk)
DISTRIBUTED BY HASH(cd_demo_sk) BUCKETS 1
PROPERTIES ("replication_num" = "1");

-- Customer address
CREATE TABLE customer_address (
    ca_address_sk INT NOT NULL,
    ca_address_id CHAR(16) NOT NULL,
    ca_street_number CHAR(10),
    ca_street_name VARCHAR(60),
    ca_street_type CHAR(15),
    ca_suite_number CHAR(10),
    ca_city VARCHAR(60),
    ca_county VARCHAR(30),
    ca_state CHAR(2),
    ca_zip CHAR(10),
    ca_country VARCHAR(20),
    ca_gmt_offset DECIMAL(5,2),
    ca_location_type CHAR(20)
)
DUPLICATE KEY(ca_address_sk)
DISTRIBUTED BY HASH(ca_address_sk) BUCKETS 1
PROPERTIES ("replication_num" = "1");

-- Promotion
CREATE TABLE promotion (
    p_promo_sk INT NOT NULL,
    p_promo_id CHAR(16) NOT NULL,
    p_start_date_sk INT,
    p_end_date_sk INT,
    p_item_sk INT,
    p_cost DECIMAL(15,2),
    p_response_target INT,
    p_promo_name CHAR(50),
    p_channel_dmail CHAR(1),
    p_channel_email CHAR(1),
    p_channel_catalog CHAR(1),
    p_channel_tv CHAR(1),
    p_channel_radio CHAR(1),
    p_channel_press CHAR(1),
    p_channel_event CHAR(1),
    p_channel_demo CHAR(1),
    p_channel_details VARCHAR(100),
    p_purpose CHAR(15),
    p_discount_active CHAR(1)
)
DUPLICATE KEY(p_promo_sk)
DISTRIBUTED BY HASH(p_promo_sk) BUCKETS 1
PROPERTIES ("replication_num" = "1");

-- Store returns
CREATE TABLE store_returns (
    sr_returned_date_sk INT,
    sr_return_time_sk INT,
    sr_item_sk INT NOT NULL,
    sr_customer_sk INT,
    sr_cdemo_sk INT,
    sr_hdemo_sk INT,
    sr_addr_sk INT,
    sr_store_sk INT,
    sr_reason_sk INT,
    sr_ticket_number BIGINT NOT NULL,
    sr_return_quantity INT,
    sr_return_amt DECIMAL(7,2),
    sr_return_tax DECIMAL(7,2),
    sr_return_amt_inc_tax DECIMAL(7,2),
    sr_fee DECIMAL(7,2),
    sr_return_ship_cost DECIMAL(7,2),
    sr_refunded_cash DECIMAL(7,2),
    sr_reversed_charge DECIMAL(7,2),
    sr_store_credit DECIMAL(7,2),
    sr_net_loss DECIMAL(7,2)
)
DUPLICATE KEY(sr_returned_date_sk, sr_return_time_sk, sr_item_sk)
DISTRIBUTED BY HASH(sr_item_sk) BUCKETS 12
PROPERTIES ("replication_num" = "1");

-- Web sales
CREATE TABLE web_sales (
    ws_sold_date_sk INT,
    ws_sold_time_sk INT,
    ws_ship_date_sk INT,
    ws_item_sk INT NOT NULL,
    ws_bill_customer_sk INT,
    ws_bill_cdemo_sk INT,
    ws_bill_hdemo_sk INT,
    ws_bill_addr_sk INT,
    ws_ship_customer_sk INT,
    ws_ship_cdemo_sk INT,
    ws_ship_hdemo_sk INT,
    ws_ship_addr_sk INT,
    ws_web_page_sk INT,
    ws_web_site_sk INT,
    ws_ship_mode_sk INT,
    ws_warehouse_sk INT,
    ws_promo_sk INT,
    ws_order_number BIGINT NOT NULL,
    ws_quantity INT,
    ws_wholesale_cost DECIMAL(7,2),
    ws_list_price DECIMAL(7,2),
    ws_sales_price DECIMAL(7,2),
    ws_ext_discount_amt DECIMAL(7,2),
    ws_ext_sales_price DECIMAL(7,2),
    ws_ext_wholesale_cost DECIMAL(7,2),
    ws_ext_list_price DECIMAL(7,2),
    ws_ext_tax DECIMAL(7,2),
    ws_coupon_amt DECIMAL(7,2),
    ws_ext_ship_cost DECIMAL(7,2),
    ws_net_paid DECIMAL(7,2),
    ws_net_paid_inc_tax DECIMAL(7,2),
    ws_net_paid_inc_ship DECIMAL(7,2),
    ws_net_paid_inc_ship_tax DECIMAL(7,2),
    ws_net_profit DECIMAL(7,2)
)
DUPLICATE KEY(ws_sold_date_sk, ws_sold_time_sk, ws_item_sk)
DISTRIBUTED BY HASH(ws_item_sk) BUCKETS 32
PROPERTIES ("replication_num" = "1");

-- Catalog sales
CREATE TABLE catalog_sales (
    cs_sold_date_sk INT,
    cs_sold_time_sk INT,
    cs_ship_date_sk INT,
    cs_bill_customer_sk INT,
    cs_bill_cdemo_sk INT,
    cs_bill_hdemo_sk INT,
    cs_bill_addr_sk INT,
    cs_ship_customer_sk INT,
    cs_ship_cdemo_sk INT,
    cs_ship_hdemo_sk INT,
    cs_ship_addr_sk INT,
    cs_call_center_sk INT,
    cs_catalog_page_sk INT,
    cs_ship_mode_sk INT,
    cs_warehouse_sk INT,
    cs_item_sk INT NOT NULL,
    cs_promo_sk INT,
    cs_order_number BIGINT NOT NULL,
    cs_quantity INT,
    cs_wholesale_cost DECIMAL(7,2),
    cs_list_price DECIMAL(7,2),
    cs_sales_price DECIMAL(7,2),
    cs_ext_discount_amt DECIMAL(7,2),
    cs_ext_sales_price DECIMAL(7,2),
    cs_ext_wholesale_cost DECIMAL(7,2),
    cs_ext_list_price DECIMAL(7,2),
    cs_ext_tax DECIMAL(7,2),
    cs_coupon_amt DECIMAL(7,2),
    cs_ext_ship_cost DECIMAL(7,2),
    cs_net_paid DECIMAL(7,2),
    cs_net_paid_inc_tax DECIMAL(7,2),
    cs_net_paid_inc_ship DECIMAL(7,2),
    cs_net_paid_inc_ship_tax DECIMAL(7,2),
    cs_net_profit DECIMAL(7,2)
)
DUPLICATE KEY(cs_sold_date_sk, cs_sold_time_sk, cs_item_sk)
DISTRIBUTED BY HASH(cs_item_sk) BUCKETS 32
PROPERTIES ("replication_num" = "1");
EOF

success "Tables created in database ${DATABASE}"

echo ""
warn "Data loading for TPC-DS is complex and requires custom scripts."
warn "Please refer to TPC-DS specification for data loading guidelines."
warn "Alternatively, use Doris Stream Load API with appropriate column mappings."
echo ""
echo "Data files are ready in: ${DATA_DIR}"
echo "To load data, use Stream Load API similar to TPC-H."
echo ""

success "TPC-DS data generation complete!"

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
echo "  1. Load data using Stream Load API"
echo "  2. Run benchmarks:"
echo "     python3 scripts/benchmark_tpcds.py --scale ${SCALE_FACTOR}"
echo ""
echo "  3. Query the data:"
echo "     mysql -h ${FE_HOST} -P ${FE_PORT} -u root ${DATABASE}"
echo "======================================"
