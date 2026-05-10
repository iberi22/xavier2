# Xavier — Project State

**Project:** iberi22/xavier
**Last Updated:** 2026-05-10 (post-merge cleanup)
**Status:** Active Development

---

## Current Status

| Check | Status | Notes |
|-------|--------|-------|
| **Build** | ✅ Pass | `cargo build --release` |
| **Tests** | ✅ Pass | All unit + integration tests |
| **Lint** | ✅ Pass | `cargo clippy` clean |
| **Docker** | ✅ Available | `docker compose up -d` |
| **Health Endpoints** | ✅ Working | `/health`, `/readiness` |

---

## Module Status

| Module | Status | Notes |
|--------|--------|-------|
| **Core Runtime** | ✅ Complete | Rust + Tokio |
| **HTTP API** | ✅ Complete | Axum-based |
| **MCP Server** | ✅ Complete | Streamable HTTP transport |
| **SurrealDB Memory** | ✅ Complete | Hybrid search (BM25 + vectors) |
| **Belief Graph** | ✅ Complete | Graph relationships |
| **Code Graph Index** | ✅ Complete | SQLite sidecar |
| **Hybrid Search** | ✅ Complete | BM25 + vector reranking |
| **Agent Runtime** | ✅ Complete | `/agents/run` endpoint |
| **Workspace Isolation** | ✅ Enforced | Multi-tenant safe |
| **Usage Tracking** | ✅ Complete | `/v1/account/usage` |
| **Semantic Caching** | ✅ Complete | Tier-1 caching |
| **Overdrive Pipeline** | ✅ Complete | HyDE, Self-Correction, RRF |

---

## Verified Features

| Feature ID | Description | Status |
|------------|-------------|--------|
| feat-unified-storage | SurrealDB for Vector + Graph | ✅ |
| feat-hybrid-search | BM25 + Vector retrieval | ✅ |
| feat-belief-graph | Belief relationships | ✅ |
| feat-mcp-server | HTTP-first + MCP | ✅ |
| feat-code-graph-index | AST/symbol search | ✅ |
| feat-src-reference | Source documentation | ✅ |

---

## Recent Changes

| Change | Date | Description |
|--------|------|-------------|
| fix(crypto): remove generic-array dep | 2026-05-10 | Use `aes_gcm::Nonce` instead of `GenericArray::from_slice` — unblocks Dependabot #177 |
| PR #210 merged | 2026-05-10 | `git2` 0.19.0 → 0.20.4 (Dependabot, all CI green) |
| PR #209 merged | 2026-05-10 | Chronicle workflow, multi-provider spawn, Groq, configurable RRF |
| Chronicle module | 2026-05-10 | Harvest, redact, generate, publish, CLI subcommand |
| Spawn base | 2026-05-10 | Multi-provider agent spawn + Groq provider |
| RRF configurable | 2026-05-10 | `XAVIER_RRF_K` env var for hybrid search |
| Workspace isolation | 2026-03-21 | Enforced `WorkspaceRegistry` |
| Monorepo formalization | 2026-03-19 | Rust + Node mixed workspace |
| MCP surface | 2026-03-19 | Model Context Protocol added |
| HTTP API | 2026-03-17 | Agent-safe HTTP endpoints |

## Open Dependabot PRs

| PR | Dep | Status | Blocker |
|----|-----|--------|---------|
| **#177** | `generic-array` 0.14.7 → 1.4.1 | ⏳ REBASE TRIGGERED | Fix landed in main (df37fe4). `@dependabot rebase` sent. Awaiting CI re-run. |
| **#183** | `tauri` 2.10.3 → 2.11.1 | ✅ CLOSED | Dependabot auto-closed: tauri superseded by newer version. |

---

## Infrastructure

| Component | Status |
|-----------|--------|
| Docker Compose | ✅ Configured |
| Health Checks | ✅ `/health`, `/readiness` |
| Smoke Tests | ✅ `scripts/release-smoke.sh` |

---

## Key Metrics

| Metric | Value |
|--------|-------|
| Features Passing | 6/6 |
| Last Audit | 2026-03-20 |
| Rust Version | 2024 Edition |

---

*Last updated: 2026-03-25*
