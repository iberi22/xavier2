# Xavier / Cortex / Engram — Architecture

> **Goal:** Stable, dogfooded, green. Xavier as open core, Cortex as enterprise layer, Engram as design reference only.

---

## System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     SWAL Memory Stack                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────────────────────────────┐│
│  │   Engram    │    │              Cortex                  ││
│  │ (reference) │    │        (enterprise layer)          ││
│  │             │    │                                     ││
│  │ - Ideas     │    │  - Multi-tenancy (RBAC)            ││
│  │ - Patterns   │    │  - Audit logging                   ││
│  │ - DesignRef │    │  - API keys (pk_live_xxx)          ││
│  │ NOT runtime │    │  - Rate limiting                    ││
│  └─────────────┘    │  - Security governance              ││
│         ▲           │  - Docker container                 ││
│         │ design    └──────────────┬──────────────────────┘│
│         │ reference                     │                  │
│         │                    ┌──────────▼───────────────┐  │
│         │                    │        Xavier             │  │
│         │                    │     (open core engine)      │  │
│         │                    │                            │  │
│         │                    │  - HTTP API (:8006)       │  │
│         │                    │  - VecSqliteMemoryStore    │  │
│         │                    │  - QmdMemory (in-memory)   │  │
│         │                    │  - Hybrid search (RAG)     │  │
│         │                    │  - Code graph (180 files)  │  │
│         │                    │  - MCP-stdio mode          │  │
│         │                    └────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Component Roles

### Xavier — Core Open Source Engine
- **Role:** Primary memory and code graph engine
- **Repo:** `E:\scripts-python\xavier`
- **Binary:** `C:\Users\belal\.cargo\target_global\release\xavier.exe`
- **Protocols:** HTTP (:8006), MCP-stdio, CLI
- **Storage:** `xavier_memory_vec.db` (SQLite + vec)
- **Build:** `cargo build --release -p xavier --bin xavier`
- **Check:** `cargo check -p xavier --bin xavier` (exit 0, zero warnings)

### Cortex — Enterprise / Security / Governance
- **Role:** Multi-tenant enterprise layer with audit, RBAC, rate limiting
- **Repo:** `E:\scripts-python\cortex`
- **Runs:** Docker (`docker compose up -d cortex`)
- **HTTP:** localhost:8003
- **Token:** `dev-token`
- **Storage:** `/data/` inside container (persisted via volume)

### Engram — Design Reference Only
- **Role:** Inspirational reference for patterns, NOT a runtime dependency
- **Binary:** `C:\Users\belal\AppData\Local\Temp\engram\engram.exe`
- **Status:** Available but 0/3 memory matches in benchmarks — not comparable
- **Note:** Treat as documentation, not infrastructure

---

## Current Status (2026-04-20)

### ✅ Green — Loop watchdog active
- **Script:** `E:\scripts-python\xavier\scripts\memory_triad_loop.ps1`
- **Interval:** 900 seconds (15 min)
- **Latest cycles (all green):**
  - `20260420_124447` — 5/5 steps exit=0
  - `20260420_122928` — 5/5 steps exit=0
  - `20260420_121409` — 5/5 steps exit=0

### ✅ Verified — Xavier
- Memory: 3/3 matches
- Code graph: 5/5 queries, 180 files, 2919 symbols
- Avg search: ~24ms
- Binary: fresh build, zero Rust warnings

### ⚠️ Issue — Cortex (embedding dimension mismatch)
- **Error:** `Dimension mismatch: expected 768, received 1024` in `memory_embeddings`
- **Impact:** Cortex container restarting, unavailable for memory triad
- **Fix needed:** Align embedding model dimensions between xavier (1024) and cortex vec store (768)

### ✅ Verified — Engram
- Runs fine, ~315ms avg latency
- 0/3 memory matches (not a match for SWAL data)
- Not comparable for code graph

---

## Benchmark Results (latest triad cycle)

| System     | Memory | Code Graph | Latency   | Status |
|------------|--------|------------|-----------|--------|
| Xavier    | 3/3 ✅ | 5/5 ✅      | ~24ms     | Green  |
| Cortex     | N/A ❌ | N/A ❌      | N/A       | Red    |
| Engram     | 0/3 ⚠️ | N/A (nc)   | ~315ms    | Yellow |

---

## Key Files

### Xavier
- `src/cli.rs` — HTTP + CLI commands
- `src/memory/qmd_memory.rs` — QmdMemory with VecSqliteMemoryStore
- `src/search/hybrid.rs` — RAG hybrid search
- `scripts/memory_triad_loop.ps1` — Watchdog loop
- `scripts/memory_triad_benchmark.py` — Benchmark runner

### Cortex
- `src/api/enterprise_http.rs` — Enterprise HTTP routes
- `src/app/memory_service.rs` — Memory with storage + embedding
- `src/enterprise/` — tenancy, rbac, audit, keys, rate_limit
- `docker-compose.yml` — Docker config

---

## Architecture Principles

1. **Xavier is the open core** — public, community-facing, simple deploy
2. **Cortex is the enterprise overlay** — security, governance, multi-tenant
3. **Engram is a design mirror** — reference patterns, not runtime dependency
4. **No tight coupling** — each system has its own storage, own API
5. **Dogfood first** — SWAL uses Xavier for its own code/graph before selling it
