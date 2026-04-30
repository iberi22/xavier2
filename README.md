# Xavier2 - Fast Vector Memory

7ms average vector search for AI agents.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.4.1-green.svg)](https://github.com/iberi22/xavier2-1)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

---

## Quick Start

```bash
# Install from source
cargo install xavier2

# Or build from source
git clone https://github.com/iberi22/xavier2-1.git
cd xavier2-1
cargo build --release

# Start HTTP server (default port 8006)
xavier2 http

# Search via CLI
xavier2 search "your query"

# Add a memory
xavier2 add "Remember to review PRs on Fridays" --title "PR reminder"

# Check stats
xavier2 stats
```

## Features

- **7ms average vector search** — SQLite-vec powered, no external services
- **MCP-stdio interface** — Connect to Claude Desktop, Cursor, Windsurf, and other MCP clients
- **CLI tool** — Human-friendly commands for search, add, and stats
- **RRF fusion** — Reciprocal Rank Fusion combines vector + keyword + graph signals
- **SQLite-vec storage** — Embedded, portable, no server needed

## API

### Health & Stats

```bash
# Health check
curl http://localhost:8006/health

# Get memory stats
curl http://localhost:8006/memory/stats \
  -H "X-Xavier2-Token: $TOKEN"
```

### Memory Operations

```bash
# Add memory
curl -X POST http://localhost:8006/memory/add \
  -H "X-Xavier2-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content":"Design decision: use RRF k=60","path":"decisions/001"}'

# Vector search
curl -X POST http://localhost:8006/memory/search \
  -H "X-Xavier2-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"design decisions","limit":5}'

# Hybrid search (vector + FTS5)
curl -X POST http://localhost:8006/memory/hybrid \
  -H "X-Xavier2-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"architecture decisions","limit":10}'

# Delete memory
curl -X DELETE http://localhost:8006/memory/evict \
  -H "X-Xavier2-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"id":"01ARZ3NDEKTSV4RRFFQ69G5FAV"}'
```

## MCP Integration

Connect Xavier2 to any MCP-compatible AI client:

```bash
# Start MCP server (stdio mode)
xavier2 mcp
```

Configure your MCP client to use stdio transport pointing to `xavier2 mcp`.

## Docker

```bash
# Run with Docker
docker run -p 8006:8006 \
  -e XAVIER2_TOKEN=your-secret-token \
  ghcr.io/iberi22/xavier2:latest

# Or use docker-compose
docker compose up -d
```

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `XAVIER2_PORT` | `8006` | HTTP server port |
| `XAVIER2_HOST` | `0.0.0.0` | Bind address |
| `XAVIER2_TOKEN` | `dev-token` | Authentication token |
| `XAVIER2_DEV_MODE` | `false` | Skip auth (dev only) |
| `XAVIER2_LOG_LEVEL` | `info` | Log level |

## Architecture

Xavier2 is moving towards a multi-crate workspace for better reusability and faster builds. See our [Workspace Evolution Strategy](docs/ARCHITECTURE/ARCHITECTURE.md#workspace-evolution).

```
┌─────────────────────────────────────────────────────┐
│                   Xavier2                            │
├─────────────────────────────────────────────────────┤
│  CLI / HTTP / MCP-stdio                             │
├─────────────────────────────────────────────────────┤
│  Hybrid Search (RRF Fusion)                         │
│  ┌──────────┬──────────┬──────────┐                 │
│  │  Vector  │   FTS5   │  Graph   │                 │
│  │ (sqlite- │ (BM25)   │ (Entity  │                 │
│  │   vec)   │          │ Relations)│                │
│  └──────────┴──────────┴──────────┘                 │
├─────────────────────────────────────────────────────┤
│  SQLite-vec Storage                                 │
└─────────────────────────────────────────────────────┘
```

## License

MIT — free for everyone, forever.

**Commercial use?** Consider supporting the project. See [PRICING](docs/PRICING.md).

---

Built with ❤️ by [SouthWest AI Labs](https://southwest-ai-labs.com)
