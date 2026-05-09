# Storage Switch Guide

Switching Xavier between file, SurrealDB, SQLite, and Vec backends — including migration and rollback procedures.

## Overview

Xavier supports four storage backends controlled by `XAVIER_MEMORY_BACKEND`:

| Backend | `XAVIER_MEMORY_BACKEND` value | Use Case |
|---------|-------------------------------|----------|
| **Vec** | `vec` | **Recommended default** — fastest, no external dependencies |
| SurrealDB | `surreal` | High-concurrency distributed setups |
| SQLite | `sqlite` | Legacy fallback |
| File | `file` | Development only |

### Backend Priority

1. **`vec`** — Fastest option. Embedding-based retrieval with FTS5, RRF ranking, Knowledge Graph traversal, and Hash Chain verification built in. No external dependencies beyond the local SQLite file. Median retrieval ~22ms with vector search.
2. **`surreal`** — Full distributed database with schema sync and multi-writer capabilities. Higher operational overhead due to network latency and schema synchronization. Use when you need multi-node high-concurrency writes.
3. **`sqlite`** — Legacy single-file backend with in-memory text filtering. O(n) performance on large datasets. Falls back to file-based indexing when FTS is unavailable.
4. **`file`** — Pure JSON/manifest storage. Useful for local development and debugging. Not recommended for production.

---

## Vec Backend (Recommended)

The `vec` backend uses SQLite with Pro extensions (FTS5, RRF, Knowledge Graph, Hash Chain) to deliver fast embedding-based retrieval.

### Features

- **FTS5** — Full-text search across memory content
- **RRF (Reciprocal Rank Fusion)** — Combines multiple ranking signals for better relevance
- **Knowledge Graph** — Entity relationships and traversal queries
- **Hash Chain** — Content integrity verification

### Environment Variables

```env
# Backend selection
XAVIER_MEMORY_BACKEND=vec

# Vec backend
XAVIER_MEMORY_VEC_PATH=./data/workspaces/default/vec-store.sqlite3

# Embedding configuration
XAVIER_EMBEDDING_DIMENSIONS=768   # Default: 768. Change only if your embedding model uses a different size.
```

### Getting Started

1. **Back up existing data** (optional but recommended)
   ```bash
   cp -r data/workspaces data/workspaces.backup.$(date +%Y%m%d)
   ```

2. **Update `.env`**
   ```env
   XAVIER_MEMORY_BACKEND=vec
   XAVIER_MEMORY_VEC_PATH=./data/workspaces/default/vec-store.sqlite3
   XAVIER_EMBEDDING_DIMENSIONS=768
   ```

3. **Start Xavier**
   ```bash
   docker compose up -d xavier pplx-embed
   ```

4. **Verify**
   ```bash
   curl http://localhost:8003/build | jq .memory_store.selected_backend
   curl http://localhost:8003/readiness | jq .
   ```

---

## Switching: File → SurrealDB (Production)

Use for production with multi-workspace or high-volume distributed writes.

### Steps

1. **Back up existing data**
   ```bash
   cp -r data/workspaces data/workspaces.backup.$(date +%Y%m%d)
   ```

2. **Start SurrealDB** (if not already running):
   ```bash
   docker compose --profile surreal up -d surrealdb
   ```

3. **Migrate data**
   ```bash
   python scripts/migrate_file_to_surreal.py --workspace default --reinstall
   ```
   Use `--all-workspaces` to migrate all workspaces.

4. **Update `.env`**
   ```env
   XAVIER_MEMORY_BACKEND=surreal
   XAVIER_SURREALDB_URL=ws://localhost:8000
   XAVIER_SURREALDB_USER=your-user
   XAVIER_SURREALDB_PASS=your-password
   ```

5. **Restart Xavier**
   ```bash
   docker compose up -d xavier
   ```

6. **Verify**
   ```bash
   curl http://localhost:8003/build | jq .memory_store.selected_backend
   curl http://localhost:8003/readiness | jq .
   ```

---

## Switching: SurrealDB → SQLite (Emergency Fallback)

Use when SurrealDB is unavailable or for a lightweight fallback.

### Steps

1. **Stop Xavier**
   ```bash
   docker compose stop xavier
   ```

2. **Dump SurrealDB data** (optional, recommended)
   ```bash
   # Using the surreal-cli if available, or REST API
   curl -X POST http://localhost:8000/sql \
     -H "NS: xavier" -H "DB: memory" \
     -d '{"sql":"SELECT * FROM memory_records"}' \
     --user root:your-password > surreal_dump.json
   ```

3. **Update `.env`**
   ```env
   XAVIER_MEMORY_BACKEND=sqlite
   XAVIER_MEMORY_SQLITE_PATH=./data/workspaces/default/memory-store.sqlite3
   ```

4. **Migrate from JSON dump if needed**
   ```bash
   python scripts/migrate_file_to_sqlite.py --workspace default --reinstall
   ```

5. **Start Xavier**
   ```bash
   docker compose up -d xavier
   ```

6. **Verify**
   ```bash
   curl http://localhost:8003/build | jq .memory_store.selected_backend
   ls -lh ./data/workspaces/default/memory-store.sqlite3
   ```

---

## Switching: File → SQLite

A lightweight alternative without the SurrealDB overhead.

### Steps

1. **Stop Xavier**
   ```bash
   docker compose stop xavier
   ```

2. **Migrate data**
   ```bash
   python scripts/migrate_file_to_sqlite.py --workspace default --reinstall
   ```

3. **Update `.env`**
   ```env
   XAVIER_MEMORY_BACKEND=sqlite
   XAVIER_MEMORY_SQLITE_PATH=./data/workspaces/default/memory-store.sqlite3
   ```

