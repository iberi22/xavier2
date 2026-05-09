# XAVIER Usage Guide for SWAL Agents

**Purpose:** Standardized, token-efficient memory operations for all SWAL agents.
**Principle:** Xavier is the shared brain — use it wisely, don't waste tokens.

---

## Backend Architecture

### Default Backend: `vec` (SQLite-pro)

The default memory backend is **`XAVIER_MEMORY_BACKEND=vec`**, which combines:

- **SQLite** — persistent storage with WAL mode (mmap 256MB, cache 32MB)
- **sqlite-vec** — vector search via KNN for semantic similarity
- **FTS5** — full-text search with BM25 ranking
- **Knowledge Graph** — entities + relations tables for structured data
- **Hash Chain** — tamper-evident `memory_chain` table for audit trail

### Search Architecture

```
Query → FTS5 (keyword) → RRF Fusion ← Vector Search (sqlite-vec KNN)
                       ← Knowledge Graph (entity traversal)
```

1. **FTS5** — keyword/bm25 search for exact matches
2. **Vector Search** — semantic similarity via sqlite-vec KNN
3. **Knowledge Graph** — entity/relation traversal for connected facts
4. **RRF** (Reciprocal Rank Fusion) — fuses all three into ranked results

### Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `XAVIER_MEMORY_BACKEND` | `vec` | Backend: `vec`, `surreal`, `sqlite`, `file` |
| `XAVIER_EMBEDDING_DIMENSIONS` | `768` | Embedding vector size (adjust to match your embedder) |

---

## Backend Comparison

| Backend | Best For | Dependencies |
|---------|---------|-------------|
| `vec` (default) | General purpose, fastest | None |
| `surreal` | High concurrency, distributed | SurrealDB container |
| `sqlite` | Simple deployments | None |
| `file` | Development | None |

---

## Core Rules

### 1. READ Before WRITE
Always check if the information already exists in Xavier before adding it.

### 2. WRITE Summaries, Not Transcripts
Don't dump full conversations. Write concise, factual summaries.

### 3. Use Specific Queries
Don't search "everything about X" — ask specific questions.

### 4. Batch Related Reads
If you need 3 things about the same topic, one query is better than 3.

---

## API Reference

### Base URL
```
http://localhost:8003
Header: X-Xavier-Token: dev-token
```

### READ Operations

**Search memories (semantic + keyword + KG):**
```powershell
POST /memory/search
Body: { "query": "specific question", "limit": 5 }
```

**Get single memory by path:**
```powershell
GET /memory/{workspace_id}/{path}
```

**Get memory stats:**
```powershell
GET /memory/stats
```

### WRITE Operations

**Add memory:**
```powershell
POST /memory/add
Body: {
  "content": "Concise factual summary. Max 2KB.",
  "metadata": {
    "source": "agent-name",
    "priority": "high|medium|low",
    "category": "technical|client|operations|sales"
  }
}
```

**Add entity (Knowledge Graph):**
```powershell
POST /memory/entity
Body: {
  "name": "entity-name",
  "type": "person|project|concept|...",
  "properties": { "key": "value" }
}
```

**Add relation (Knowledge Graph):**
```powershell
POST /memory/relation
Body: {
  "from": "entity-a",
  "to": "entity-b",
  "relation": "related_to|owns|depends_on|..."
}
```

### MANAGEMENT Operations

**Auto-curation (decay + consolidate + evict):**
```powershell
POST /memory/decay    # Apply time-based decay
POST /memory/consolidate  # Merge duplicates
DELETE /memory/evict?threshold=0.2  # Remove low-quality
```

---

## Token-Saving Patterns

### DO: Efficient Reads

```powershell
# GOOD: One specific query
$r = POST /memory/search -Body @{ query = "Xavier version status" }
# Returns: 5 most relevant memories, ~500 tokens

# BAD: Vague query
$r = POST /memory/search -Body @{ query = "everything about projects" }
# Returns: Random memories, wastes tokens
```

### DO: Write Summaries

```powershell
# GOOD: 200 byte summary
Add: "Xavier v0.4.1 running. pplx-embed healthy. Benchmark 99.1% recall."

# BAD: 5KB transcript dump
Add: Full conversation transcript with all filler words
```

### DO: Use Metadata

```powershell
# GOOD: Categorized and prioritized
Add: { content: "...", metadata: @{ source="agent"; priority="high"; category="technical" } }

# BAD: No metadata
Add: { content: "..." }
```

### DO: Check Before Adding

```powershell
# Before adding:
$existing = POST /memory/search -Body @{ query = "your exact fact" }
if ($existing.results[0].content.Contains("your fact")) {
    # Already exists, skip
} else {
    # Add new
}
```

---

## When to Write to Xavier

### ALWAYS Write:
- Decisions made (with reasoning)
- Client requirements or preferences
- Technical architecture choices
- Project status changes
- Results of analysis or research
- Knowledge Graph entities and relations for structured context

### NEVER Write:
- Intermediate thinking (keep in local context)
- Full conversation dumps
- Information that will be stale in hours
- Duplicate information already in Xavier

