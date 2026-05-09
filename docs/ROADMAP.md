# Xavier Roadmap - State of the Art Memory for AI Agents
Generated: 2026-04-20
Purpose: Features analysis and implementation roadmap for Xavier v1.0

---

## Executive Summary

Xavier currently has a solid foundation (vector storage, FTS5, security) but lacks
several key features that define state-of-the-art memory systems for LLM agents.

**Key Gap Analysis:**
- Missing: Memory tiers, structured memories, memory importance/recency scoring
- Missing: MCP-native structured memory operations (mem_save with typed fields)
- Missing: Context summarization and memory compression
- Missing: Agent reflection and self-improvement mechanisms

---

## Part 1: Competitive Analysis

### 1.1 Engram (Go, ~2MB binary) - Reference Architecture
**Why it's successful:**
- Zero dependencies (single binary, SQLite)
- MCP-first design
- Simple mem_save API with structured fields

**Key Features:**
```
mem_save {
  title: string,        // Quick identifier
  type: string,         // "learning", "decision", "bug", "pattern"
  what: string,         // What happened
  why: string,         // Why it matters
  where: string,       // Where it applies
  learned: string      // Key takeaway
}
```

**What's missing:** No vector search, no memory tiers, basic FTS5 only.

---

### 1.2 Letta/MemGPT (State of Art) - Memory Architecture
**Key Innovations:**

1. **Memory Tiers**
   - **Recall Memory**: Fast, small, recent context
   - **Archival Memory**: Larger, slower, deep storage
   - System auto-manages what goes where

2. **Memory Blocks**
   - Editable, typed memory sections
   - Human-readable format
   - Agent can read/write directly

3. **External Memory Summary**
   - System auto-generates summaries
   - Token budget management
   - Context window optimization

4. **Stateful Runtime**
   - Agent maintains state across sessions
   - Self-improving through reflection
   - Personality persistence

---

### 1.3 Mem0 - Framework Agnostic
**Key Features:**
- SDK for LangChain, CrewAI, AutoGen, Custom
- User preferences storage
- Multi-agent memory sharing
- Persona management

---

## Part 2: Xavier Gap Analysis

### Current Implementation:

| Feature | Status | Quality |
|---------|--------|---------|
| Vector storage | ? sqlite-vec | Good |
| Keyword search | ? FTS5 | Good |
| Security | ? PromptInjectionDetector | Excellent |
| Code indexing | ? code-graph | Good |
| HTTP API | ? Axum | Good |
| CLI | ? Basic | Needs work |
| MCP mode | ? Stub | Needs completion |

### Missing Features (Priority Order):

#### P0 - Critical for v1.0

1. **Structured Memory Types**
   - Define: `memory_type` enum (episodic, semantic, procedural, declarative)
   - Each type has different retention/retrieval rules

2. **Memory Importance Scoring**
   - Auto-score based on: recency, access frequency, novelty, user feedback
   - Priority queue for memory consolidation

3. **Memory Consolidation/Summarization**
   - Auto-compress old memories when storage budget exceeded
   - Keep key facts, discard redundant details

4. **Enhanced mem_save CLI**
   - Structured input: `xavier save --type learning --what "..." --why "..."`
   - Auto-classify content type

5. **Memory Search by Type**
   - `xavier search "query" --type episodic --limit 10`
   - Filter by date range, importance score

#### P1 - Important for differentiation

6. **Memory Graph/Relationships**
   - Track which memories relate to each other
   - Memory A "relates_to" Memory B

7. **Memory TTL/Auto-expiry**
   - Configurable retention per memory type
   - "Temporary facts" that expire

8. **Memory Tags/Categories**
   - Flat tag system for organization
   - Cross-reference memories by tag

9. **Memory Versioning**
   - Track changes to important memories
   - Audit trail for decisions

10. **Memory Analytics**
    - `xavier stats --insights`
    - Show memory usage patterns, gaps

#### P2 - Nice to have

11. **Memory Import/Export**
    - JSON/CSV export for backup
    - Import from Engram, Mem0

