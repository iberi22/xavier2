# Xavier2 — Public Release Roadmap (Master Plan)

> **Generated:** 2026-04-30  
> **Owner:** Xavier2 CEO  
> **Status:** Planning  
> **Target v0.5 Public Release:** Q2 2026

---

## Table of Contents

1. [Pre-Release Checklist](#1-pre-release-checklist)
2. [Showstoppers & Quick Wins](#2-showstoppers--quick-wins)
3. [v0.5 — Minimal Public-Ready Release](#3-v05--minimal-public-ready-release)
4. [v1.0 — Full Feature Parity](#4-v10--full-feature-parity)
5. [v1.1+ — Nice-to-Have Improvements](#5-v11--nice-to-have-improvements)
6. [Complete Dependency Graph](#6-complete-dependency-graph)
7. ["Good Enough for v0.5" Threshold](#7-good-enough-for-v05-threshold)
8. [Risk Matrix](#8-risk-matrix)

---

## 1. Pre-Release Checklist

> **MAY NOT ship public with ANY of these unchecked.**

| # | Item | Category | Effort | Status | Dependency |
|---|------|----------|--------|--------|------------|
| 1 | Remove hardcoded credentials in kanban.rs (PlankaConfig default hardcodes password `swaladmin2026`, email `admin@swal.ai`, IP `192.168.1.8:3000`) | **SECURITY-SHOWSTOPPER** | 30 min | ❌ OPEN | None |
| 2 | Remove `dev-token` fallback in all auth paths — `XAVIER2_TOKEN` default must error, not silently use `dev-token` | **SECURITY-SHOWSTOPPER** | 1 hr | ❌ OPEN | None |
| 3 | Remove `#[derive(Debug)]` from all structs containing secrets (password, token, keys, credentials) across the codebase | **SECURITY** | 1 hr | ❌ OPEN | None |
| 4 | Finish prompt_guard sanitize — incomplete patterns, missing contextual sanitization (template patterns only partially blocked, no URL/LLM injection context) | **SECURITY** | 2 hr | ❌ OPEN | None |
| 5 | Create `docs/` for public consumption — current docs are internal/agent-only or scattered; need curated public-facing docs | **DOCS** | 4 hr | ❌ OPEN | None |
| 6 | Create `examples/` directory with working examples (CLI, HTTP API, MCP integration) | **DOCS** | 2 hr | ❌ OPEN | #5 |
| 7 | Clean up README — make CEO concept public-friendly, remove internal references, add proper badges and quick-start | **DOCS** | 1 hr | ❌ OPEN | #5 |
| 8 | Audit all `unwrap()` and `expect()` in public API paths (crash risk) | **QUALITY** | 2 hr | ❌ OPEN | None |
| 9 | Ensure no `localhost`-only assumption in HTTP binding — hardcoded bind patterns | **QUALITY** | 30 min | ❌ OPEN | None |
| 10 | GitHub repo visibility finalization — set license, CONTRIBUTING.md, issue templates, CI workflow badges | **OPS** | 1 hr | ❌ OPEN | #7 |

---

## 2. Showstoppers & Quick Wins

### 🚫 Showstoppers (Block Public Release)

| Item | Why It Blocks | Fix |
|------|---------------|-----|
| Hardcoded credentials in kanban.rs | Any public user can `git grep` and see `swaladmin2026` + `admin@swal.ai` + internal IP — instant compromise | → Env vars only, panic on missing; remove Default impl |
| `dev-token` fallback | Every user who pulls and runs without XAVIER2_TOKEN gets `dev-token` as auth — trivial auth bypass | → Unconditional env check, fail with clear error |
| Debug derive on sensitive structs | `std::fmt::Debug` leaks secrets to logs, error messages, and debug output | → Manual Debug impl that redacts fields |
| prompt_guard incomplete | Sanitize misses contextual attacks, LLM-escaped payloads, and advanced jailbreak patterns | → Add missing patterns, contextual detection |

### ⚡ Quick Wins (< 1 hour)

| Item | Time |
|------|------|
| Remove PlankaConfig Default impl (hardcoded creds) | 30 min |
| Remove `dev-token` default in env var parsing | 20 min |
| Add `.cargo/config.toml` deny of sensitive derives | 15 min |
| Fix HTTP binding to respect XAVIER2_HOST properly | 30 min |
| Add SECURITY.md banner pointing to docs | 15 min |
| Create `.gitignore` for any credential artifacts | 5 min |
| Pin Rust edition/MSRV in Cargo.toml | 10 min |
| Add CI badge scans (cargo audit, clippy) | 30 min |

---

## 3. v0.5 — Minimal Public-Ready Release

> **Goal:** Ship something safe, documented, and usable. Security + Docs + Basic CLI.  
> **Target effort:** ~2-3 days  
> **Must ship:** After all pre-release checklist items complete.

### 3.1 Priority Order

| Priority | Item | Category | Effort | Depends On |
|----------|------|----------|--------|------------|
| **P0** | Fix kanban.rs hardcoded credentials | Security | 30 min | — |
| **P0** | Fix dev-token fallback everywhere (search all `unwrap_or`/`unwrap_or_else` for token patterns) | Security | 1 hr | — |
| **P0** | Audit and fix Debug on sensitive structs | Security | 1 hr | — |
| **P1** | Complete prompt_guard sanitize patterns | Security | 2 hr | — |
| **P1** | Create public-facing README revision | Docs | 1 hr | — |
| **P1** | Create docs/ entry point (ARCHITECTURE.md, API.md, QUICKSTART.md) | Docs | 4 hr | — |
| **P1** | Create examples/ with CLI, HTTP, MCP examples | Docs | 2 hr | #1 docs |
| **P2** | Add cargo-audit to CI | Quality | 30 min | — |
| **P2** | Fix all panicking unwraps in public API paths | Quality | 2 hr | — |
| **P2** | Add issue/PR templates | Ops | 30 min | — |
| **P3** | Add CHANGELOG.md for v0.5 | Docs | 30 min | All above |
| **P3** | Tag v0.5.0 release in GitHub | Release | 15 min | All above |

### 3.2 Effort Breakdown

| Category | Items | Total Effort |
|----------|-------|-------------|
| 🔒 Security (P0-P1) | 4 | **4.5 hr** |
| 📝 Documentation (P1) | 3 | **7 hr** |
| 🛡️ Quality/CI (P2) | 3 | **3 hr** |
| 🚀 Release (P3) | 2 | **45 min** |
| **TOTAL** | **12** | **~15.25 hr (~2 days)** |

### 3.3 v0.5 Success Criteria

```
✅ No hardcoded credentials in source code
✅ No dev-token fallback — env var required or fail fast
✅ No Debug leaks of passwords/tokens in production paths
✅ Prompt guard meets basic injection detection and sanitization
✅ README is public-ready (no internal secrets, CEO concept explained accessibly)
✅ docs/ exists with at minimum: ARCHITECTURE, API, QUICKSTART
✅ examples/ has 3 working examples (CLI, HTTP, MCP)
✅ CI passes (build + clippy + cargo-audit)
✅ GitHub: proper license, CONTRIBUTING, issue templates
```

---

## 4. v1.0 — Full Feature Parity

> **Goal:** Production-grade memory system with multi-tier architecture, proper MCP, split System modules.  
> **Target effort:** ~3-4 weeks  
> **Prerequisite:** v0.5 released

### 4.1 Priority Order

| Priority | Item | Category | Effort | Depends On |
|----------|------|----------|--------|------------|
| **P0** | Implement reasoning chain in System2 — currently `reasoning_chain: vec![]` (empty stub) | Architecture | 4 hr | v0.5 |
| **P0** | Refactor System3 god object (900+ lines → max 200-300 per module) — split LLMs, caches, formatting into sub-modules | Architecture | 8 hr | v0.5 |
| **P0** | Wire MCP through agent pipeline — currently MCP bypasses System1/2/3 entirely | Architecture | 6 hr | v0.5 |
| **P0** | Break QmdMemory into focused modules (3000+ lines → max 500 per file) | Architecture | 10 hr | v0.5 |
| **P0** | Finish consolidation engine — current implementation in `consolidation/` has real logic but needs scheduler, triggers, and proper integration | Architecture | 4 hr | v0.5 |
| **P1** | Add memory tiers (Working → Archival), auto-migration | Feature | 6 hr | #4 (QmdMemory split) |
| **P1** | Add memory importance scoring (recency + access + novelty) | Feature | 4 hr | #4 |
| **P1** | Add structured memory types (episodic, semantic, procedural, declarative) | Feature | 3 hr | v0.5 |
| **P1** | Memory consolidation scheduler (background cron/interval) | Feature | 3 hr | #5 |
| **P1** | Enhanced CLI: `xavier2 save --type X`, `xavier2 search --type X`, `xavier2 recall` | Feature | 3 hr | v0.5 |
| **P2** | Memory summarization (LLM-powered compression) | Feature | 4 hr | #10 |
| **P2** | Memory graph/entity relationships | Feature | 5 hr | #4 |
| **P2** | Context window optimization (auto-summarize) | Feature | 4 hr | #11 |
| **P3** | Memory TTL/auto-expiry per type | Feature | 2 hr | #9 |
| **P3** | Memory tags & categories | Feature | 1 hr | #9 |
| **P3** | Memory versioning | Feature | 3 hr | #7 |
| **P3** | Memory analytics (`xavier2 stats --insights`) | Feature | 2 hr | #4 |
| **P3** | Add end-to-end encryption (AES-256-GCM) for agent communication | Security | 6 hr | v0.5 |

### 4.2 Effort Breakdown

| Category | Items | Total Effort |
|----------|-------|-------------|
| 🏗️ Architecture (P0) | 5 | **32 hr** |
| ⚡ Features (P1) | 6 | **25 hr** |
| 🚀 Features (P2) | 3 | **13 hr** |
| ✨ Features (P3) | 5 | **14 hr** |
| **TOTAL** | **19** | **~84 hr (3-4 weeks)** |

### 4.3 v1.0 Success Criteria

```
✅ System2 has a real reasoning chain (not empty vec)
✅ System3 split into focused modules (<300 lines each)
✅ MCP uses System1/2/3 pipeline (not direct bypass)
✅ QmdMemory broken into focused modules (<500 lines each)
✅ Consolidation runs on a background scheduler
✅ Memory tiers: Working ↔ Archival auto-migration
✅ Structured memory types with proper save/search/filter
✅ Enhanced CLI with all v0.5 features + structured commands
✅ Memory summarization works
✅ Memory graph with entity relationship queries
✅ ALL v0.5 criteria still passing
```

---

## 5. v1.1+ — Nice-to-Have Improvements

> **Goal:** Differentiators, enterprise features, ecosystem growth.  
> **Target effort:** 2-4 weeks per sub-release  
> **Prerequisite:** v1.0 released

### 5.1 v1.1 — Production Hardening

| Priority | Item | Effort | Quality |
|----------|------|--------|---------|
| P1 | Rate limiting (prevent DoS on public API) | 3 hr | Must-have for public API |
| P1 | Audit logging (security events, all writes) | 4 hr | Compliance |
| P1 | Memory import/export (JSON, CSV) | 2 hr | User demand |
| P2 | Multi-workspace support | 3 hr | Power users |
| P2 | Memory reflection (self-analysis, pattern detection) | 4 hr | Differentiation |
| P2 | WASM-based prompt_guard cross-platform | 5 hr | Reach |
| P3 | Memory sharing between agents (team knowledge base) | 4 hr | Enterprise |

### 5.2 v1.2 — Ecosystem & Performance

| Priority | Item | Effort | Quality |
|----------|------|--------|---------|
| P1 | Performance benchmarking suite | 2 hr | CI gating |
| P1 | P99 latency <50ms for all endpoints | 4 hr | SLA |
| P2 | Plugin system for custom memory backends | 6 hr | Extensibility |
| P2 | SurrealDB adapter (current is stub) | 4 hr | Alternative storage |
| P2 | OpenAPI/Swagger docs generation | 3 hr | Developer experience |
| P3 | Web UI dashboard | 8 hr | Visual appeal |
| P3 | Python SDK package | 5 hr | Ecosystem |
| P3 | JavaScript/Typescript SDK package | 5 hr | Ecosystem |

### 5.3 v2.0 — Enterprise Features

| Priority | Item | Effort | Quality |
|----------|------|--------|---------|
| P1 | Multi-tenant isolation (workspace per team) | 6 hr | Enterprise |
| P1 | Role-based access control (RBAC) | 8 hr | Enterprise |
| P1 | Audit trail for compliance | 5 hr | Enterprise |
| P2 | SAML/SSO integration | 6 hr | Enterprise |
| P2 | Self-hosted Helm chart for Kubernetes | 4 hr | Enterprise |
| P3 | Horizontal scaling / sharding | 10 hr | Scale |

---

## 6. Complete Dependency Graph

```
    v0.5 (Security + Docs)                ◄── CRITICAL PATH
    ├── kanban.rs hardcoded creds          ─ No deps
    ├── dev-token fallback                 ─ No deps
    ├── Debug on sensitive structs         ─ No deps
    ├── prompt_guard complete              ─ No deps
    ├── public README                      ─ No deps
    ├── docs/ + examples/                  ─ No deps
    └── CI/packaging                       ─ No deps
          │
    ┌─────┘
    ▼
    v1.0 (Architecture + Features)         ◄── PARALLEL WAVES
    │
    ├── Phase A: Core Architecture
    │   ├── System2 reasoning chain        ─ Depends on: v0.5
    │   ├── System3 refactor               ─ Depends on: v0.5
    │   └── MCP agent pipeline             ─ Depends on: v0.5
    │
    ├── Phase B: Storage Refactor
    │   └── QmdMemory split                ─ Depends on: v0.5
    │         │
    │         ├── Memory tiers             ─ Depends on: QmdMemory split
    │         ├── Memory importance scoring ─ Depends on: QmdMemory split
    │         └── Memory graph             ─ Depends on: QmdMemory split
    │
    ├── Phase C: Feature Parity
    │   ├── Consolidation scheduler         ─ Depends on: v0.5 + QmdMemory split
    │   ├── Structured memory types         ─ Depends on: v0.5
    │   ├── Enhanced CLI                    ─ Depends on: v0.5
    │   ├── Memory summarization            ─ Depends on: consolidation
    │   └── Context optimization            ─ Depends on: memory summarization
    │
    └── Phase D: Polish
        ├── Memory TTL/expiry              ─ Depends on: structured types
        ├── Memory tags                    ─ Depends on: QmdMemory split
        ├── Memory versioning              ─ Depends on: QmdMemory split
        ├── Memory analytics               ─ Depends on: QmdMemory split
        └── E2E encryption                 ─ Depends on: v0.5
              │
         ┌────┘
         ▼
    v1.1+ (Nice-to-Have)                   ◄── NON-BLOCKING
    ├── Rate limiting                      ─ Depends on: v1.0
    ├── Audit logging                      ─ Depends on: v1.0
    ├── Multi-workspace                    ─ Depends on: v1.0
    ├── Python/JS SDKs                     ─ Depends on: v1.0
    └── Web UI                             ─ Depends on: v1.0
```

### Critical Path (Minimum Timeline)

```
v0.5 (2 days) → Phase A (3 days) → Phase B (3 days) → Phase C (5 days) → Phase D (4 days) → v1.0 Release
                                                                                                    │
                                                                                                    ▼
                                                                                             v1.1+ (ongoing)
```

---

## 7. "Good Enough for v0.5" Threshold

> What's the minimum MVP that's safe to release?

### ✅ MUST PASS (Gate Items)

| Check | Description | Pass/Fail |
|-------|-------------|-----------|
| 🔒 No hardcoded secrets | `git grep` for password, secret, token, key in source → only valid env vars | ❌ FAIL |
| 🔒 Auth can't be bypassed | Running without XAVIER2_TOKEN must fail with clear error, not use `dev-token` | ❌ FAIL |
| 🔒 No Debug on secrets | `grep derive(Debug)` on password/token/credential structs → none | ❌ FAIL |
| 🔒 Basic injection protection | Ran prompt_guard test suite → all inject patterns blocked | ❌ FAIL |
| 📝 README is public-worthy | No internal IPs, no CEO-only messages, no hardcoded paths | ❌ FAIL |
| 📝 Minimum docs exist | At least: QUICKSTART.md, API.md, ARCHITECTURE.md (public version) | ❌ FAIL |
| 📝 Examples work | `examples/cli.sh`, `examples/http.sh`, `examples/mcp.sh` run without errors | ❌ FAIL |
| 🛡️ CI passes with audit | `cargo build`, `cargo clippy`, `cargo audit` all green | ❌ FAIL |

### ✅ NICE TO HAVE (Blocks Nothing)

| Item | If Ready |
|------|----------|
| CHANGELOG.md | Include if time |
| GitHub Actions CI | Include if time |
| Docker image tagged | Include if time |
| Issue/PR templates | Include if time |

### ✅ THE LINE

If all 8 items in **MUST PASS** are green and you have at least 2 of the **NICE TO HAVE** items, you can ship v0.5.

---

## 8. Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Credential leak before v0.5 | **HIGH** | CRITICAL | Immediate fix (kanban.rs), add `.secrets.baseline` scanning |
| Debug leak in logs | **HIGH** | HIGH | Manual Debug impls, regex audit script |
| Uncaught injection bypass | **MEDIUM** | HIGH | Test harness for prompt_guard, community review |
| CI flakiness | **LOW** | MEDIUM | Fix first, use deterministic test fixtures |
| Architecture debt blocks v1.0 | **HIGH** | HIGH | Ship QmdMemory split as P0, parallelize System3 refactor |
| MCP integration breaks existing users | **MEDIUM** | MEDIUM | Add integration tests for stdio + HTTP MCP modes |
| Public release with doc gaps | **MEDIUM** | MEDIUM | Strict gating on MUST PASS items |

---

## Appendix A: Quick Reference — Key Files to Fix

| File | Issue | Action |
|------|-------|--------|
| `src/tools/kanban.rs` | Hardcoded credentials in Default impl | Remove Default, env-only, fail on missing |
| `src/security/auth.rs` | dev-token fallback | Remove fallback, require env |
| `src/security/prompt_guard.rs` | Incomplete sanitize | Add missing patterns, contextual detection |
| `src/agents/system2.rs` | Empty reasoning_chain | Implement actual reasoning pipeline |
| `src/agents/system3.rs` | 900+ line god object | Split into sub-modules |
| `src/server/mcp_server.rs` | 1300+ lines, bypasses agent pipeline | Refactor, wire through System1/2/3 |
| `src/memory/qmd_memory.rs` | 3000+ lines (105KB file) | Split per concern |
| `src/consolidation/` | Needs scheduler + triggers | Add interval/tick scheduler |
| `README.md` | Mentions `dev-token`, internal IP | Clean up for public |
| `docs/` | Internal-heavy, no public entry | Create curated public docs |
| `examples/` missing | No working examples | Create CLI/HTTP/MCP examples |

## Appendix B: Estimated Timeline

| Phase | Calendar Time | Parallelizable? | Team Size |
|-------|--------------|----------------|-----------|
| v0.5 Pre-release | 2-3 days | ✅ (security + docs parallel) | 1-2 |
| v0.5 Release | — | — | — |
| v1.0 Phase A | 3-4 days | ✅ (System2/3 vs QmdMemory split) | 2 |
| v1.0 Phase B | 3-4 days | ✅ (depends on A) | 1-2 |
| v1.0 Phase C | 4-5 days | Partially | 1-2 |
| v1.0 Phase D | 3-4 days | ✅ (parallel with testing) | 1-2 |
| v1.0 Release | — | — | — |
| v1.1+ ongoing | 2-4 weeks each | ✅ (items parallel) | 1-2 |

**Total to v1.0:** ~3-5 weeks from today  
**Total to v0.5 (safe public release):** ~2-3 days from today

---

*Xavier2 CEO — Construyendo el futuro con memoria.*