---

## Category Guidelines

| Category | Examples | Priority |
|----------|----------|----------|
| `technical` | Architecture, bugs, fixes, code decisions | High |
| `client` | BELA profile, Leonardo/Rodacenter, requirements | High |
| `operations` | Cron jobs, monitoring, security | Medium |
| `sales` | ManteniApp pricing, demos, prospects | Medium |
| `ephemeral` | Temporary calculations, one-time analysis | Low |

---

## Memory Quality Rules

**High Quality Memory:**
- Factual and verifiable
- Specific (not vague)
- Includes source and timestamp
- Uses correct category/priority

**Low Quality Memory (will be evicted):**
- Vague: "things are going well"
- Duplicate: same fact added multiple times
- Stale: old info never accessed
- Huge: chunks >4KB without structure

---

## Sync Strategy

**OpenClaw Agent Sessions → Xavier:**
- Session summaries (not full transcripts)
- Decisions and outcomes
- Client-facing information

**Frequency:**
- Sync cron: every 1 hour
- Auto-curation: daily at 6 AM
- Manual sync: after major session

---

## Example: Agent Workflow

```
1. START SESSION
   → Read Xavier: "BELA profile, active projects, recent decisions"
   → KG traverse: "What did Alice decide recently?"

2. DURING SESSION
   → Read Xavier for specific facts as needed
   → Write decisions and outcomes immediately
   → Add KG entities/relations for structural decisions

3. END SESSION
   → Write summary: "Completed X, decided Y, next steps Z"
   → Update project status if changed

4. XAVIER SYNC (automatic via cron)
   → Sessions sync'd hourly
   → Memories auto-curated daily
```

---

## Memory Size Limits

| Type | Max Size | Recommendation |
|------|----------|----------------|
| Single memory | 4KB | Keep under 2KB |
| Search results | 5 items | Use limit=5 |
| Session summary | 500 bytes | Be concise |

---

## Troubleshooting

**Memory not found in search:**
1. Check if it was added (GET /memory/stats)
2. Try different phrasing (semantic search varies)
3. Add with more specific content

**Too many duplicates:**
- Run manual consolidation:
```powershell
POST /memory/consolidate -Body @{ dry_run = false }
```

**Low quality memories:**
- Check: GET /memory/quality?threshold=0.3
- Evict: DELETE /memory/evict?threshold=0.25

**KG traversal returns nothing:**
1. Verify entity exists: POST /memory/entity/search -Body @{ "query": "entity-name" }
2. Check relation spelling matches what was stored
3. Try shallower hops (hops=1 first)

---

## Knowledge Graph Operations

The Knowledge Graph (KG) stores structured entity/relationship data for rich memory context.

### What It Is
- **Entity**: A named node (person, project, decision, concept)
- **Relation**: A typed edge between entities with optional properties
- **Traversal**: Query paths through the graph to answer multi-hop questions

### When to Use KG vs. Vector Search

| Use Case | Best Choice |
|----------|-------------|
| "What does BELA prefer for auth?" | Vector search (`/memory/search`) |
| "What did Alice decide about auth last week?" | KG traverse (`/memory/kg/traverse`) |
| "Find all decisions about project X" | KG + vector combined |
| "Who made the call on this architecture?" | KG traverse by person entity |

### API Functions

**`add_entity(name, type, properties)`** — Register a new entity
```powershell
POST /memory/entity
Body: { "name": "AuthWorkingGroup", "type": "team", "properties": { "formed": "2026-03" } }
```

**`add_relation(from, relation, to, properties)`** — Create a relationship
```powershell
POST /memory/relation
Body: { "from": "Alice", "relation": "decided", "to": "use OAuth2", "properties": { "date": "2026-04-10" } }
```

**`kg_traverse(start, hops, relation_filter)`** — Traverse the graph
```powershell
POST /memory/kg/traverse
Body: { "start": "Alice", "hops": 2, "relation_filter": ["decided", "owns"] }
```
Returns all entities reachable from `Alice` via the specified relations, up to N hops.

**Search entities:**
```powershell
POST /memory/entity/search
Body: { "query": "project-name", "type": "project" }
```

### Example: Decision Tracking

```
1. AGENT MAKES A DECISION
   → POST /memory/entity  { "name": "Alice", "type": "agent" }
   → POST /memory/relation { "from": "Alice", "relation": "decided", "to": "JWT auth", "properties": { "date": "2026-04-10" } }
   → POST /memory/add     { "content": "Alice decided to use JWT for auth", "metadata": {...} }

2. LATER QUERY: "What did Alice decide about auth?"
   → POST /memory/kg/traverse { "start": "Alice", "hops": 2, "relation_filter": ["decided"] }
   → Returns: [ { "to": "JWT auth", "properties": { "date": "2026-04-10" } } ]

3. VECTOR SEARCH FALLBACK
   → POST /memory/search { "query": "Alice auth decision JWT" }
   → Returns semantic matches from memory store
```

---

*Last updated: 2026-04-13*
