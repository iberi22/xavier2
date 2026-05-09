# Xavier Architecture Overview

## Memory Stores

### Available Stores

Xavier supports multiple memory store backends via the `MemoryBackend` enum:

| Backend | Description | Key Features |
|---------|-------------|--------------|
| `Surreal` | SurrealDB native | Full-featured, primary production backend |
| `Sqlite` | SQLite (rusqlite) | ACID-compliant fallback, simpler than Surreal |
| `Vec` | SQLite + sqlite-vec | HNSW-like ANN vector search, hybrid retrieval |
| `Memory` | In-memory | Ephemeral, testing/dev only |
| `File` | File-based | Simple JSON persistence |

### Store Files in `src/memory/`

- `store.rs` — `MemoryStore` trait & `StoreManager`
- `surreal_store.rs` — SurrealDB implementation (~1296 lines)
- `sqlite_vec_store.rs` — Vec-backed SQLite with HNSW (~2573 lines)
- `qmd_memory.rs` — In-memory document store (~105KB, see issue #137)
- `schema.rs` — `MemoryKind`, `MemoryNamespace`, `MemoryRecord` types
- `manager.rs` — StoreManager for backend selection
- `embedder.rs` — Embedding client interface
- `semantic.rs`, `semantic_cache.rs` — Semantic layer
- `episodic.rs` — Episodic memory layer
- `working.rs` — Working memory
- `belief_graph.rs`, `entity_graph.rs` — Graph-based memory
- `checkpoint_summary.rs` — Checkpoint/summary logic
- `file_indexer.rs` — File indexing

### VecSqliteMemoryStore vs SurrealStore Comparison

| Aspect | SurrealStore | VecSqliteStore |
|--------|--------------|----------------|
| **Vector Search** | Via SurrealDB internals | Via sqlite-vec HNSW |
| **Storage** | Network database | Local SQLite file |
| **Hybrid Search** | Limited | Full Text + Vector + KG fusion |
| **RRF Scoring** | Basic | Configurable weights (0.40/0.35/0.25) |
| **Dependencies** | surrealdb crate (3.0.5) | rusqlite + sqlite-vec |
| **Concurrency** | Async network ops | WAL mode enabled |
| **Setup** | Requires running SurrealDB | Embedded, no server needed |

### Memory Kinds (24 types)

`Episodic`, `Semantic`, `Procedural`, `Belief`, `Org`, `Workspace`, `User`, `Agent`, `Session`, `Event`, `Fact`, `Decision`, `Repo`, `Branch`, `File`, `Symbol`, `Url`, `Task`, `Contact`, `Meeting`, `ContentProject`, `VideoAsset`, `Document`

---

## MCP Capabilities (or lack thereof)

### Existing MCP Implementation

MCP support exists in `src/server/mcp_server.rs` and `mcp_stdio.rs`:

**Implemented:**
- JSON-RPC request/response handling
- MCP tool definitions (`MCPTool` struct)
- Xavier-specific tools: `create_memory`, `search_memory`, `get_memory`, `list_projects`, `get_project_context`
- Input schemas for each tool

**Gap (Issue #139):**
- No `MemoryFragment`-compatible layer for Gestalt schema
- Missing tools: `save_fragment`, `search_by_agent_context_tags`, `get_recent`
- MCP tools exist but don't map directly to Gestalt's `MemoryFragment` fields

### Gestalt Integration Context

Gestalt requires these operations (from issue #149):
| Gestalt Operation | Xavier Equivalent | Status |
|-------------------|-------------------|--------|
| `save()` MemoryFragment | `POST /memory/add` | Needs field mapping |
| `search()` | `POST /memory/search` | Exists, superior (hybrid) |
| `recent()` | No equivalent | Missing endpoint |
| `build_context_string()` | `POST /memory/search` + formatting | Partial |
| `polling timeline_events` | **No equivalent** | ⚠️ Critical gap |

---

## Open Issues Summary

| # | Title | Priority | Labels |
|---|-------|----------|--------|
| **#149** | [MIGRATION] Gestalt Rust → Xavier: Unificar memoria como backend único | P1 | migration |
| **#139** | [FEATURE] Add MCP protocol handler for Gestalt MemoryFragment schema | P2 | jules, enhancement |
| **#137** | [PERF] qmd_memory.rs is 105KB - needs modularization/split | P2 | performance, jules |
| **#127** | [refactor] Extract magic constants to config in gating.rs | P2 | refactor, jules |
| **#125** | [fix] Replace unwrap/expect with error handling in http.rs | P2 | bug, jules |
| **#120** | jules: Core Review - Code Quality Analysis | P2 | jules, enhancement, review |
| **#115** | P2: Constantes mágicas hardcoded | P2 | technical-debt |
| **#100** | (unknown - not fetched) | - | - |
| **#101** | (unknown - not fetched) | - | - |
| **#99** | (unknown - not fetched) | - | - |
| **#98** | (unknown - not fetched) | - | - |
| **#97** | (unknown - not fetched) | - | - |

---

## Migration Readiness

### What Exists for Gestalt Integration

✅ **HTTP API Endpoints:**
- `POST /memory/search` — hybrid search (text + vector + KG)
- `POST /memory/add` — add memory
- `POST /memory/query` — flexible query
- `GET /memory/stats` — statistics
- `POST /xavier/agents/{id}/push` — push context to agent

✅ **MCP Server:** Basic MCP protocol handler with Xavier tools

✅ **Schema:** Rich `MemoryRecord` schema with embeddings, metadata, revisions

### Critical Gaps for Migration

1. **No timeline events streaming/polling endpoint**
   - Gestalt uses 500ms polling to `timeline_events` table
   - Xavier has no equivalent `GET /timeline/events?since=` endpoint
   - Options: REST polling endpoint, WebSocket/SSE streaming

2. **Missing `recent()` endpoint**
   - No direct equivalent for "get recent memories per agent"
   - Can approximate via `POST /memory/query` with time filters

3. **No MemoryFragment-compatible MCP tools**
   - Issue #139 specifically addresses this
   - Need MCP tools matching Gestalt schema fields

### Minimum Viable Product for Migration

To enable Gestalt as a Xavier memory client:

1. **Add timeline polling endpoint:**
   ```
   GET /timeline/events?since=<timestamp>&agent_id=<id>
   ```
   Returns events since timestamp for real-time observation

2. **Add recent memories endpoint:**
   ```
   GET /memory/recent?agent_id=<id>&limit=<n>
   ```
   Returns most recent memories for an agent

3. **Add MemoryFragment MCP tools:**
   - `save_fragment(agent_id, content, context, tags, provenance)`
   - `search_fragments(query, agent_id, context, tags, limit)`
   - `get_recent_fragments(agent_id, limit)`

4. **Schema mapping:**
   - Map Gestalt `MemoryFragment` → Xavier `MemoryRecord`
   - Fields: `content` → `content`, `tags` → `metadata.tags`, `provenance` → metadata

### Dependencies / Blockers

- Issue #139 (MCP for MemoryFragment) blocks clean Gestalt integration
- Issue #149 proposes full migration plan but is tracking issue for the migration itself
- No SurrealDB upgrade path for RUSTSEC-2023-0071 (Marvin Attack) — no fixed version available
