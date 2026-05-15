# Xavier Architecture Overview

## Core Philosophy

Xavier is built on a **Hexagonal Architecture** (Ports & Adapters) to ensure the core logic remains isolated from external dependencies like database drivers, LLM providers, and transport protocols (HTTP/CLI/MCP).

## Memory Stores

### Primary Backend: SQLite-Vec

As of v0.6+, Xavier has transitioned to **SQLite-Vec** as the primary storage engine for the open-source distribution. This provides a zero-infrastructure, ACID-compliant vector database that can be embedded directly into agent runtimes.

| Backend | Description | Key Features |
|---------|-------------|--------------|
| `Vec` | SQLite + sqlite-vec | **Primary.** HNSW vector search, hybrid RRF retrieval, embedded. |
| `Memory` | In-memory | Ephemeral, used for unit testing and fast-access caches. |
| `Surreal` | SurrealDB native | Optional production backend for high-concurrency cloud deployments. |

### Semantic Layer

- **Belief Graph**: Maps semantic relationships between memories (L0-L1-L2 hierarchy).
- **Hybrid Retrieval**: Uses Reciprocal Rank Fusion (RRF) to combine keyword (FTS5) and semantic (Vector) search.
- **Threat Detection**: Integrated `SecurityScanner` that monitors memory ingestion for prompt injection and leaks.

---

## System Components

### 1. Inbound Ports (Entry Points)

- **HTTP API (`src/server/`)**: High-performance Axum-based REST API with token authentication.
- **CLI (`src/cli/`)**: Command-line interface for local memory operations and administration.
- **MCP (`src/server/mcp/`)**: Model Context Protocol implementation for native AI agent integration.

### 2. Domain Core (`src/app/`)

- **ProxyUseCase**: The central orchestrator that coordinates security, embeddings, and persistence.
- **SecurityService**: Multi-layer scanner (Aho-Corasick, Entropy, Regex) for input validation.

### 3. Outbound Ports (Persistence & Infrastructure)

- **MemoryBackend**: Trait defining persistence operations.
- **EmbeddingPort**: Interface for vector generation (Local GLLM or OpenAI/Anthropic).

---

## 1.0 Release Status

| Feature | Status | Verified? |
|---------|--------|-----------|
| **Hierarchical Memory** | Stable | ✅ (L0-L2 isolation) |
| **Belief Graph** | Stable | ✅ (KG-RRF fusion) |
| **Security Scanner** | Stable | ✅ (Async middleware) |
| **TUI Installer** | Stable | ✅ (6-step wizard) |
| **Public Export** | Stable | ✅ (NDJSON manifest) |

## Development Ecosystem

Xavier uses autonomous agents for continuous improvement:
- **Jules**: Background execution agent for refactoring and clippy fixes.
- **Antigravity**: Strategic architect and integration manager.
- **Cortex**: Synchronization plugin for cloud-native memory distribution.

---

## Technical Debt Resolved

- [x] **Modularization**: `qmd_memory.rs` has been split into dedicated modules.
- [x] **Async Security**: The security pipeline is now fully non-blocking.
- [x] **Shared HTTP Client**: Use of `LazyLock` for global performance optimization.
- [x] **Hierarchical Fields**: All memory records now strictly enforce hierarchy levels.
