# Xavier HTTP API Reference

Base URL: `http://localhost:8006` (configurable via `XAVIER_HOST` and `XAVIER_PORT`)

Authentication: All endpoints require `X-Xavier-Token` header with the value of `XAVIER_TOKEN` environment variable.

## Health

```
GET /health
```

Returns server status and version.

**Response:**
```json
{
  "status": "ok",
  "version": "0.6.0-beta",
  "uptime_seconds": 3600
}
```

---

## Add Memory

```
POST /memory/add
```

Store a new memory with content and metadata.

**Request:**
```json
{
  "content": "The user configured the payment module for Stripe integration.",
  "metadata": {
    "kind": "episodic",
    "title": "Payment module config"
  }
}
```

**Response:**
```json
{
  "id": "mem_abc123",
  "path": "default/user/session/abc",
  "timestamp": "2026-05-06T12:00:00Z"
}
```

**Metadata fields:**
| Field | Type | Description |
|-------|------|-------------|
| `kind` | string | Memory type: `episodic`, `semantic`, `procedural`, `fact`, `decision`, `belief`, etc. |
| `title` | string | Optional human-readable title |
| `evidence_kind` | string | Source evidence type |
| `namespace` | object | Organizational scope (org, workspace, user, agent, session) |
| `provenance` | object | Source attribution (source_app, repo_url, file_path, etc.) |

---

## Search Memories

```
POST /memory/search
```

Keyword-based search across stored memories.

**Request:**
```json
{
  "query": "payment module Stripe",
  "limit": 10
}
```

**Response:**
```json
{
  "results": [
    {
      "id": "mem_abc123",
      "content": "The user configured the payment module for Stripe integration.",
      "metadata": {
        "kind": "episodic",
        "title": "Payment module config"
      },
      "score": 0.85
    }
  ],
  "total": 1
}
```

---

## Query Memories (AI-powered)

```
POST /memory/query
```

Contextual query with hybrid search — combines keyword, semantic vector, and knowledge graph signals using Reciprocal Rank Fusion (RRF).

**Request:**
```json
{
  "query": "How was payment configured?",
  "limit": 10,
  "search_type": "hybrid"
}
```

**Search types:** `keyword`, `semantic`, `hybrid` (default), `kg`

---

## Hybrid Search

```
POST /memory/hybrid
```

Explicit hybrid search with configurable RRF parameters.

**Request:**
```json
{
  "query": "payment module",
  "limit": 10,
  "rrf_k": 60
}
```

---

## Consolidate Memories

```
POST /memory/consolidate
```

Trigger memory consolidation — deduplicates, summarizes, and archives old or low-importance memories.

---

## Reflect

```
POST /memory/reflect
```

Analyze stored memories for patterns, insights, and connections.

---

## Stats

```
GET /memory/stats
```

Returns memory usage statistics.

**Response:**
```json
{
  "total_memories": 1250,
  "total_entities": 340,
  "storage_bytes": 52428800,
  "by_kind": {
    "episodic": 450,
    "semantic": 300,
    "procedural": 200,
    "fact": 180,
    "decision": 120
  }
}
```

---

## Memory Graph

```
GET /memory/graph?entity=<id>
POST /memory/graph/hops
```

Retrieve entity relationships and traverse the memory graph via BFS.

---

## Error Responses

All endpoints return errors in a consistent format:

```json
{
  "status": "error",
  "message": "Description of what went wrong"
}
```

Common HTTP status codes:
- `200` — Success
- `400` — Bad request (invalid parameters)
- `401` — Unauthorized (missing or invalid token)
- `404` — Not found
- `500` — Internal server error
