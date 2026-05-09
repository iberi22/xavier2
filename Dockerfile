# syntax=docker/dockerfile:1
# Xavier - Optimized Multi-Stage Docker Build
# Target: < 100MB production image
#
# Usage: docker build -t xavier . && docker run -p 8006:8006 xavier

# Stage 1: Builder
# Using slim variant to keep final image small (~500MB vs ~800MB for full)
FROM rust:1.90-bookworm AS builder

WORKDIR /app

# Install ONLY what cargo/rustc need at build time:
# - protobuf-compiler: for tonic (gRPC/prost) used by surrealdb
# - libssl-dev: for OpenSSL linkage during build
# - pkg-config: for finding libraries
# - curl: for healthcheck in final image
RUN apt-get update && apt-get install -y --no-install-recommends \
        protobuf-compiler \
        libssl-dev \
        pkg-config \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Copy source (minimal build context)
COPY Cargo.toml Cargo.lock ./
COPY benches/ benches/
COPY src/ src/
COPY code-graph/ code-graph/

# Build only xavier binary (skip bench, gui, tui for smaller image)
# Using -j 1 to avoid OOM on memory-constrained systems (Windows Docker Desktop)
RUN cargo build --release --bin xavier -j 1

# Strip debug symbols to reduce binary size (~15-20MB savings)
RUN strip -s /app/target/release/xavier

# Stage 2: Runtime
# Minimal Debian-based runtime with only essential libs
FROM debian:bookworm-slim

ARG XAVIER_VERSION=0.4.1

# Runtime dependencies:
# - ca-certificates: for HTTPS/TLS certificate validation
# - libssl3: required by rusqlite bundled SQLite and any OpenSSL-using deps
# - curl: for healthcheck
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl \
    && rm -rf /var/lib/apt/lists/*

# Create data directory
RUN mkdir -p /data

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/xavier /usr/local/bin/xavier

EXPOSE 8006

# Healthcheck: verify the server is responding
HEALTHCHECK --interval=30s --timeout=10s --start-period=15s --retries=3 \
    CMD curl -fsS http://localhost:8006/health || exit 1

ENV XAVIER_PORT=8006 \
    XAVIER_HOST=0.0.0.0 \
    XAVIER_URL=http://localhost:8006 \
    RUST_LOG=info \
    XAVIER_VERSION=${XAVIER_VERSION} \
    XAVIER_WORKSPACE_ID=default

# Required: set XAVIER_TOKEN to a secure random value
CMD ["/usr/local/bin/xavier", "http"]
