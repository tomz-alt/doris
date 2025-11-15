# Rust FE Docker Image
FROM rust:1.84-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy source code
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
COPY tests ./tests
COPY build.rs ./
COPY proto ./proto

# Build in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    mysql-client \
    && rm -rf /var/lib/apt/lists/*

# Create doris user
RUN useradd -m -u 1000 -s /bin/bash doris

# Create directories
RUN mkdir -p /opt/doris-rust-fe/bin \
    /opt/doris-rust-fe/conf \
    /opt/doris-rust-fe/log \
    /opt/doris-rust-fe/meta \
    && chown -R doris:doris /opt/doris-rust-fe

# Copy binary from builder
COPY --from=builder /app/target/release/doris-rust-fe /opt/doris-rust-fe/bin/

# Copy configuration template
COPY docker/fe.conf /opt/doris-rust-fe/conf/fe.conf.template

USER doris
WORKDIR /opt/doris-rust-fe

# Expose ports
# 8031: HTTP port (Rust FE)
# 9021: RPC port (Rust FE)
# 9031: MySQL port (Rust FE)
# 9011: Edit log port (Rust FE)
EXPOSE 8031 9021 9031 9011

# Environment variables
ENV FE_HTTP_PORT=8031 \
    FE_RPC_PORT=9021 \
    FE_QUERY_PORT=9031 \
    FE_EDIT_LOG_PORT=9011 \
    RUST_LOG=info

# Startup script
COPY docker/start-fe.sh /opt/doris-rust-fe/bin/
RUN chmod +x /opt/doris-rust-fe/bin/start-fe.sh

CMD ["/opt/doris-rust-fe/bin/start-fe.sh"]
