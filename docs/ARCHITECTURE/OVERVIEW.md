# Visión General de Arquitectura

**Proyecto:** Xavier
**Fecha:** 2026-03-13 (actualizado: 2026-04-13)

---

## Arquitectura

*Diagrama de arquitectura por definir*

## Componentes

| Componente | Responsabilidad | Tecnología |
|------------|-----------------|------------|
| *Frontend* | Interfaz de usuario | *TBD* |
| *Backend* | Lógica de negocio | *TBD* |
| *Database* | Almacenamiento | *TBD* |

## Flujo de Datos

*Diagrama de flujo por definir*

---

## Memory Backend: sqlite-pro (vec)

Xavier uses **sqlite-pro** with the `sqlite-vec` extension as its primary memory store, providing a unified backend that combines full-text search, vector similarity, and knowledge graph traversal.

### Search Pipeline

```
search(query)
  ├── FTS5 (keyword/exact match) → BM25 ranking
  ├── Vector Search (sqlite-vec) → KNN distance
  ├── Knowledge Graph (entities) → recursive CTE traversal
  └── RRF Fusion → 1/(60+rank) per signal → fused score
```

**Fusion:** Reciprocal Rank Fusion (RRF) combines results from all three retrieval signals using the formula `score = 1 / (60 + rank)`, ensuring balanced contribution from keyword, vector, and graph-based retrieval.

### SQLite Tables

| Table | Type | Purpose |
|-------|------|---------|
| `memory_fts` | FTS5 virtual table | Full-text search with BM25 ranking for keyword/exact match queries |
| `entities` | Regular table | Knowledge graph nodes — entity identifiers, types, metadata |
| `relations` | Regular table | Knowledge graph edges — source, target, relationship type, properties |
| `memory_chain` | Regular table | Tamper-evident hash chain — each entry references the previous hash via SHA-256 |

### WAL Optimization

SQLite is configured for high-throughput concurrent access:

```sql
PRAGMA journal_mode = WAL;
PRAGMA mmap_size = 268435456;      -- 256MB
PRAGMA wal_autocheckpoint = 1000;  -- checkpoint every 1000 pages
PRAGMA cache_size = -32768;        -- 32MB negative = pages, not bytes
```

This configuration (identical to prior SurrealDB optimization notes) provides:
- **WAL mode:** Concurrent readers without writer blocking
- **mmap_size=256MB:** Memory-mapped I/O for faster reads
- **wal_autocheckpoint=1000:** Automatic WAL compaction at 1MB threshold
- **cache_size=-32768:** 32MB page cache (32768 × 1KB pages)

---

*Actualizado: 2026-04-13*
