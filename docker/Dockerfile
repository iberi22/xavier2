# Xavier2 - Multi-stage Docker Build
# Stage 1: Build dependencies and binary
FROM rust:1.77-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock* ./
COPY code-graph/ code-graph/

# Create dummy source for dependency caching
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies (cached layer)
RUN cargo build --release 2>&1 | tail -5
RUN rm -rf src

# Copy actual source
COPY src ./src
COPY bin ./bin

# Build the binary
RUN cargo build --release 2>&1 | tail -3

# Stage 2: Runtime (minimal image)
FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN useradd -m -u 1000 xavier2

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/xavier2 /usr/local/bin/

# Set environment
ENV XAVIER2_PORT=8006
ENV RUST_LOG=info
ENV XAVIER2_STORAGE=/data

# Create data directory
RUN mkdir /data && chown xavier2:xavier2 /data

USER xavier2

# Expose port
EXPOSE 8006

# Healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8006/health || exit 1

# Run
ENTRYPOINT ["xavier2", "--port", "8006"]