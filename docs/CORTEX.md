# Xavier

> Cognitive memory infrastructure for agent workflows.

Xavier is a Rust-native memory system for AI agents. It combines hybrid retrieval, long-horizon context, code indexing, and authenticated HTTP access so agents can reuse knowledge across sessions, tasks, and repositories.

The recommended operational path is local HTTP/CLI usage with authenticated `curl`. MCP is supported, but it is a secondary transport for IDE-native integrations.

## Core Positioning

Xavier is not just a RAG add-on. It is the memory substrate for agentic workflows:

- persistent project memory
- reusable research and architectural context
- hybrid retrieval for code and documents
- HTTP integration for scripts and external agents
- MCP integration for IDE-native tool transport
- graph-oriented relationships for belief and concept tracking

## System Overview

```text
┌─────────────────────────────────────────────────────────────┐
│                        XAVIER CORE                          │
├─────────────────────────────────────────────────────────────┤
│  Retrieval     Reasoning     Runtime     HTTP / MCP        │
│  Memory Store  Belief Graph  Agents      Integration       │
└─────────────────────────────────────────────────────────────┘
```

## Main Subsystems

| Subsystem | Purpose | Current Shape |
|-----------|---------|---------------|
| `memory/` | document, graph, and search memory | hybrid search + in-process runtime memory with SurrealDB direction |
| `agents/` | runtime behavior and orchestration | System 1 / 2 / 3 workflow |
| `server/` | external access surface | authenticated HTTP endpoints and MCP-adjacent server surface |
| `tools/` | agent-facing operations | search, validation, and integration helpers |
| `checkpoint/` | resumable state primitives | session and state continuity |

## Runtime Interfaces

### HTTP

Use this as the default local integration surface. For automation, smoke checks, and debugging, prefer these authenticated HTTP endpoints over MCP.

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/health` | service health |
| `GET` | `/readiness` | runtime and dependency readiness |
| `GET` | `/build` | build and provider metadata |
| `POST` | `/memory/add` | store memory |
| `POST` | `/memory/delete` | delete memory by `id` or `path` |
| `POST` | `/memory/reset` | reset in-memory documents |
| `POST` | `/memory/search` | search memory |
| `POST` | `/memory/query` | query through the runtime |
| `GET` | `/memory/graph` | inspect belief graph data |
| `POST` | `/agents/run` | run the agent runtime with session tracking |
| `POST` | `/sync` | report syncable memory count |
| `POST` | `/code/scan` | index a source tree |
| `POST` | `/code/find` | search indexed symbols |
| `GET` | `/code/stats` | inspect code index stats |

All HTTP endpoints except `/health` and `/readiness` require `X-Xavier-Token`, and every response carries `X-Request-Id`.

### Example

```bash
curl -X POST http://localhost:8003/memory/search \
  -H "X-Xavier-Token: dev-token" \
  -H "Content-Type: application/json" \
  -d '{"query":"xavier memory","limit":5}'
```

### MCP

Use MCP only when a local IDE or tool host requires MCP transport. For local scripts and operational workflows, HTTP/`curl` is the preferred interface.

The current documented tool surface aligns with the in-repo implementation:

- `create_memory`
- `search_memory`
- `get_memory`
- `list_projects`
- `get_project_context`
- `sync_gitcore`

## Storage Backends

Xavier supports three storage backends for memory persistence, controlled via `XAVIER_MEMORY_BACKEND`:

| Backend | Value | Best For | Limits |
|---------|-------|----------|--------|
| File (default) | `file` | Development, single-instance | ~50k records, single workspace |
| SurrealDB | `surreal` | Production, multi-workspace, scaling | Unlimited with clustering |
| SQLite | `sqlite` | Lightweight fallback, embedded | ~500k records |

### Switching Backends

Set `XAVIER_MEMORY_BACKEND` before starting Xavier:

```bash
# File (default)
XAVIER_MEMORY_BACKEND=file

# SurrealDB
XAVIER_MEMORY_BACKEND=surreal
XAVIER_SURREALDB_URL=ws://surrealdb:8000
XAVIER_SURREALDB_USER=root
XAVIER_SURREALDB_PASS=your-password

# SQLite
XAVIER_MEMORY_BACKEND=sqlite
XAVIER_MEMORY_SQLITE_PATH=./data/workspaces/default/memory-store.sqlite3
```

### Data Migration

Use the migration scripts in `scripts/` to move data between backends:

```bash
# File → SurrealDB
python scripts/migrate_file_to_surreal.py --workspace default --reinstall

# File → SQLite
python scripts/migrate_file_to_sqlite.py --workspace default --reinstall

# SurrealDB → SQLite (dump + load via JSON)
python scripts/migrate_file_to_sqlite.py --workspace default --reinstall
```

> **Always back up your data before migrating.** The migration scripts read from the file backend and write to the target backend.

### Backend Benchmarks

| Backend | Write/s | Read/s | Storage | Max Records |
|---------|---------|--------|---------|-------------|
| File | ~200 | ~400 | JSON file | ~50,000 |
| SQLite | ~2,000 | ~5,000 | `.sqlite3` | ~500,000 |
| SurrealDB | ~10,000 | ~20,000 | RocksDB | Unlimited |

*Benchmarks on local NVMe, single workspace, 100-record batches.*

### Health Checks

Each backend exposes health via the HTTP API:

```bash
# Check which backend is active
curl http://localhost:8003/build | jq .backend

# File backend - check workspace disk space
curl http://localhost:8003/readiness | jq .workspace

# SurrealDB backend - check DB connectivity
curl http://localhost:8003/readiness | jq .surrealdb

# SQLite backend - check DB file size
ls -lh ./data/workspaces/default/memory-store.sqlite3
```

## Technology Stack

- Language: Rust
- Runtime: Tokio
- API: Axum
- Memory database: SurrealDB, SQLite, or file (configurable)
- Protocol integration: HTTP and MCP
- Search strategy: hybrid keyword + semantic retrieval

## Operational Notes

- `System3` now uses semantic cache lookups before expensive LLM generation and writes back successful LLM answers.
- Query routing now classifies requests into direct, retrieved, and complex paths to support lower-cost model selection.
- `/readiness` reports workspace, embedding, and LLM readiness.
- `/build` reports version, logging configuration, and provider/model status for diagnostics.
- `/v1/account/usage` now includes optimization counters for routing, semantic cache hits/misses, and LLM calls by model.
- `scripts/release-smoke.ps1` and `scripts/release-smoke.sh` provide basic release validation against a live Xavier instance.

## Monorepo Packages

- Rust workspace: root `xavier`, `code-graph/`, `web/`
- Node workspace: `panel-ui/`, `docs/site/`
- Support assets: `skills/`, `scripts/`, `docker/`, `docs/`

## Agent Workflow Integration

In this repository the contract is explicit:

- GitHub Issues track task state
- Xavier stores reusable memory and durable context
- `.gitcore/ARCHITECTURE.md` defines binding implementation decisions
- `AGENTS.md` defines workflow behavior for all agents

## Documentation Map

- `docs/README.md` for the documentation entrypoint
- `docs/site/` for the published docs site source
- `docs/system/` for the documentation system prototype
- `docs/agent-docs/` for agent-facing specs, research, and archives
- `skills/xavier-http-curl/` for curl-based agent usage without MCP

## Status

Xavier is under active development and already runs locally through Docker with a healthy application surface and an authenticated HTTP API for agent integrations.
