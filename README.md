# Xavier2 — Fast Vector Memory for AI Agents

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.6.0--beta-green.svg)](https://github.com/iberi22/xavier2)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)
[![CI](https://img.shields.io/badge/CI-passing-brightgreen.svg)](https://github.com/iberi22/xavier2/actions)

Xavier2 is a **Rust-based memory runtime for AI agents** with HTTP, CLI, and MCP entry points. It stores, retrieves, and manages vector embeddings and structured memory over a SQLite-backed store, giving agents fast contextual recall without external dependencies.

## Quick Start

```bash
# Option 1: Install from source
cargo install xavier2

# Option 2: Run with Docker
docker run -p 8006:8006 -v xavier-data:/data ghcr.io/iberi22/xavier2:latest

# Start the server with a token
export XAVIER2_TOKEN=your-secret-token
xavier2 http

# Add and search memory
xavier2 add "AI agents should always verify their sources" "agent-guidelines"
xavier2 search "agent guidelines"
```

## Features

- **HTTP API** — JSON REST endpoints for memory CRUD with token-based auth
- **CLI client** — `add`, `search`, `stats` commands for quick interaction
- **MCP server** — stdio-based [Model Context Protocol](https://modelcontextprotocol.io) server exposing `search`, `add`, and `stats` tools
- **SQLite-backed** — Persistent, zero-infrastructure storage with vector search support
- **Public dataset export** — Generate read-optimized NDJSON datasets for agent bootstrap (see [Public Export](#public-dataset-export))
- **Hybrid retrieval** — Building blocks for combining keyword and semantic search
- **Agent runtime modules** — Ready-to-use runtime components for agent memory workflows

## Architecture Overview

```
┌─────────────┐  ┌──────────┐  ┌──────────┐
│   CLI       │  │  HTTP    │  │   MCP    │
│  (add/search)│  │  Server  │  │  (stdio) │
└──────┬──────┘  └────┬─────┘  └────┬─────┘
       │              │              │
       └──────────────┼──────────────┘
                      │
              ┌───────▼────────┐
              │  Core Engine   │
              │  (add, search, │
              │   stats,       │
              │   export)      │
              └───────┬────────┘
                      │
              ┌───────▼────────┐
              │  SQLite Store  │
              │  + Vector      │
              │  Embeddings    │
              └────────────────┘
```

The three entry points (CLI, HTTP, MCP) share the same core engine, which handles memory operations over a SQLite-backed store. Each entry point is independent — you can run the HTTP server, use the CLI against it, or connect the MCP server to any MCP-compatible host.

## Public Dataset Export

Generate a public, read-optimized dataset for agent context without cloning or rebuilding:

```bash
xavier2 export --public
```

Output lives in `xavier-dataset/` at the repository root with NDJSON files for memories, entities, timeline events, git commits, code symbols, and more.

Example agent bootstrap from GitHub raw:

```bash
BASE="https://raw.githubusercontent.com/iberi22/xavier2/main/xavier-dataset"

curl -fsSL "$BASE/dataset_manifest.json"
curl -fsSL "$BASE/memories.ndjson" | head -n 20
curl -fsSL "$BASE/code_symbols.ndjson" | jq -c 'select(.kind == "function")' | head
```

Full export schema is documented at [docs/FEATURE_STATUS.md](docs/FEATURE_STATUS.md).

## HTTP API

```bash
curl http://localhost:8006/health

curl -X POST http://localhost:8006/memory/add \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content":"Design decision: use RRF","path":"decisions/001"}'

curl -X POST http://localhost:8006/memory/search \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"design decision","limit":5}'
```

Full API reference: [docs/site/src/content/docs/reference/api.md](docs/site/src/content/docs/reference/api.md).

## MCP

Start the MCP stdio server:

```bash
xavier2 mcp
```

Current MCP tools: `search`, `add`, `stats`.

## Configuration

Runtime configuration lives in [config/xavier2.config.json](config/xavier2.config.json). Secrets go in `.env` (see [.env.example](.env.example)).

| Variable | Default | Description |
|---|---|---|
| `XAVIER2_TOKEN` | auto-generated | Authentication token for HTTP API |
| `XAVIER2_DEV_MODE` | `false` | Skip auth in development scenarios |
| `XAVIER2_CONFIG_PATH` | `config/xavier2.config.json` | Override runtime config file |
| Provider keys | unset | External API credentials (e.g. embedding providers) |

## Documentation

- [Feature Status](docs/FEATURE_STATUS.md) — Current verified surface and 1.0 gaps
- [CLI Reference](docs/guides/CLI_REFERENCE.md) — Full command documentation
- [API Reference](docs/site/src/content/docs/reference/api.md) — HTTP endpoint details
- [Public Release Roadmap](docs/PUBLIC_RELEASE_ROADMAP.md) — Upcoming milestones

## Status

Current release: **0.6 beta usable**. Not yet 1.0 — see [FEATURE_STATUS.md](docs/FEATURE_STATUS.md) for verified features and remaining gaps.

## License

MIT — see [LICENSE](LICENSE) for details.
