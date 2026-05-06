# Xavier2 Architecture

> **Version:** 0.6.0-beta  
> **License:** MIT  

Xavier2 is a **cognitive memory runtime for AI agents** built in Rust. It follows a **hexagonal (ports & adapters) architecture** to keep the core domain logic decoupled from frameworks, databases, and transport protocols.

---

## Table of Contents

1. [Hexagonal Architecture Overview](#1-hexagonal-architecture-overview)
2. [Ports (Inbound & Outbound)](#2-ports-inbound--outbound)
3. [Memory System](#3-memory-system)
4. [Embedding Pipeline](#4-embedding-pipeline)
5. [Search System](#5-search-system)
6. [Consolidation System](#6-consolidation-system)
7. [Layer Mapping](#7-layer-mapping)

---

## 1. Hexagonal Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                     Entry Points                         │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐ │
│   │   CLI    │  │   HTTP   │  │   MCP    │  │  TUI   │ │
│   │ (clap)   │  │  (axum)  │  │ (stdio)  │  │(ratatui)│ │
│   └────┬─────┘  └────┬─────┘  └────┬─────┘  └───┬────┘ │
│        │              │              │             │      │
│   ┌────▼──────────────▼──────────────▼─────────────▼──┐  │
│   │               Adapters (Inbound)                    │  │
│   │  HTTP routes, CLI handlers, MCP tool handlers,     │  │
│   │  DTOs, auth middleware                             │  │
│   └────────────────────┬───────────────────────────────┘  │
│                        │                                   │
│   ┌────────────────────▼───────────────────────────────┐  │
│   │              Ports (Inbound Traits)                  │  │
│   │  MemoryQueryPort  │  AgentLifecyclePort  │          │  │
│   │  SessionPort      │  SecurityPort        │          │  │
│   │  HealthPort       │  VerificationPort    │  ...     │  │
│   └────────────────────┬───────────────────────────────┘  │
│                        │                                   │
│   ┌────────────────────▼───────────────────────────────┐  │
│   │                 Domain / App Layer                   │  │
│   │  ┌──────────────────────────────────────────────┐   │  │
│   │  │  Application Services                        │   │  │
│   │  │  SecurityService, PatternService,            │   │  │
│   │  │  QmdMemoryAdapter, VerificationService       │   │  │
│   │  └──────────────────────────────────────────────┘   │  │
│   │  ┌──────────────────────────────────────────────┐   │  │
│   │  │  Domain Models                               │   │  │
│   │  │  MemoryRecord, TimeMetric, MemoryQueryFilters │   │  │
│   │  └──────────────────────────────────────────────┘   │  │
│   └────────────────────┬───────────────────────────────┘  │
│                        │                                   │
│   ┌────────────────────▼───────────────────────────────┐  │
│   │              Ports (Outbound Traits)                 │  │
│   │  EmbeddingPort  │  AgentRuntimePort  │              │  │
│   │  HealthCheckPort                                    │  │
│   └────────────────────┬───────────────────────────────┘  │
│                        │                                   │
│   ┌────────────────────▼───────────────────────────────┐  │
│   │               Adapters (Outbound)                    │  │
│   │  OpenAI-compatible embedder, HTTP health adapter,    │  │
│   │  Pattern adapter (vector store)                      │  │
│   └─────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Layer Responsibilities

| Layer | Directory | Responsibility |
|-------|-----------|----------------|
| **Entry Points** | `src/bin/`, `src/cli.rs` | Binary entry points: `xavier2 http`, `xavier2 mcp`, `xavier2` (CLI) |
| **Inbound Adapters** | `src/adapters/inbound/` | HTTP routes (`axum`), CLI handler bindings, request DTOs, auth middleware |
| **Inbound Ports** | `src/ports/inbound/` | Trait definitions for what the application provides (memory queries, agent lifecycle, health, etc.) |
| **Domain** | `src/domain/` | Pure domain models: `MemoryRecord`, `MemoryQueryFilters`, `TimeMetric` |
| **App Services** | `src/app/` | Application orchestration: security service, pattern service, verification, memory adapter |
| **Outbound Ports** | `src/ports/outbound/` | Trait definitions for what the application needs from external systems (embedding, agent runtime, health checks) |
| **Outbound Adapters** | `src/adapters/outbound/` | Concrete implementations: OpenAI-compatible embedding client, health check reporter |

### Dependency Rule

Dependencies point **inward**: Entry points → Adapters → Ports → Domain. The domain layer has zero dependencies on infrastructure. Port traits are defined in `src/ports/` and implemented by adapters in `src/adapters/`.

---

## 2. Ports (Inbound & Outbound)

### Inbound Ports (`src/ports/inbound/`)

| Trait | Purpose | Key Methods |
|-------|---------|-------------|
| `MemoryQueryPort` | Core memory operations | `search()`, `add()`, `delete()`, `get()`, `list()` |
| `AgentLifecyclePort` | Agent lifecycle management | `register()`, `spawn()`, `shutdown()`, `list_agents()` |
| `SessionPort` | Session management | `create()`, `get()`, `update()`, `close()` |
| `SessionSyncPort` | Session syncing | `sync_check()`, `needs_sync()`, `mark_synced()` |
| `HealthPort` | System health | `check()`, `HealthStatus` |
| `SecurityPort` | Input security scanning | `scan()`, `sanitize()` |
| `InputSecurityPort` | Input-level security | `validate()`, `sanitize()` |
| `PatternDiscoverPort` | Pattern discovery | `discover()`, `analyze()` |
| `VerificationPort` | Verification checks | `verify()` |
| `TimeMetricsPort` | Time tracking | `record_metric()`, `query_metrics()` |

### Outbound Ports (`src/ports/outbound/`)

| Trait | Purpose | Key Methods |
|-------|---------|-------------|
| `EmbeddingPort` | Text-to-vector embedding | `embed()` |
| `AgentRuntimePort` | Agent execution runtime | `run()`, `status()` |
| `HealthCheckPort` | External health checks | `check()`, `is_healthy()` |

---

## 3. Memory System

Xavier2's memory system has three tiers, each serving a different purpose in the agent's cognitive architecture.

### 3.1 Memory Store Backends

Defined by the `MemoryBackend` enum in `src/memory/store.rs`:

| Backend | Description | When to Use |
|---------|-------------|-------------|
| **Vec** (default) | SQLite + `sqlite-vec` with HNSW-like vector search, FTS5 full-text search, and knowledge graph fusion | Production. Best hybrid search. |
| **Sqlite** | Plain SQLite via `rusqlite`, ACID-compliant | When vectors aren't needed. Simpler. |
| **File** | JSON file persistence | Development, testing. |
| **Memory** | In-memory `Vec<MemoryDocument>` | Ephemeral/testing only. |

### 3.2 Core Types

```rust
// Primary memory document (src/memory/qmd_memory.rs)
struct MemoryDocument {
    id: Option<String>,
    path: String,
    content: String,
    metadata: serde_json::Value,
    embedding: Option<Vec<f32>>,
}

// Memory store trait (src/memory/store.rs)
trait MemoryStore {
    async fn add(&self, record: MemoryRecord) -> Result<String>;
    async fn get(&self, id: &str) -> Result<Option<MemoryRecord>>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryRecord>>;
    async fn search_filtered(&self, query: &str, limit: usize, filters: Option<&MemoryQueryFilters>) -> Result<Vec<MemoryDocument>>;
    async fn delete(&self, id: &str) -> Result<Option<MemoryDocument>>;
    async fn all_documents(&self) -> Vec<MemoryDocument>;
}
```

### 3.3 Memory Layers (Adaptive Retrieval)

The retrieval module (`src/retrieval/`) implements a **three-layer memory model**:

| Layer | Source | Description |
|-------|--------|-------------|
| **Working** | `QmdMemory` (in-memory) | Active session documents, recent context |
| **Episodic** | Panel thread store | Historical session summaries with timestamps |
| **Semantic** | Entity graph | Long-term concepts, entities, and relationships |

These layers are combined using **weighted RRF fusion** via the `AdaptiveGating` system, which allows tunable weights per layer.

### 3.4 Key Memory Files

| File | Purpose |
|------|---------|
| `src/memory/qmd_memory.rs` | In-memory document store with cached BM25 + vector search (~105KB) |
| `src/memory/sqlite_vec_store.rs` | SQLite-backed store with `sqlite-vec` HNSW vector search (~2.7K lines) |
| `src/memory/sqlite_store.rs` | Plain SQLite store (ACID, no vectors) |
| `src/memory/store.rs` | `MemoryStore` trait, `MemoryRecord`, `MemoryBackend` enum |
| `src/memory/schema.rs` | `MemoryKind` (24 types), `MemoryQueryFilters`, `TypedMemoryPayload` |
| `src/memory/manager.rs` | Store lifecycle, auto-management, decay coordination |
| `src/memory/embedder.rs` | Embedding client interface |
| `src/memory/belief_graph.rs` | Belief graph nodes and relations |
| `src/memory/entity_graph.rs` | Entity extraction and graph storage |
| `src/memory/working.rs` | Working memory (active context) |
| `src/memory/episodic.rs` | Episodic memory (session history) |
| `src/memory/semantic.rs` | Semantic memory (long-term knowledge) |

### 3.5 Memory Kinds (24 Types)

All memory can be tagged with a `MemoryKind`:

`Episodic`, `Semantic`, `Procedural`, `Belief`, `Org`, `Workspace`, `User`, `Agent`, `Session`, `Event`, `Fact`, `Decision`, `Repo`, `Branch`, `File`, `Symbol`, `Url`, `Task`, `Contact`, `Meeting`, `ContentProject`, `VideoAsset`, `Document`

---

## 4. Embedding Pipeline

### Architecture

```
Text Input
    │
    ▼
Embedder trait (async fn encode(&self, text: &str) -> Result<Vec<f32>>)
    │
    ├── OpenAICompatibleEmbedder (primary)
    │     ├── Endpoint: configurable (default cloud: api.openai.com / local: localhost:11434)
    │     ├── Model: configurable (default cloud: text-embedding-3-small / local: embeddinggemma)
    │     └── API key: XAVIER2_EMBEDDING_API_KEY or OPENAI_API_KEY
    │
    └── FallbackEmbedder (optional)
          └── Secondary embedder for resilience
```

### Configuration

The embedding system in `src/embedding/` supports three modes, controlled by `XAVIER2_EMBEDDING_PROVIDER_MODE`:

| Mode | Auto-detection | Behavior |
|------|---------------|----------|
| **Local** | `XAVIER2_EMBEDDING_ENDPOINT`, `XAVIER2_EMBEDDING_URL`, `XAVIER2_EMBEDDING_MODEL` set, or explicit `XAVIER2_EMBEDDING_PROVIDER_MODE=local` | Uses local endpoint (e.g., Ollama) |
| **Cloud** | `OPENAI_API_KEY` or `XAVIER2_EMBEDDING_API_KEY` set, or explicit `XAVIER2_EMBEDDING_PROVIDER_MODE=cloud` | Uses OpenAI-compatible cloud endpoint |
| **Disabled** | Explicit `XAVIER2_EMBEDDING_PROVIDER_MODE=disabled` | Returns empty vectors (keyword-only search) |

If both local and cloud signals are present, Xavier2 configures a **primary + fallback** chain automatically.

### Known Embedding Models & Dimensions

| Model | Dimensions |
|-------|-----------|
| `nomic-embed-text` / `nomic-embed-text-v1.5` | 768 |
| `embeddinggemma` | 768 |
| `all-minilm` | 384 |
| `qwen3-embedding` | 1024 |
| `text-embedding-3-small` | 1536 |
| `text-embedding-3-large` | 3072 |

---

## 5. Search System

Xavier2 provides three search methods, all accessible via HTTP API and internal APIs.

### 5.1 Keyword Search (BM25)

- Uses SQLite FTS5 full-text search
- Stemming-aware tokenization
- Fast, deterministic — no external service needed
- Configuration: `src/search/bm25.rs`

### 5.2 Semantic Search (Vector)

- Converts query to embedding vector via the configured embedder
- Performs approximate nearest neighbor (ANN) search via `sqlite-vec` HNSW
- Returns documents ranked by cosine similarity
- Configuration: `src/memory/sqlite_vec_store.rs`

### 5.3 Hybrid Search (Keyword + Vector + KG Fusion)

Uses **Reciprocal Rank Fusion (RRF)** to combine multiple retrieval strategies:

```rust
// Weighted RRF formula:
score(d) = Σ [ weight_i / (k + rank_i(d)) ]

// Default weights:
vector: 0.40
fts:    0.35
kg:     0.25
rrf_k:  60
```

The hybrid searcher in `src/search/hybrid.rs`:

1. Runs **pre-query hooks** (query expansion, rewriting)
2. Executes keyword (BM25) and vector search in parallel
3. Fuses results via weighted RRF in `src/search/rrf.rs`
4. Deduplicates by path, keeping the most recent `updated_at`
5. Runs **post-query hooks** (reranking, filtering)
6. Returns top-N results

#### Search Hooks

The `HookRegistry` in `src/search/hooks.rs` allows extending search behavior:

| Hook Type | When It Runs | Use Case |
|-----------|-------------|----------|
| **Pre-query** | Before search | Query expansion, synonym replacement, intent classification |
| **Post-query** | After fusion | Result reranking, filtering, augmentation |

### 5.4 Multi-Layer Retrieval

The `POST /memory/retrieve` endpoint combines all three memory layers (working, episodic, semantic) using `AdaptiveGating` with tunable weights:

```json
{
  "layer_weights": { "working": 0.3, "episodic": 0.3, "semantic": 0.4 },
  "relevance_threshold": 0.5,
  "rrf_k": 60
}
```

---

## 6. Consolidation System

The consolidation module (`src/consolidation/`) manages memory health through two main operations.

### 6.1 Consolidation (`POST /memory/consolidate`)

The consolidation task performs:

1. **Selection** — Picks the top-N memories by quality score and access count
2. **Clustering** — Groups similar memories by embedding similarity (threshold: 0.88)
3. **Merging** — Combines clusters into a single canonical memory, removing duplicates
4. **Decay** — Applies exponential decay to importance scores:
   ```
   importance_decayed = importance * decay_rate ^ age_days
   ```
5. **Cleanup** — Removes memories below `min_importance_for_decay` (default: 0.30)

### 6.2 Reflection (`POST /memory/reflect`)

Reflection identifies low-importance or aged memories and:

1. **Summarizes** them using an LLM (if configured) into a concise reflection document
2. **Themes** — Extracts recurring themes and notes
3. **Cleanup** — Removes source memories that are fully covered by the summary

### Key Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `batch_size` | 32 | Memories processed per consolidation run |
| `similarity_threshold` | 0.88 | Cosine similarity for cluster merging |
| `decay_rate` | 0.94 | Exponential decay factor per day |
| `min_importance_for_decay` | 0.30 | Minimum importance before removal |
| `reflection_age_days` | 30 | Age threshold for reflection candidates |
| `cleanup_similarity_threshold` | 0.91 | How similar a source must be to summary to be removed |

---

## 7. Layer Mapping

```
src/
├── lib.rs              # Crate root, AppState, module declarations
├── main.rs             # Binary entry point
├── cli.rs              # CLI parser & command handlers (Clap ~88KB)
├── settings.rs         # Runtime configuration
│
├── adapters/
│   ├── inbound/        # HTTP routes, DTOs, auth middleware
│   │   └── http/
│   └── outbound/       # Embedding client, health reporter
│       ├── embedding/
│       └── vec/
│
├── app/                # Application services
│   ├── security_service.rs
│   ├── pattern_service.rs
│   ├── qmd_memory_adapter.rs
│   └── verification_service.rs
│
├── domain/
│   ├── memory/         # MemoryRecord, MemoryQueryFilters, TimeMetric
│   ├── belief/
│   └── pattern/
│
├── ports/
│   ├── inbound/        # Port trait definitions
│   └── outbound/       # External port trait definitions
│
├── memory/             # Memory system implementation
│   ├── qmd_memory.rs   # In-memory document store
│   ├── sqlite_vec_store.rs  # Vec-backed SQLite store
│   ├── sqlite_store.rs # Plain SQLite store
│   ├── store.rs        # MemoryStore trait
│   ├── schema.rs       # Types & serialization
│   ├── manager.rs      # Store lifecycle
│   └── ...             # Layers, embedder, graphs, cache
│
├── embedding/          # Embedding pipeline
│   ├── mod.rs          # EmbedderConfig, OpenAICompatibleEmbedder, FallbackEmbedder
│   └── openai.rs       # HTTP embedding client
│
├── search/             # Search engine
│   ├── hybrid.rs       # HybridSearcher (keyword + vector fusion)
│   ├── rrf.rs          # Reciprocal Rank Fusion
│   ├── bm25.rs         # BM25 keyword scoring
│   └── hooks.rs        # Pre/post query hooks
│
├── consolidation/      # Memory consolidation & reflection
│   ├── mod.rs          # ConsolidationTask, ConsolidationStats, ReflectionStats
│   ├── merger.rs       # Document merging, clustering, importance scoring
│   └── reflection.rs   # LLM-based reflection and summarization
│
├── retrieval/          # Multi-layer adaptive retrieval
│   ├── gating.rs       # AdaptiveGating — weighted layer fusion
│   └── config.rs       # Retrieval configuration
│
├── server/             # HTTP / MCP server handlers
│   ├── http.rs         # Axum routes, ShutdownState, health/readiness endpoints
│   ├── v1_api.rs       # V1 RESTful memory API
│   ├── mcp_server.rs   # MCP protocol server
│   ├── mcp_stdio.rs    # MCP stdio transport
│   └── panel.rs        # Admin panel routes
│
├── agents/             # Agent runtime
│   ├── provider.rs     # Model provider routing
│   ├── runtime.rs      # Agent execution runtime
│   └── ...
│
├── session/            # Session and conversation management
├── tasks/              # Background task coordination
├── tools/              # Agent tool system
├── crypto/             # Encryption and hashing utilities
├── security/           # Input sanitization, prompt guard
├── verification/       # Output verification
├── utils/              # Shared utilities
├── workspace.rs        # Workspace management
├── coordination/       # Multi-agent coordination
├── scheduler/          # Task scheduling
├── sync/               # Cross-instance synchronization
├── checkpoint/         # State checkpointing
├── consistency/        # Coherence reporting and regularization
├── time/               # Time metrics and tracking
├── context/            # Context management
├── billing/            # Usage metering and billing
├── secrets/            # Secret management
├── a2a/                # Agent-to-agent protocol
└── ui/                 # UI components (panel)
```

---

*Xavier2 — Cognitive Memory Runtime for AI Agents. Built with Rust.*