12. **Multi-workspace**
    - `xavier switch-workspace <name>`
    - Isolated memory per project

13. **Memory Sharing (Enterprise)**
    - Share memories between agents
    - Team knowledge base

---

## Part 3: Implementation Roadmap

### Phase 1: Quick Wins (This Week)

#### 3.1.1 Enhanced Memory Types
```rust
// Add to MemoryDocument.metadata
pub enum MemoryType {
    Episodic,    // Events, conversations
    Semantic,    // Facts, knowledge
    Procedural,  // How-to, processes
    Declarative, // Preferences, settings
}
```

**File:** `src/domain/memory/types.rs`
**Effort:** Low (metadata field)

#### 3.1.2 Memory Importance Score
```rust
// Add importance field, computed from:
// - Access frequency (from logs)
// - Recency (decay function)
// - Content novelty (embedding similarity)
// - User annotations

pub struct MemoryScore {
    total: f32,
    recency_component: f32,
    importance_component: f32,
    novelty_component: f32,
}
```

**File:** `src/memory/scorer.rs` (new)
**Effort:** Medium

#### 3.1.3 Enhanced CLI Commands
```
xavier save <content> [--type episodic|semantic|procedural|declarative]
xavier search <query> [--type X] [--limit N] [--min-score 0.5]
xavier recall [--recent 10] [--important] [--type X]
xavier forget <memory-id>  # Soft delete
xavier archive <memory-id> # Move to archival
```

**File:** `src/cli.rs` + `src/commands/` (new dir)
**Effort:** Medium

---

### Phase 2: Memory Architecture (Next Week)

#### 3.2.1 Memory Tier System
```
Working Memory (RAM-like, fast)
+-- Recent context (last N memories, < 1 day)
+-- Active focus (current task relevant)
+-- High-importance (score > 0.8)

Archival Memory (disk-like, slow)
+-- Historical (1-30 days old)
+-- Semantic knowledge (stable facts)
+-- Procedural memory (how-to, patterns)
+-- Archived (low importance, old)
```

**File:** `src/memory/tier_manager.rs` (new)
**Logic:** Auto-move between tiers based on score + age

#### 3.2.2 Memory Consolidation
```rust
pub async fn consolidate(&self, budget_bytes: u64) -> Result<()> {
    // 1. Get all memories sorted by score
    // 2. If total_size > budget:
    //    - Summarize low-importance memories
    //    - Merge similar memories
    //    - Delete very low importance
    // 3. Return new storage size
}
```

**File:** `src/memory/consolidator.rs` (new)
**Trigger:** On add, if storage > threshold

#### 3.2.3 Memory Summarization (LLM-powered)
```rust
pub async fn summarize(memory: &MemoryDocument) -> String {
    // Use LLM to compress memory content
    // Keep: facts, decisions, key learnings
    // Discard: context, redundant info
}
```

**Note:** Requires LLM integration (use existing minimax config)

---

### Phase 3: Advanced Features (Week 3-4)

#### 3.3.1 Memory Relationships
```rust
pub struct MemoryRelation {
    from: String,  // memory ID
    to: String,    // memory ID
    relation_type: String, // "relates_to", "??", "similar_to"
    confidence: f32,
}
```

**File:** `src/memory/relations.rs` (new)

#### 3.3.2 Memory Reflection (Self-improvement)
```rust
// Agent calls this periodically
pub async fn reflect(&self) -> Vec<ReflectionResult> {
    // 1. Analyze recent memories
    // 2. Identify patterns
    // 3. Generate insights
    // 4. Store as new memories with type="insight"
}
```

**File:** `src/agents/reflection.rs` (extend existing)

#### 3.3.3 Context Window Optimization
```rust
pub struct ContextSummary {
    summary_text: String,
    memory_ids: Vec<String>,
    token_budget: usize,
}

// Auto-generate when context window near limit
pub async fn auto_summarize(&self, available_tokens: usize) -> ContextSummary
```

**File:** `src/memory/context_summarizer.rs` (new)

---

## Part 4: Technical Debt / Refactoring

