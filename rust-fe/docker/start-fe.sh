#!/bin/bash
set -e

echo "Starting Doris Rust FE..."
echo "================================"

# Process configuration template
envsubst < /opt/doris-rust-fe/conf/fe.conf.template > /opt/doris-rust-fe/conf/fe.conf

echo "Configuration:"
echo "  HTTP Port:     ${FE_HTTP_PORT:-8031}"
echo "  RPC Port:      ${FE_RPC_PORT:-9021}"
echo "  Query Port:    ${FE_QUERY_PORT:-9031}"
echo "  Edit Log Port: ${FE_EDIT_LOG_PORT:-9011}"
echo ""
echo "Optimizations:"
echo "  Cost-based joins:   ${ENABLE_COST_BASED_JOIN:-true}"
echo "  Partition pruning:  ${ENABLE_PARTITION_PRUNING:-true}"
echo "  Runtime filters:    ${ENABLE_RUNTIME_FILTERS:-true}"
echo "  Bucket shuffle:     ${ENABLE_BUCKET_SHUFFLE:-true}"
echo "================================"
echo ""

# Wait for BE if specified
if [ -n "$WAIT_FOR_BE" ]; then
    echo "Waiting for BE at ${WAIT_FOR_BE}..."
    timeout=60
    while [ $timeout -gt 0 ]; do
        if nc -z $(echo $WAIT_FOR_BE | cut -d: -f1) $(echo $WAIT_FOR_BE | cut -d: -f2) 2>/dev/null; then
            echo "BE is ready!"
            break
        fi
        sleep 1
        timeout=$((timeout-1))
    done
    if [ $timeout -eq 0 ]; then
        echo "WARNING: BE not available after 60 seconds, continuing anyway..."
    fi
fi

# Set environment variables for optimizations
export DORIS_COST_BASED_JOIN=${ENABLE_COST_BASED_JOIN:-true}
export DORIS_PARTITION_PRUNING=${ENABLE_PARTITION_PRUNING:-true}
export DORIS_RUNTIME_FILTERS=${ENABLE_RUNTIME_FILTERS:-true}
export DORIS_BUCKET_SHUFFLE=${ENABLE_BUCKET_SHUFFLE:-true}

# Start the FE
exec /opt/doris-rust-fe/bin/doris-rust-fe \
    --config /opt/doris-rust-fe/conf/fe.conf \
    2>&1 | tee /opt/doris-rust-fe/log/fe.log
