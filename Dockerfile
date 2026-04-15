# Xavier2 - Simple Docker Build
# Usage: docker build -t xavier2 . && docker run -p 8003:8003 xavier2

FROM rust:1.89-bookworm

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler libssl-dev pkg-config && \
    rm -rf /var/lib/apt/lists/*

# Copy source (minimal - just what's needed to build)
COPY Cargo.toml Cargo.lock ./
COPY benches/ benches/
COPY src/ src/
COPY code-graph/ code-graph/

# Build xavier2 binary (release mode)
RUN cargo build --release --bin xavier2

# Runtime image
FROM debian:bookworm-slim

# Install runtime deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

# Create data directory
RUN mkdir -p /data

WORKDIR /app

# Copy binary from builder
COPY --from=0 /app/target/release/xavier2 /usr/local/bin/

EXPOSE 8003

CMD ["xavier2"]