### Quick Fixes Needed:

1. **code_find_handler returns 0 results**
   - Investigate: index format vs query format mismatch
   - Priority: High (blocks code search feature)

2. **Embeddings not active**
   - Binary not using Ollama embedder
   - Need: rebuild with env vars or fix embedder initialization

3. **MCP mode incomplete**
   - Handler exists but may not implement full MCP spec
   - Priority: Medium (for Claude Code integration)

### Refactoring Opportunities:

1. **DTOs scattered in cli.rs**
   - Move to `src/adapters/inbound/dto.rs`

2. **Duplicate error handling**
   - Create `axum::Json` error helper macro

3. **Large match statements in handlers**
   - Extract to separate command objects

---

## Part 5: Feature Priority Matrix

| Feature | Complexity | Impact | Priority | ETA |
|---------|------------|--------|----------|-----|
| Memory types (metadata) | Low | High | P0 | Today |
| Importance scoring | Medium | High | P0 | 2 days |
| Enhanced CLI | Medium | High | P0 | 3 days |
| Memory tier system | High | Very High | P1 | 1 week |
| Memory consolidation | High | Very High | P1 | 1 week |
| Memory summarization | High | Very High | P1 | 1 week |
| Memory relationships | Medium | Medium | P2 | 2 weeks |
| MCP completion | Medium | High | P1 | 3 days |
| Memory reflection | High | Medium | P2 | 2 weeks |
| Context optimization | High | Very High | P1 | 1 week |

---

## Part 6: Competitive Advantage

### Xavier's Strengths:
1. **Security-first**: Only solution with PromptInjectionDetector integrated
2. **Code intelligence**: Unique code-graph indexing and search
3. **Performance**: Local Ollama embeddings, fast FTS5
4. **Dual license**: Open core (MIT) + Enterprise (dual license)

### How to Differentiate:
1. **"Most Secure Memory System"**: Marketing angle
   - Blocks prompt injection attacks
   - Sanitizes inputs/outputs
   - Audit logging ready

2. **"Memory for Code Agents"**: Specialized focus
   - Code search as first-class feature
   - Works with Claude Code, Codex natively
   - Indexed code context in memories

3. **"Production-Ready Open Core"**: Reliability
   - Stable storage (sqlite-vec)
   - Easy deployment (single binary)
   - Clear upgrade path to Enterprise

---

## Part 7: Recommended First Implementation

### Day 1-2: Quick Wins
1. Add `memory_type` field to all new memories (auto-detect from content)
2. Add importance score (simple: recency + access_count)
3. Enhance CLI: `xavier save`, `xavier recall`

### Day 3-4: Fix Critical Bugs
1. Debug code_find_handler returning 0
2. Activate Ollama embeddings

### Day 5-7: Memory Architecture
1. Implement memory tier system
2. Add consolidation trigger
3. Basic summarization (LLM-powered)

### Week 2: MCP + Security
1. Complete MCP handler
2. Add memory encryption at rest (enterprise)
3. Audit logging

---

## Appendix: Reference Implementations

### Engram mem_save Pattern:
```json
{
  "title": "Use ptr::offset_from for pointer diff",
  "type": "learning",
  "what": "Discovered ptr::offset_from is UB for non-equivalent pointers",
  "why": "Compiler optimizations can break code relying on it",
  "where": "src/utils/pointer.rs:45",
  "learned": "Always use ptr_sub() from std::ptr"
}
```

### Letta Memory Block:
```json
{
  "block_name": "persona",
  "block_type": "persona",
  "contents": "You are a helpful coding assistant...",
  "id": "blk_123",
  "created_at": "2026-04-20T00:00:00Z",
  "updated_at": "2026-04-20T00:00:00Z"
}
```

### Mem0 Entity Memory:
```json
{
  "user_id": "user_123",
  "entity_name": "user_preferences",
  "memory": {
    "language": "Spanish",
    "timezone": "America/Bogota",
    "preferred_response_style": "concise"
  }
}
```

---

*Document version: 1.0*
*Next update: After Phase 1 implementation*
