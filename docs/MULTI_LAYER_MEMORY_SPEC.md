# Multi-Layer Memory Architecture - xavier v0.5

**Version:** 1.0
**Date:** 2026-04-15
**Target:** LOCOMO >98% (match Jia et al. consistency model)
**Status:** READY FOR IMPLEMENTATION

---

## 🎯 Target Architecture

Based on research from Jia et al. (98% on LOCOMO), Kang et al., and Phadke et al.

```
┌─────────────────────────────────────────────────────────────────┐
│                    Multi-Layer Memory Framework                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│  │   Working     │───▶│  Episodic    │───▶│   Semantic   │      │
│  │   Memory     │    │   Memory     │    │   Memory     │      │
│  │              │    │              │    │              │       │
│  │ • Raw recent │    │ • Session    │    │ • Entities   │      │
│  │ • Bounded   │    │   summaries  │    │ • Principles │      │
│  │ • FIFO      │    │ • Compressed │    │ • Facts      │      │
│  └──────────────┘    └──────────────┘    └──────────────┘       │
│         │                   │                   │                │
│         ▼                   ▼                   ▼                │
│  ┌─────────────────────────────────────────────────────┐       │
│  │              Adaptive Retrieval Gating               │       │
│  │                                                         │       │
│  │  • Relevance scoring per layer                        │       │
│  │  • Dynamic weight allocation                          │       │
│  │  • Cross-layer deduplication                          │       │
│  └─────────────────────────────────────────────────────┘       │
│                            │                                     │
│                            ▼                                     │
│  ┌─────────────────────────────────────────────────────┐       │
│  │           Retention Regularization                   │       │
│  │                                                         │       │
│  │  • Semantic drift prevention                          │       │
│  │  • Consistency verification                           │       │
│  │  • Memory coherence scoring                           │       │
│  └─────────────────────────────────────────────────────┘       │
│                            │                                     │
│                            ▼                                     │
│                    ┌──────────────┐                             │
│                    │   Response   │                             │
│                    │   Generator  │                             │
│                    └──────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

---

## 📋 Implementation Tasks

### Task 1: Working Memory Layer

**File:** `src/memory/working.rs`

```rust
pub struct WorkingMemory {
    capacity: usize,        // Max items (e.g., 100)
    items: Vec<MemoryItem>, // FIFO queue
    access_counts: HashMap<String, u32>,
}

impl WorkingMemory {
    pub fn push(&mut self, item: MemoryItem);
    pub fn get(&self, id: &str) -> Option<&MemoryItem>;
    pub fn evict_oldest(&mut self) -> Option<MemoryItem>;
    pub fn access(&mut self, id: &str); // Update access count
    pub fn search(&self, query: &str) -> Vec<ScoredResult>;
}
```

**Features:**
- Bounded capacity (configurable, default 100)
- FIFO eviction with LRU fallback
- Access frequency tracking
- Fast in-memory search (BM25)

### Task 2: Episodic Memory Layer

**File:** `src/memory/episodic.rs`

```rust
pub struct EpisodicMemory {
    summary_window: usize,   // Turns per summary
    session_store: HashMap<SessionId, SessionSummary>,
    max_sessions: usize,
}

pub struct SessionSummary {
    session_id: SessionId,
    start_time: DateTime,
    summary: String,        // LLM-generated summary
    key_events: Vec<Event>,
    sentiment_timeline: Vec<f32>,
}
```

**Features:**
- Session-based grouping
- LLM-powered summarization
- Key event extraction
- Sentiment tracking per session

### Task 3: Semantic Memory Layer

**File:** `src/memory/semantic.rs`

```rust
pub struct SemanticMemory {
    entities: EntityGraph,
    principles: Vec<Principle>,
    facts: FactStore,
}

pub struct EntityGraph {
    nodes: HashMap<EntityId, Entity>,
    edges: Vec<Relation>,
    embeddings: VecStore,
}

#[derive(Clone)]
pub struct Entity {
    id: EntityId,
    name: String,
    entity_type: EntityType, // Person, Org, Product, Concept
    properties: HashMap<String, Value>,
    trust_score: f32,
    last_updated: DateTime,
}

pub enum EntityType {
    Person,
    Organization,
    Product,
    Concept,
    Location,
    Event,
}
```

**Features:**
- Entity extraction (NER)
- Relationship tracking
- Trust scoring per entity
- Concept hierarchy

### Task 4: Adaptive Retrieval Gating

**File:** `src/retrieval/gating.rs`

```rust
pub struct AdaptiveGating {
    layer_weights: LayerWeights,
    relevance_threshold: f32,
}

