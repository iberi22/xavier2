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

### Runtime configuration

Xavier2 now treats `config/xavier2.config.json` as the canonical location for non-secret runtime settings. Secrets and credentials belong in `.env`.

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `XAVIER2_TOKEN` | no safe default | API authentication token |
| `XAVIER2_CONFIG_PATH` | `config/xavier2.config.json` | Optional override for the canonical runtime config file |
| provider API keys | unset | External API credentials |

The current server still contains environment-based code paths internally, but the repository standard is now:

- `config/xavier2.config.json` for non-secret operational settings
- `.env` for credentials and secrets

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
