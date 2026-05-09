# Xavier Quick Reference

**Base:** `http://localhost:8003` | **Token:** `dev-token`

## Backends

| Backend | Description |
|---------|-------------|
| `vec` (default) | SQLite + sqlite-vec + FTS5 + KG + hash chain |
| `surreal` | SurrealDB (high concurrency, distributed) |
| `sqlite` | Simple SQLite |
| `file` | Development |

**Config:** `XAVIER_MEMORY_BACKEND=vec` | `XAVIER_EMBEDDING_DIMENSIONS=768`

## Search Architecture

```
Query → FTS5 (keyword) → RRF Fusion ← Vector Search (sqlite-vec KNN)
                       ← Knowledge Graph (entity traversal)
```

## Read
```powershell
POST /memory/search -Body @{query="specific question"; limit=5}
GET /memory/stats
```

## Write
```powershell
POST /memory/add -Body @{
  content="Factual summary. Max 2KB."
  metadata=@{source="agent"; priority="high"; category="technical"}
}

# Knowledge Graph
POST /memory/entity -Body @{name="X"; type="project"; properties=@{}}
POST /memory/relation -Body @{from="A"; to="B"; relation="related_to"}
POST /memory/kg/traverse -Body @{start="X"; hops=2}
```

## Rules
1. **READ before WRITE** — check if info exists
2. **WRITE summaries** — not transcripts (max 2KB)
3. **BE SPECIFIC** — vague queries waste tokens
4. **USE metadata** — always add source, priority, category

## Categories
`technical` | `client` | `operations` | `sales` | `ephemeral`

## Quality
- ✅ Factual, specific, with source
- ❌ Vague, duplicate, >4KB, stale

## Sync (automatic)
- Sessions → Xavier: every 1 hour (cron)
- Auto-curation: daily 6 AM

## Management
```powershell
POST /memory/decay
POST /memory/consolidate
DELETE /memory/evict?threshold=0.2
```

---

*Last updated: 2026-04-13*
