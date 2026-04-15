---
title: Memory Module
description: Core memory operations and storage
---

# Memory Module

The memory module is the core of Xavier2. It stores, retrieves, and searches workspace memories through a common `MemoryStore` abstraction.

## Current Runtime Truth

- The current validated runtime defaults to `FileMemoryStore`.
- The persisted file path in Docker is typically `/data/workspaces/<workspace>/memory-store.json`.
- SurrealDB remains present in the codebase as an optional or future-facing direction, not as the default validated backend.

## Main Components

### `MemoryItem`

The fundamental unit stored in the runtime:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub content: String,
    pub path: String,
    pub metadata: Metadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### `MemoryStore`

The runtime uses a trait-based boundary so storage backends can be swapped without changing the higher-level API surface.

```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn add(&self, item: MemoryItem) -> Result<MemoryId, MemoryError>;
    async fn get(&self, id: &MemoryId) -> Result<Option<MemoryItem>, MemoryError>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, MemoryError>;
    async fn delete(&self, id: &MemoryId) -> Result<(), MemoryError>;
    async fn list(&self, path: &str) -> Result<Vec<MemoryItem>, MemoryError>;
}
```

### Active Default Backend

For the current deployment story, Xavier2 uses the file-backed runtime store. That is the backend to assume when documenting behavior, recovery, and local persistence.

### Optional SurrealDB Direction

SurrealDB is still relevant to the architecture, especially for broader hosted durability goals, but it should be described as optional or future-facing until it is revalidated as the active runtime backend.

## Example Usage

### Add Memory

```rust
use xavier2::memory::{MemoryManager, MemoryItem};

let manager = MemoryManager::new();

let item = MemoryItem::new(
    "Xavier2 stores this memory in the active file-backed runtime",
    "xavier2/storage",
);

let id = manager.add(item).await?;
println!("Memory added with ID: {}", id);
```

### Search Memory

```rust
let results = manager.search("file-backed runtime", 10).await?;

for result in results {
    println!("Score: {:.2}", result.score);
    println!("Content: {}", result.item.content);
}
```

## Operational Notes

- Auth is enforced at the HTTP layer through `X-Xavier2-Token`.
- `GET /health` and `GET /readiness` are intentionally public.
- JWT/RBAC code exists elsewhere in the repo, but it is not the active server auth path.
- Retrieval quality is strong in the latest benchmark, but the **2026-04-11** run still reported **1006.4ms** average latency, above the older `< 500ms` target.

## Verification

Use the same verification path referenced in the aligned project docs:

```bash
cargo test --workspace --features ci-safe --exclude xavier2-web
npm run build --workspace panel-ui
npm run build --workspace docs/site
```

## Related Docs

- [Quick Start](/guides/quick-start/)
- [Architecture Overview](/architecture/overview/)
- [API Reference](/reference/api/)
- [Testing Overview](/testing/overview/)
