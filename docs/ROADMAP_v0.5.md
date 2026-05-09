# Xavier - State-of-the-Art Memory System

**Goal:** Build the best memory system for LLM agents
**Version:** 0.5.0 (target)
**Date:** 2026-04-15
**Status:** PLANNED

---

## Executive Summary

Transformar xavier de un simple vector store a un **cognitive memory runtime** completo con:
- Semantic search (embeddings)
- Hybrid retrieval (vector + keyword + reranking)
- Memory graph (entity relationships)
- Consolidation (replay + importance decay)
- Reflection (self-analysis)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    LLM / Agent                            │
└─────────────────┬─────────────────────────────────────────┘
                  │
                  ▼
┌─────────────────────────────────────────────────────────────┐
│              Xavier Memory System (v0.5)                  │
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐│
│  │ Embedding   │  │ Memory     │  │ Temporal / Entity    ││
│  │ Layer       │  │ Graph      │  │ Tracking             ││
│  │ (vec)       │  │ (beliefs)  │  │                      ││
│  └─────────────┘  └─────────────┘  └─────────────────────┘│
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐│
│  │ Reranker    │  │ Consolidation│ │ Reflection          ││
│  │ (RRF)       │  │ (replay)    │  │ (self-analysis)    ││
│  └─────────────┘  └─────────────┘  └─────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Embeddings 🔴 CRITICAL

**Goal:** Enable semantic search (understand meaning, not just keywords)

### Changes Required

#### `src/api/memory.rs` - Modificar `/memory/add`
```rust
// Agregar campo vector al guardar
pub struct MemoryDoc {
    pub id: String,
    pub content: String,
    pub content_vector: Option<Vec<f32>>,  // NUEVO
    pub metadata: Metadata,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Modificar add_belief para generar embedding
pub async fn add_memory(
    State(state): State<AppState>,
    Json(payload): Json<AddMemoryRequest>,
) -> Result<Json<AddMemoryResponse>, StatusCode> {
    // 1. Generar embedding del content
    let content_vector = state.embedding_model.encode(&payload.content).await?;

    // 2. Guardar en sqlite-vec
    state.vec_store.insert(&payload.content, &content_vector).await?;

    // 3. Guardar documento completo
    state.db.insert_memory(&payload).await?;

    Ok(Json(AddMemoryResponse { id: memory_id }))
}
```

#### `src/embedding/mod.rs` - Nuevo módulo
```rust
pub trait Embedder: Send + Sync {
    fn encode(&self, text: &str) -> impl Future<Output = Result<Vec<f32>>> + Send;
}

// Implementaciones disponibles
pub struct OpenAIEmbedder { api_key: String, model: String }
pub struct MiniMaxEmbedder { api_key: String }
pub struct LocalEmbedder { model_path: PathBuf }  // ONNX/shrimp

// Config via environment
// XAVIER_EMBEDDER=openai|minimax|local
// XAVIER_EMBEDDING_MODEL=text-embedding-3-small
```

### Dependencies to Add
```toml
# Cargo.toml
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1.50", features = ["full"] }

# Para embeddings locales (futuro)
# candle-core = "0.5"  # para local ONNX embeddings
```

### API Changes
```
POST /memory/add
Body: {
    "content": "string",
    "path": "string",
    "metadata": {...}
}

Response: {
    "id": "01KP...",
    "status": "ok",
    "embedding_generated": true  // NUEVO
}
```

### Tests
```rust
#[tokio::test]
async fn test_embedding_generation() {
    let embedder = OpenAIEmbedder::new();
    let vec = embedder.encode("hello world").await.unwrap();
    assert_eq!(vec.len(), 1536); // openai dimensions
}
```

---

## Phase 2: Reranking (RRF) 🔴 CRITICAL

**Goal:** Combine vector + keyword search with Reciprocal Rank Fusion

### Changes Required

