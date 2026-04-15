---
title: Installation
description: How to install and configure Xavier2
---

# Installation Guide

## Prerequisites

- **Rust** 1.70+ with cargo
- **Docker** for the local stack
- **OpenSSL** development libraries
- **Node.js** 22.12+ for the `docs/site` workspace

## Build from Source

### 1. Clone the repository

```bash
git clone https://github.com/southwest-ai-labs/xavier2.git
cd xavier2
```

### 2. Build the project

```bash
# Debug build
cargo build

# Main runtime binary
cargo build --bin xavier2
```

### 3. Run validation

```bash
cargo test --workspace --features ci-safe --exclude xavier2-web
npm run build --workspace panel-ui
npm run build --workspace docs/site
```

## Configuration

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `XAVIER2_PORT` | `8003` | HTTP server port |
| `XAVIER2_HOST` | `0.0.0.0` | Bind address |
| `XAVIER2_CODE_GRAPH_DB_PATH` | `/data/code_graph.db` | Code graph database path |
| `XAVIER2_TOKEN` | no safe default | API authentication token |
| `XAVIER2_LOG_LEVEL` | `info` | Logging verbosity |
| `XAVIER2_IMAGE_TAG` | `0.4.1` | Docker image tag for controlled upgrades |
| `XAVIER2_MEMORY_BACKEND` | `file` | Active runtime backend in the current deployment story |

The current server configuration is driven primarily through environment variables and Docker Compose. There is no repository-standard `config.yaml` flow documented as the primary runtime path.

## Running Xavier2

### Development mode

```bash
cargo run --bin xavier2
```

### Release binary

```bash
cargo build --release --bin xavier2
./target/release/xavier2
```

### Docker

```bash
cp .env.example .env
docker compose build xavier2
docker compose up -d
```

## Verify Installation

```bash
curl http://localhost:8003/health
curl http://localhost:8003/readiness
```

## Storage Reality

- The current validated deployment uses `FileMemoryStore`.
- SurrealDB is present in the repo and Docker setup, but it is not the default validated backend for current production-style documentation.
- If you need broader hosted persistence work, treat SurrealDB as an explicit follow-up track rather than an assumed default.
