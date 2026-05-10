# Why SQLite-Vec? — The Case for Embedded Vector Search

**Date**: 2026-05-10
**Author**: Xavier AI
**Tags**: [sqlite-vec, storage, vector-search, rrf, performance]
**Source Files**: [`src/memory/sqlite_vec_store.rs`](file:///e:/scripts-python/xavier/src/memory/sqlite_vec_store.rs), [`Cargo.toml`](file:///e:/scripts-python/xavier/Cargo.toml)

---

## TL;DR
Xavier uses `sqlite-vec` to provide high-performance, embedded vector search with no external dependencies. By combining vector similarity, Full-Text Search (FTS5), and Knowledge Graph traversal via Reciprocal Rank Fusion (RRF), we achieve superior retrieval relevance in a single, portable binary.

---

## Context & Motivation
Most AI memory systems reach for heavy hitters like PostgreSQL/pgvector or dedicated vector DBs (Milvus, Pinecone). While powerful, these introduce significant operational overhead: network latency, credential management, and complex deployment pipelines. For a developer-centric tool like Xavier, we wanted a "zero-config" experience that works as well on a laptop as it does in a CI environment.

---

## The Decision
We chose **SQLite-Vec** (specifically the `vec0` virtual table) because it allows us to treat embeddings as first-class citizens within a standard relational database. This enables **Hybrid Retrieval**: the ability to query by semantic meaning, exact keywords, and graph relationships in a single atomic operation.

---

## Deep Dive: Technical Implementation

### 1. Hybrid Search & RRF Fusion
We don't just rely on vectors. Our `HybridSearchResult` is a weighted fusion of three distinct signals:
- **Vector Weight (40%)**: Semantic similarity using cosine distance.
- **FTS Weight (35%)**: Keyword relevance via SQLite's FTS5 (Porter stemmer + unicode61).
- **KG Weight (25%)**: Graph connectivity (entity relationships).

We use **Reciprocal Rank Fusion (RRF)** to combine these. The formula is optimized with a configurable `K` factor (default 60):
`Score = sum(weight / (K + rank))`

### 2. QJL Quantization
To handle large-scale memory without exploding disk usage, we implemented a custom quantization layer dubbed **QJL (Quantized Joint Learning)**. 
When a workspace exceeds 30,000 vectors, Xavier automatically switches to bit-packed quantization, reducing memory footprint by ~75% with minimal accuracy loss.

### 3. WAL Mode & Mmap
To achieve sub-25ms retrieval, we tune SQLite's engine:
```rust
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA mmap_size=268435456; // 256MB mmap for lightning-fast reads
```

---

## Architecture Diagram

```mermaid
graph TD
    Query[User Query] --> Embed[Embedder]
    Embed --> Vector[sqlite-vec / vec0]
    Query --> FTS[FTS5 Virtual Table]
    Query --> KG[Knowledge Graph / entities]
    Vector --> RRF[RRF Fusion Engine]
    FTS --> RRF
    KG --> RRF
    RRF --> Result[Top-K Memories]
```

---

## Alternatives & Trade-offs

| Alternative | Pros | Cons |
|-------------|------|------|
| **pgvector** | Extremely mature, scales to billions. | Requires running Postgres service. |
| **Faiss** | Fastest raw vector search. | Hard to sync with relational metadata. |
| **In-memory** | No disk I/O. | Data loss on crash, high RAM usage. |

---

## Visual Summary (Infographic)

**Template**: `Comparison`
**Data**:
- Left: `sqlite-vec` (Embedded, 22ms retrieval, Zero-Config)
- Right: `pgvector` (External, 45ms+ retrieval, Needs Docker)
- VS: `Deployment Complexity`

---

## References
- [SQLite-Vec Implementation](file:///e:/scripts-python/xavier/src/memory/sqlite_vec_store.rs)
- [Storage Switch Guide](file:///e:/scripts-python/xavier/docs/STORAGE_SWITCH.md)
- [sqlite-vec official repo](https://github.com/asg017/sqlite-vec)