#### `src/api/search.rs` - Nuevo endpoint
```rust
// GET /memory/search (actual) + NUEVO POST /memory/hybrid

pub async fn hybrid_search(
    State(state): State<AppState>,
    Json(request): Json<HybridSearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    // 1. Keyword search (sqlite FTS5)
    let keyword_results = state.db.fts_search(&request.query, request.limit * 2).await?;

    // 2. Vector search (sqlite-vec)
    let query_vector = state.embedder.encode(&request.query).await?;
    let vector_results = state.vec_store.search(&query_vector, request.limit * 2).await?;

    // 3. RRF fusion
    let fused = reciprocal_rank_fusion(
        vec![keyword_results, vector_results],
        request.r rf_k.unwrap_or(60),
    );

    // 4. Return top results
    Ok(Json(SearchResponse { results: fused }))
}

fn reciprocal_rank_fusion(results: Vec<Vec<ScoredResult>>, k: u32) -> Vec<ScoredResult> {
    // RRF formula: 1 / (k + rank)
    let mut scores: HashMap<String, f32> = HashMap::new();

    for result_set in results {
        for (rank, item) in result_set.iter().enumerate() {
            let score = 1.0 / (k + rank as u32);
            *scores.entry(item.id.clone()).or_default() += score;
        }
    }

    let mut ranked: Vec<_> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    ranked.into_iter().map(|(id, score)| ScoredResult { id, score }).collect()
}
```

### API Changes
```
POST /memory/hybrid  (NUEVO)
Body: {
    "query": "string",
    "limit": 10,
    "rrf_k": 60,  // optional, default 60
    "filters": {
        "path": "string",  // optional
        "metadata.category": "string"  // optional
    }
}

Response: {
    "results": [
        {
            "id": "01KP...",
            "content": "...",
            "score": 0.95,
            "source": "vector|keyword|hybrid"  // NUEVO
        }
    ],
    "query_vector": [0.1, ...],  // solo si include_embedding=true
    "total_available": 100
}
```

### Tests
```rust
#[test]
fn test_rrf_fusion() {
    let results = vec![
        vec![ScoredResult { id: "a".into(), score: 1.0 }, ScoredResult { id: "b".into(), score: 0.9 }],
        vec![ScoredResult { id: "b".into(), score: 1.0 }, ScoredResult { id: "c".into(), score: 0.8 }],
    ];
    let fused = reciprocal_rank_fusion(results, 60);
    // "b" debería estar primero (aparece en ambos rankings)
    assert_eq!(fused[0].id, "b");
}
```

---

## Phase 3: Memory Graph 🟡 MEDIUM

**Goal:** Track entity relationships and beliefs

### Changes Required

#### `src/memory/belief_graph.rs` - Expandir existente
```rust
// BeliefGraph ya existe, expandir con:
pub struct Belief {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,  // Puede ser entidad o valor
    pub confidence: Confidence,
    pub provenance: Provenance,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

// Relations para entities
pub struct EntityRelation {
    pub from_entity: String,
    pub to_entity: String,
    pub relation_type: String,  // "knows", "works_at", "part_of"
    pub weight: f32,
    pub last_updated: DateTime<Utc>,
}

// Query graph
pub async fn query_graph(
    State(state): State<AppState>,
    Json(request): Json<GraphQuery>,
) -> Result<Json<GraphResponse>, StatusCode> {
    // 1. Encontrar entity inicial
    let start_entity = state.db.find_entity(&request.entity).await?;

    // 2. BFS/DFS traversal
    let relations = state.graph.traverse(
        start_entity,
        request.max_depth.unwrap_or(2),
        request.relation_types.as_deref(),
    ).await?;

    Ok(Json(GraphResponse { relations }))
}
```

### API Changes
```
POST /memory/graph  (NUEVO)
Body: {
    "entity": "string",
    "max_depth": 2,
    "relation_types": ["knows", "works_at"]  // optional filter
}

Response: {
    "entity": "BELA",
    "relations": [
        {
            "type": "works_at",
            "target": "SWAL",
            "confidence": 0.95
        },
        {
            "type": "knows",
            "target": "Leonardo",
            "confidence": 0.8
        }
    ]
}
```

### Database Schema Changes
```sql
-- entities table
CREATE TABLE entities (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    entity_type TEXT,  -- person, org, concept, event
    created_at TIMESTAMP,
    last_seen TIMESTAMP
);

-- entity_relations table
CREATE TABLE entity_relations (
    id TEXT PRIMARY KEY,
    from_entity TEXT REFERENCES entities(id),
    to_entity TEXT REFERENCES entities(id),
    relation_type TEXT,
    weight REAL DEFAULT 1.0,
    created_at TIMESTAMP,
    UNIQUE(from_entity, to_entity, relation_type)
);

-- belief_graph (expandir)
ALTER TABLE belief_graph ADD COLUMN subject_entity TEXT;
ALTER TABLE belief_graph ADD COLUMN object_entity TEXT;
ALTER TABLE belief_graph ADD COLUMN expires_at TIMESTAMP;
```

