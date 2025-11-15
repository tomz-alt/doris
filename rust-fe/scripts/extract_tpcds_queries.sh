#!/bin/bash
# Extract TPC-DS queries from tpcds-kit templates
# Generates Doris-compatible SQL from TPC-DS query templates

set -e

TPCDS_KIT="/tmp/tpcds-kit-temp"
OUTPUT_DIR="$(dirname "$0")/tpcds/queries"

echo "======================================"
echo "Extracting TPC-DS Queries"
echo "======================================"

# Clone tpcds-kit if not exists
if [ ! -d "$TPCDS_KIT" ]; then
    echo "==> Cloning TPC-DS kit..."
    git clone --depth 1 https://github.com/gregrahn/tpcds-kit.git "$TPCDS_KIT"
fi

# Build dsqgen tool
if [ ! -f "$TPCDS_KIT/tools/dsqgen" ]; then
    echo "==> Building dsqgen tool..."
    cd "$TPCDS_KIT/tools"
    make -f Makefile.suite
fi

# Generate queries
echo "==> Generating TPC-DS queries..."
cd "$TPCDS_KIT/tools"

mkdir -p "$OUTPUT_DIR"

# Generate all 99 queries using dsqgen
for i in $(seq 1 99); do
    echo "  Generating query $i..."

    # Generate query with ansi dialect (most compatible)
    ./dsqgen \
        -DIRECTORY ../query_templates \
        -INPUT ../query_templates/templates.lst \
        -VERBOSE N \
        -QUALIFY N \
        -SCALE 1 \
        -DIALECT ansi \
        -TEMPLATE query$i.tpl \
        -OUTPUT_DIR "$OUTPUT_DIR" \
        > /dev/null 2>&1 || true

    # The generated file will be named query_0.sql (dsqgen always uses query_0)
    if [ -f "$OUTPUT_DIR/query_0.sql" ]; then
        # Add header comment and move to proper name
        {
            echo "-- TPC-DS Query $i"
            echo "-- Generated from TPC-DS toolkit query templates"
            echo "-- Dialect: ANSI SQL"
            echo ""
            cat "$OUTPUT_DIR/query_0.sql"
        } > "$OUTPUT_DIR/q$i.sql"

        rm "$OUTPUT_DIR/query_0.sql"
        echo "  ✓ Created q$i.sql"
    else
        # If dsqgen failed, keep template placeholder
        if [ ! -f "$OUTPUT_DIR/q$i.sql" ] || grep -q "placeholder" "$OUTPUT_DIR/q$i.sql"; then
            echo "  ! Query $i: using template (dsqgen failed)"
        fi
    fi
done

echo ""
echo "✓ TPC-DS query extraction complete!"
echo ""
echo "Generated queries in: $OUTPUT_DIR"
echo "Total: $(ls -1 "$OUTPUT_DIR"/q*.sql | wc -l) queries"
echo ""
echo "Note: Queries are generated from official TPC-DS v3.2.0 templates"
echo "      Some queries may need minor syntax adjustments for Doris"
