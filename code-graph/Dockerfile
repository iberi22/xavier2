# =============================================================================
# code-graph Docker Image
# =============================================================================
# Codebase Understanding without RAG - Tree-sitter + Agentic Search
#
# SECURITY FEATURES:
# - HTTP Token authentication required
# - Runs as non-root user
# - Read-only filesystem by default (mount code volume)
# - No default token (must be set via environment)
#
# =============================================================================

# Build stage
FROM rust:1.75-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy project
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build release binary
RUN cargo build --release --bin code-graph

# =============================================================================
# Runtime stage
# =============================================================================
FROM debian:bookworm-slim

# Security: install minimal dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/code-graph /usr/local/bin/

# Create data directory
RUN mkdir -p /data && chown -R appuser:appuser /app /data

# Switch to non-root user
USER appuser

# Environment variables (set at runtime)
# CODE_GRAPH_TOKEN - REQUIRED: Authentication token (min 16 chars)
# CODE_GRAPH_PORT  - Server port (default: 8080)
# CODE_GRAPH_HOST  - Server host (default: 0.0.0.0)

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Run server by default
ENTRYPOINT ["code-graph", "serve"]
CMD ["--token", "${CODE_GRAPH_TOKEN}"]