---

## Phase 4: Consolidation 🟡 MEDIUM

**Goal:** Background replay of memories to consolidate learning

### Changes Required

#### `src/consolidation/mod.rs` - Nuevo módulo
```rust
pub struct ConsolidationTask {
    interval: Duration,
    replay_batch_size: usize,
}

impl ConsolidationTask {
    pub async fn run(&self, state: &AppState) -> Result<ConsolidationStats> {
        // 1. Seleccionar memories para replay
        let to_replay = state.db.select_memories_for_consolidation(
            self.replay_batch_size
        ).await?;

        let mut stats = ConsolidationStats::default();

        for memory in to_replay {
            // 2. Re-generate embedding
            let new_vector = state.embedder.encode(&memory.content).await?;

            // 3. Comparar con embedding existente
            let similarity = cosine_similarity(&memory.content_vector, &new_vector);

            // 4. Si similarity < threshold, marcar para revisión
            if similarity < 0.85 {
                state.db.mark_for_revision(&memory.id).await?;
                stats.revised += 1;
            } else {
                // Reforzar: actualizar score de importancia
                state.db.increment_importance(&memory.id).await?;
                stats.reinforced += 1;
            }

            // 5. Aplicar decay aImportance score
            state.db.apply_importance_decay(&memory.id).await?;
            stats.processed += 1;
        }

        Ok(stats)
    }
}

pub async fn start_consolidation_scheduler(state: AppState) {
    let task = ConsolidationTask {
        interval: Duration::hours(1),
        replay_batch_size: 50,
    };

    let mut interval = tokio::time::interval(task.interval);
    loop {
        interval.tick().await;
        if let Err(e) = task.run(&state).await {
            tracing::error!("Consolidation failed: {}", e);
        }
    }
}
```

### API Changes
```
POST /memory/consolidate  (NUEVO, también manual trigger)
Body: {
    "batch_size": 50,  // optional
    "force": true  // optional, skip schedule
}

Response: {
    "status": "ok",
    "processed": 50,
    "reinforced": 45,
    "revised": 5
}

GET /memory/consolidation/status  (NUEVO)
Response: {
    "last_run": "2026-04-15T10:00:00Z",
    "total_processed": 1500,
    "next_run": "2026-04-15T11:00:00Z"
}
```

### Cron Job
```json
{
  "name": "xavier-consolidation",
  "schedule": { "kind": "cron", "expr": "0 * * * *" },
  "payload": { "kind": "agentTurn", "message": "Run /memory/consolidate" }
}
```

---

## Phase 5: Reflection 🟢 LOW

**Goal:** Self-analysis of memory patterns

### Changes Required

#### `src/reflection/mod.rs` - Nuevo módulo
```rust
pub struct ReflectionResult {
    pub themes: Vec<String>,
    pub entities: Vec<EntitySummary>,
    pub recent_learning: Vec<String>,
    pub suggestions: Vec<String>,
}

pub async fn reflect(state: &AppState) -> Result<ReflectionResult> {
    // 1. Analyze recent memories (last 24h)
    let recent = state.db.get_recent_memories(Duration::days(1)).await?;

    // 2. Extract entities
    let entities = extract_entities(&recent);

    // 3. Find themes (simple clustering by vector similarity)
    let themes = find_themes(&recent);

    // 4. Identify recent learning
    let learning = identify_learning(&recent);

    // 5. Generate suggestions based on gaps
    let suggestions = generate_suggestions(&entities, &themes);

    Ok(ReflectionResult {
        themes,
        entities,
        recent_learning: learning,
        suggestions,
    })
}

fn identify_learning(memories: &[MemoryDoc]) -> Vec<String> {
    // heuristics:
    // - memories con tags "decision", "learned", "result"
    // - alta importancia + reciente = "learning"
    memories.iter()
        .filter(|m| m.importance > 0.7 && is_recent(&m.created_at))
        .map(|m| m.content.clone())
        .collect()
}
```