4. **Start Xavier**
   ```bash
   docker compose up -d xavier
   ```

5. **Verify**
   ```bash
   curl http://localhost:8003/build | jq .memory_store.selected_backend
   ```

---

## Switching: SurrealDB → File

### Steps

1. **Stop Xavier**
   ```bash
   docker compose stop xavier
   ```

2. **Update `.env`**
   ```env
   XAVIER_MEMORY_BACKEND=file
   XAVIER_WORKSPACE_DIR=./data/workspaces
   ```

3. **Restart Xavier** (data stays in SurrealDB; new writes go to file)
   ```bash
   docker compose up -d xavier
   ```

> **Note:** Switching back to file will not automatically sync SurrealDB data into the file. To preserve SurrealDB data, run the migration first.

---

## Switching: SQLite → SurrealDB

### Steps

1. **Stop Xavier**
   ```bash
   docker compose stop xavier
   ```

2. **Start SurrealDB**
   ```bash
   docker compose --profile surreal up -d surrealdb
   ```

3. **Migrate SQLite → SurrealDB**
   ```bash
   python scripts/migrate_file_to_surreal.py --workspace default --reinstall
   ```

4. **Update `.env`**
   ```env
   XAVIER_MEMORY_BACKEND=surreal
   XAVIER_SURREALDB_URL=ws://localhost:8000
   XAVIER_SURREALDB_USER=your-user
   XAVIER_SURREALDB_PASS=your-password
   ```

5. **Restart Xavier**
   ```bash
   docker compose up -d xavier
   ```

---

## Rollback Procedures

### Rolling back to File from SurrealDB

1. Stop Xavier: `docker compose stop xavier`
2. Backup SurrealDB data: `curl -X POST http://localhost:8000/sql ...`
3. Set `XAVIER_MEMORY_BACKEND=file`
4. Migrate if needed: use file as-is
5. Restart: `docker compose up -d xavier pplx-embed`

### Emergency rollback to SQLite

If SurrealDB crashes and won't restart:

1. Stop Xavier immediately
2. Set `XAVIER_MEMORY_BACKEND=sqlite` and `XAVIER_MEMORY_SQLITE_PATH` to an existing SQLite file
3. Start Xavier: `docker compose up -d xavier`
4. Verify: `curl http://localhost:8003/build | jq .memory_store.selected_backend`
5. Investigate SurrealDB failure without pressure

---

## Backend Performance Characteristics

| Backend | Median Retrieval | Notes |
|---------|-----------------|-------|
| `vec` | ~22ms | With vector search; FTS5 + RRF ranking; no network overhead |
| `surreal` | Variable | Network latency + schema sync overhead; scales with cluster size |
| `sqlite` | O(n) | In-memory text filtering on large datasets; no vector acceleration |
| `file` | Variable | Filesystem overhead; no structured indexing |

### Performance Notes

- **`vec`** — Fastest for most retrieval workloads. Embedding-based similarity search avoids scanning all records. RRF combines vector + keyword rankings for better relevance than either alone.
- **`surreal`** — Adds network round-trip latency per operation plus schema synchronization overhead. Best when you need true distributed writes across multiple nodes.
- **`sqlite` (legacy)** — Simple `LIKE`/`FTS` text matching scans all records. Degrades linearly with dataset size. No vector support.
- **`file`** — JSON files on disk with no indexing. Performance depends on filesystem caching.

---

## Health Check Endpoints

| Backend | Check | Command |
|---------|-------|---------|
| All | `GET /health` | `curl http://localhost:8003/health` |
| All | `GET /build` → `.memory_store.selected_backend` | `curl http://localhost:8003/build \| jq .memory_store.selected_backend` |
| All | `GET /readiness` | `curl http://localhost:8003/readiness \| jq .` |
| File | `GET /readiness` → `.workspace` | `curl http://localhost:8003/readiness \| jq .workspace` |
| SQLite | File size check | `ls -lh ./data/workspaces/default/memory-store.sqlite3` |

---

## Migration Reference

| From → To | Script |
|-----------|--------|
| File → SurrealDB | `scripts/migrate_file_to_surreal.py --workspace <ID> --reinstall` |
| File → SQLite | `scripts/migrate_file_to_sqlite.py --workspace <ID> --reinstall` |
| File → Vec | `scripts/migrate_file_to_vec.py --workspace <ID> --reinstall` |
| SurrealDB → SQLite | Dump via REST API, then `scripts/migrate_file_to_sqlite.py` |
| SurrealDB → Vec | `scripts/migrate_surreal_to_vec.py --workspace <ID> --reinstall` |
| SQLite → SurrealDB | `scripts/migrate_file_to_surreal.py --workspace <ID> --reinstall` |
| SQLite → Vec | `scripts/migrate_sqlite_to_vec.py --workspace <ID> --reinstall` |

Use `--all-workspaces` to process every workspace in `XAVIER_WORKSPACE_DIR` at once.

---

## Environment Variables Reference

```env
# Backend selection
XAVIER_MEMORY_BACKEND=vec          # Options: vec (recommended), surreal, sqlite, file

# Vec backend (recommended)
XAVIER_MEMORY_VEC_PATH=./data/workspaces/default/vec-store.sqlite3
XAVIER_EMBEDDING_DIMENSIONS=768    # Must match your embedding model output size

# File backend
XAVIER_WORKSPACE_DIR=./data/workspaces

# SurrealDB backend (optional)
XAVIER_SURREALDB_URL=ws://surrealdb:8000
XAVIER_SURREALDB_USER=root
XAVIER_SURREALDB_PASS=your-password

# SQLite backend (legacy)
XAVIER_MEMORY_SQLITE_PATH=./data/workspaces/default/memory-store.sqlite3
```

See `.env.example` for the complete set of configuration options.