#[derive(Clone)]
pub struct LayerWeights {
    working: f32,   // Default 0.3
    episodic: f32, // Default 0.3
    semantic: f32, // Federal    0.4
}

impl AdaptiveGating {
    pub fn retrieve(
        &self,
        working: &[MemoryItem],
        episodic: &[SessionSummary],
        semantic: &[Entity],
        query: &str,
    ) -> Vec<ScoredResult> {
        // 1. Score each layer independently
        // 2. Normalize scores
        // 3. Apply layer weights
        // 4. Fuse with RRF
        // 5. Deduplicate
        // 6. Return top-k
    }
}
```

**Features:**
- Layer-specific relevance scoring
- Dynamic weight adjustment based on query type
- Cross-layer deduplication
- Configurable weights

### Task 5: Retention Regularization

**File:** `src/consistency/regularization.rs`

```rust
pub struct RetentionRegularizer {
    drift_threshold: f32,
    consistency_model: ConsistencyChecker,
}

impl RetentionRegularizer {
    pub fn check_coherence(&self, memories: &[MemoryItem]) -> CoherenceReport {
        // 1. Detect conflicting facts
        // 2. Check entity consistency
        // 3. Verify temporal ordering
        // 4. Score overall coherence
    }

    pub fn detect_drift(&self, old: &Entity, new: &Entity) -> bool {
        // Compare properties for semantic drift
    }
}
```

**Features:**
- Conflict detection
- Entity consistency verification
- Temporal coherence checking
- Drift detection and alerting

### Task 6: Consistency Model (LLM-based)

**File:** `src/consistency/llm_checker.rs`

```rust
pub struct ConsistencyChecker {
    llm_client: LLMClient,
}

impl ConsistencyChecker {
    pub async fn verify(&self, memories: &[MemoryItem]) -> Result<ConsistencyReport> {
        // Use lightweight LLM to verify:
        // 1. No contradictions
        // 2. Facts align with entities
        // 3. Temporal order correct
    }

    pub async fn generate_summary(&self, events: &[Event]) -> Result<String> {
        // LLM-powered session summary
    }
}
```

**Features:**
- Lightweight LLM verification (GPT-4o-mini or local)
- Contradiction detection
- Coherence scoring
- Auto-fix suggestions

---

## 🔌 API Endpoints

### New Endpoints

```
POST /memory/layer/working/add
  → Add to working memory

POST /memory/layer/episodic/summarize
  → Generate session summary

POST /memory/layer/semantic/entity
  → Add/update entity

POST /memory/retrieve (enhanced)
  → Multi-layer retrieval with gating

POST /memory/consistency/check
  → Verify memory coherence

POST /memory/consolidate
  → Run retention regularization

GET /memory/layer/{layer}/stats
  → Get layer statistics
```

### Enhanced Existing

```
POST /memory/add (enhanced)
  → Auto-classify into appropriate layer

POST /memory/search (enhanced)
  → Multi-layer weighted retrieval
```

---

## 📊 Configuration

```toml
[memory.layers]
working_capacity = 100
max_episodic_sessions = 50
entity_trust_threshold = 0.7

[memory.retrieval]
working_weight = 0.3
episodic_weight = 0.3
semantic_weight = 0.4
relevance_threshold = 0.5

[memory.consistency]
drift_threshold = 0.2
llm_check_enabled = true
llm_model = "gpt-4o-mini"
```

---

## 📈 Expected Results

| Metric | Current (v0.4) | Target (v0.5) |
|--------|---------------|---------------|
| LOCOMO Score | ~45% | >98% |
| Single-hop F1 | Low | >0.9 |
| Multi-hop F1 | Low | >0.7 |
| Temporal F1 | Low | >0.6 |
| False Memory Rate | High | <5% |
| Coherence Score | N/A | >0.95 |

---

## 🔄 Implementation Order

1. **Phase 1:** Working Memory + basic retrieval
2. **Phase 2:** Episodic Memory + summarization
3. **Phase 3:** Semantic Memory + entity graph
4. **Phase 4:** Adaptive Gating + RRF
5. **Phase 5:** Retention Regularization
6. **Phase 6:** Consistency Model (LLM)

---

## 📝 References

- Jia et al. (2024) - "Consistency model achieving 98% LOCOMO"
- Kang et al. (2024) - "Three-tier Memory Operating System"
- Phadke et al. (2024) - "Four-tier hierarchical memory with truth verification"
- Maharana et al. (2024) - "LOCOMO benchmark construction"

---

*Spec created: 2026-04-15*
*Ready for implementation*