### API Changes
```
POST /memory/reflect  (NUEVO)
Body: {
    "time_window": "24h",  // 24h, 7d, 30d
    "include_suggestions": true
}

Response: {
    "themes": ["sales", "project-x", "feedback"],
    "top_entities": [
        {"name": "BELA", "mentions": 45},
        {"name": "Leonardo", "mentions": 12}
    ],
    "recent_learning": [
        "Rodacenter está interesado en ManteniApp",
        "Leonardo maneja las negociaciones en Chile"
    ],
    "suggestions": [
        "Explorar más casos de uso de ManteniApp para manufacturing",
        "Follow-up con Leonardo sobre estado de Rodacenter"
    ]
}
```

---

## Phase 6: Android Sync 🟡 MEDIUM

**Goal:** Synchronize memory between Android APK and server

### Changes Required

#### Tauri App - Nuevo módulo `sync.rs`
```rust
pub struct SyncManager {
    server_url: String,
    ws_client: WebSocket,
    last_sync: DateTime<Utc>,
}

impl SyncManager {
    pub async fn sync(&mut self) -> Result<SyncResult> {
        // 1. Get server state
        let server_state = self.fetch_server_state().await?;

        // 2. Get local changes since last_sync
        let local_changes = self.db.get_changes_since(self.last_sync).await?;

        // 3. Merge with conflict resolution (timestamp-based)
        let merged = self.merge_changes(server_state, local_changes);

        // 4. Apply to local
        self.db.apply_changes(merged).await?;

        self.last_sync = Utc::now();
        Ok(SyncResult { synced: true })
    }

    fn merge_changes(&self, server: MemoryState, local: MemoryState) -> MemoryState {
        // Last-write-wins (timestamp-based)
        // TODO: more sophisticated merge for conflicts
        let mut merged = server;
        for (id, local_mem) in &local.memories {
            if let Some(server_mem) = merged.memories.get(id) {
                if local_mem.updated_at > server_mem.updated_at {
                    merged.memories.insert(id.clone(), local_mem.clone());
                }
            } else {
                merged.memories.insert(id.clone(), local_mem.clone());
            }
        }
        merged
    }
}
```

### WebSocket Protocol
```
// Client → Server
{ "type": "sync_request", "last_sync": "2026-04-15T10:00:00Z" }

// Server → Client
{ "type": "sync_response", "memories": [...], "server_time": "..." }

// Bidirectional sync
{ "type": "memory_update", "memory": {...} }
{ "type": "memory_delete", "id": "..." }
```

---

## Dependencies

```toml
# Core dependencies (ya tienen)
tokio = { version = "1.50", features = ["full"] }
axum = "0.8"
serde = { version = "1.0", features = ["derive"] }
sqlite-vec = "0.1"
parking_lot = "0.12"

# Nuevas dependencies
reqwest = { version = "0.12", features = ["json"] }  # para API calls

# Para embeddings locales (futuro)
# candle-core = "0.5"
# candle-encoders = "0.5"
```

---

## Metrics & Success Criteria

| Metric | Current | Target | How to Measure |
|--------|---------|--------|----------------|
| Latency (simple query) | ~10ms | <50ms | benchmark |
| Latency (hybrid search) | N/A | <100ms | benchmark |
| Recall@10 | ~60% | >90% | retrieval tests |
| Memory footprint | ~50MB | <500MB for 100K docs | benchmark |
| Embedding latency | N/A | <200ms per doc | benchmark |

---

## Implementation Order

1. **Phase 1:** Embeddings (blocking for other phases)
2. **Phase 2:** Reranking (RRF) - depends on Phase 1
3. **Phase 3:** Memory Graph - independent, can parallelize
4. **Phase 4:** Consolidation - depends on Phase 1
5. **Phase 5:** Reflection - depends on Phase 3
6. **Phase 6:** Android Sync - depends on Phase 1-2

---

## Testing Strategy

```bash
# Unit tests por módulo
cargo test --lib

# Integration tests
cargo test --test '*'

# Benchmarks
cargo bench

# Full benchmark suite
./scripts/benchmark_full.sh
```

---

*Document version 1.0 - 2026-04-15*
