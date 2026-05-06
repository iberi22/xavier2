# Xavier2 - Fast Vector Memory

Current release label: `0.6 beta usable`

Xavier2 is a Rust memory runtime for AI agents with HTTP, CLI, and MCP entry points over a SQLite-backed memory store.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.4.1-green.svg)](https://github.com/iberi22/xavier2)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

## Status

This repository is not positioned as `1.0` yet. The current verified feature surface and the gaps still blocking `1.0` are tracked in [docs/FEATURE_STATUS.md](docs/FEATURE_STATUS.md).

## Quick Start

```bash
# Install from source
cargo install xavier2

# Or build locally
git clone https://github.com/iberi22/xavier2.git
cd xavier2
cargo build --release

# Non-secret runtime configuration lives in config/xavier2.config.json
# Secrets live in .env

# Start the HTTP server
export XAVIER2_TOKEN=replace-with-a-real-token
xavier2 http

# CLI add/search/stats currently talk to the running HTTP server
xavier2 add "Remember to review PRs on Fridays" "PR reminder"
xavier2 search "review PRs"
xavier2 stats
```

## Verified Features

- HTTP server with auth-protected memory endpoints
- CLI client for `add`, `search`, and `stats`
- MCP stdio server with `search`, `add`, and `stats` tools
- SQLite-backed memory storage
- Hybrid retrieval building blocks and agent runtime modules

## HTTP API

Current default CLI server examples use port `8006`.

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

The richer route inventory, including caveats around current server surfaces, is documented in [docs/site/src/content/docs/reference/api.md](docs/site/src/content/docs/reference/api.md).

## MCP

Start the MCP stdio server with:

```bash
xavier2 mcp
```

Current verified MCP tools:

- `search`
- `add`
- `stats`

## Configuration

Operational runtime configuration lives in [config/xavier2.config.json](config/xavier2.config.json).
Credentials and secrets live in `.env`, based on [.env.example](.env.example).

| Environment Variable | Default | Description |
|---|---|---|
| `XAVIER2_TOKEN` | generated at startup if unset in current HTTP mode | Authentication token |
| `XAVIER2_DEV_MODE` | `false` | Skip auth in explicit development scenarios |
| `XAVIER2_CONFIG_PATH` | `config/xavier2.config.json` | Optional override for the canonical runtime config file |
| provider keys | unset | External API credentials |

## Documentation

- Feature status: [docs/FEATURE_STATUS.md](docs/FEATURE_STATUS.md)
- CLI reference: [docs/guides/CLI_REFERENCE.md](docs/guides/CLI_REFERENCE.md)
- Public API reference: [docs/site/src/content/docs/reference/api.md](docs/site/src/content/docs/reference/api.md)
- Release roadmap: [docs/PUBLIC_RELEASE_ROADMAP.md](docs/PUBLIC_RELEASE_ROADMAP.md)

## License

MIT
