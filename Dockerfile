# Xavier2 - Production Docker Build
# Optimized multi-stage build for minimal image size (< 100MB)

# Stage 1: Recipe (optional, but good for caching)
FROM rust:1.89-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler libssl-dev pkg-config clang build-essential curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy source
COPY Cargo.toml Cargo.lock ./
COPY benches/ benches/
COPY src/ src/
COPY code-graph/ code-graph/

# Build xavier2 binary (release mode)
# Using RUSTFLAGS to optimize for size if needed, but standard release is usually fine
RUN cargo build --release --bin xavier2

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl sqlite3 && \
    rm -rf /var/lib/apt/lists/*

# Create data and log directories
RUN mkdir -p /data /logs

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/xavier2 /usr/local/bin/

# Environment defaults
ENV XAVIER2_PORT=8003 \
    XAVIER2_HOST=0.0.0.0 \
    XAVIER2_DATA_DIR=/data \
    RUST_LOG=info

EXPOSE 8003

# Healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8003/health || exit 1

CMD ["xavier2"]
