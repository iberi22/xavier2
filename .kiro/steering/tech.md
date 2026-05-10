# Xavier — Tech Stack & Build System

## Language & Runtime

| Component | Technology |
|-----------|-----------|
| Primary language | Rust (Edition 2021) |
| Async runtime | Tokio (full features) |
| HTTP framework | Axum 0.8 (with WebSocket support) |
| Agent framework | `zavora-ai/adk-rust` |
| Memory / vector store | SQLite + `sqlite-vec` |
| Code index sidecar | `code-graph` (SQLite, path `./code-graph`) |
| UI (desktop) | egui / eframe (optional feature) |
| TUI dashboard | ratatui + crossterm (optional feature) |
| Panel UI | React + Vite (Tauri app in `panel-ui/`) |
| Docs site | Astro Starlight (`docs/site/`) |

## Key Rust Dependencies

- `axum 0.8` — HTTP server
- `rusqlite 0.32` + `sqlite-vec 0.1` — durable vector memory
- `tokio 1.52` — async runtime
- `clap 4` — CLI argument parsing
- `serde` / `serde_json` — serialization
- `tracing` / `tracing-subscriber` — structured logging
- `aes-gcm`, `argon2`, `sha2` — E2E encryption and security
- `proptest 1.11` — property-based testing (dev)
- `criterion 0.5` — benchmarking (dev)

## Cargo Features

| Feature | Description |
|---------|-------------|
| `default` | `cli-interactive` + `local-gllm` |
| `cli-interactive` | ratatui TUI dashboard |
| `egui-standalone` | Native egui desktop UI |
| `telegram` | Telegram bot via teloxide |
| `local-gllm` | Local LLM inference (gllm + candle-core) |
| `enterprise` | Cortex Enterprise Cloud integration |
| `ci-safe` | No GUI/Winit — safe for CI |

## Binaries

| Binary | Entry Point | Notes |
|--------|-------------|-------|
| `xavier` | `src/main.rs` | Main server + CLI |
| `xavier-gui` | `src/main_egui.rs` | Requires `egui-standalone` feature |
| `xavier-tui` | `src/main_tui.rs` | Requires `cli-interactive` feature |

## Runtime Configuration (Environment Variables)

| Variable | Purpose |
|----------|---------|
| `XAVIER_HOST` | Bind host (default `127.0.0.1`) |
| `XAVIER_PORT` | Bind port (default `8003`) |
| `XAVIER_CODE_GRAPH_DB_PATH` | Path to code-graph SQLite DB |
| `XAVIER_RRF_K` | RRF fusion constant for hybrid search |

LLM provider defaults: `ModelProviderKind::Local` checked first; Ollama at `localhost:11434`. External providers (Gemini, OpenAI, Groq) require explicit API keys.

## Common Commands

### Rust / Cargo

```bash
# Build release binary
cargo build --release

# Run all tests
cargo test -p xavier

# Unit tests only
cargo test --lib

# Integration tests
cargo test -p xavier

# E2E tests
cargo test --test e2e -- --nocapture

# Lint (zero warnings enforced)
cargo clippy -p xavier --all-targets -- -D warnings

# Format check
cargo fmt --check

# Benchmarks
cargo bench --bench api_v1
```

### Docker

```bash
# Start full stack
docker compose up -d

# Health check
curl http://localhost:8003/health
curl http://localhost:8003/readiness
```

### Node / Frontend

```bash
# Install all workspaces
npm install --workspaces

# Build panel UI + docs site
npm run build

# Build panel UI only
npm run build:panel

# Build docs site only
npm run build:docs

# Lint JS/TS
npm run lint
```

### Release Smoke Test

```bash
# PowerShell
./scripts/release-smoke.ps1

# Bash
./scripts/release-smoke.sh
```

## Node Workspace Structure

Root `package.json` manages two npm workspaces:
- `panel-ui` — React/Vite/Tauri desktop panel
- `docs/site` — Astro Starlight documentation site

Node ≥ 22.12.0 required.
