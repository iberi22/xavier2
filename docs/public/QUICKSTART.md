# Xavier Quick Start

> Fast Vector Memory for AI Agents — get up and running in 5 minutes.

## Prerequisites

- **Rust** 1.80+ (`rustup install stable`)
- Or **Docker** (if using the container image)

## Installation

### From source (recommended)

```bash
cargo install xavier
```

### From Docker

```bash
docker run -p 8006:8006 -e XAVIER_TOKEN=my-secret-token xavier/xavier
```

### From GitHub

```bash
git clone https://github.com/iberi22/xavier
cd xavier
cargo build --release
./target/release/xavier http
```

## Configuration

Set the following environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `XAVIER_TOKEN` | **Yes** | — | Authentication token for API access |
| `XAVIER_HOST` | No | `127.0.0.1` | HTTP bind address |
| `XAVIER_PORT` | No | `8006` | HTTP server port |
| `XAVIER_URL` | No | `http://127.0.0.1:8006` | Client-facing URL |
| `XAVIER_EMBEDDING_PROVIDER_MODE` | No | `auto` | Embedding mode: `local`, `cloud`, `disabled` |

## Quick Start (3 commands)

### 1. Start the server

```bash
export XAVIER_TOKEN=my-secret-token
xavier http
```

### 2. Add a memory

```bash
xavier add "Rust is a systems programming language focused on safety and performance." --kind semantic
```

### 3. Search your memories

```bash
xavier search "Rust programming"
```

## Next Steps

- [Architecture Overview](ARCHITECTURE.md) — Understand how Xavier works
- [API Reference](API.md) — Full HTTP API documentation
- [Examples](../examples/) — Working CLI, HTTP, and MCP examples

## Example: Using the HTTP API

```bash
# Health check
curl -s http://localhost:8006/health \
  -H "X-Xavier-Token: $XAVIER_TOKEN"

# Store a memory
curl -s -X POST http://localhost:8006/memory/add \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content": "Hello from Xavier!", "metadata": {"kind": "episodic"}}'

# Search
curl -s -X POST http://localhost:8006/memory/search \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "hello", "limit": 5}'
```

## Example: Using the CLI

```bash
# Add a typed memory
xavier add "The system uses hexagonal architecture with ports and adapters." --kind semantic

# Add with title
xavier add "Meeting about Q3 planning." --title "Q3 Planning Sync" --kind episodic

# Search
xavier search "architecture"

# Recall (scored results)
xavier recall "planning"

# Statistics
xavier stats
```

## Key Features

- **Vector Memory** — Semantic search with embedding support (OpenAI, Ollama, MiniMax)
- **Hybrid Search** — Combines keyword (BM25/FTS5), vector, and knowledge graph signals via RRF
- **Memory Tiers** — Automatic consolidation from working to archival storage
- **Memory Graph** — Entity-relationship tracking with BFS traversal
- **Reflection** — Pattern detection and insight generation across memories
- **MCP Support** — Model Context Protocol stdio server for AI agent integration
- **Multi-Provider** — Pluggable embedding backends with automatic fallback
