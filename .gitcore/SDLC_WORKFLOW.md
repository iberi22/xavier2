# SDLC Workflow - Xavier

**Versión:** 1.0
**Fecha:** 2026-04-14
**Proyecto:** Xavier Cognitive Memory System

---

## 1. Visión General

Este documento define el flujo de trabajo SDLC (Software Development Life Cycle) para Xavier, un motor de memoria cognitiva para agentes IA construido en Rust.

---

## 2. Stack Técnico

| Componente | Tecnología |
|-------------|------------|
| Runtime | Rust + Tokio |
| HTTP Server | Axum |
| Memory Backend | SQLite + SQLite-vec |
| MCP | Model Context Protocol |
| UI | React + Vite |
| Docs | Astro (Starlight) |
| Tests | cargo test |

---

## 3. Flujo SDLC

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           SDLC XAVIER                                  │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ISSUE ──> SRC ──> IMPLEMENT ──> TEST ──> REVIEW ──> DEPLOY            │
│                                                                         │
│  tools: gh CLI         cargo test    cargo clippy    docker/deploy      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Fases

### Fase 1: Análisis

| Step | Acción | Command |
|------|--------|---------|
| 1.1 | Crear issue | `gh issue create --title "..."` |
| 1.2 | Asignar labels | `gh issue edit --add-label feature` |
| 1.3 | Documentar en SRC | `docs/SRC/REQUIREMENTS.md` |

### Fase 2: Implementación

| Step | Acción | Command |
|------|--------|---------|
| 2.1 | Crear branch | `git checkout -b feat/description-#issue` |
| 2.2 | Implementar | Rust code en `src/` |
| 2.3 | Tests | `cargo test -p xavier` |
| 2.4 | Lint | `cargo clippy -- -D warnings` |

### Fase 3: Testing

| Step | Acción | Command |
|------|--------|---------|
| 3.1 | Unit tests | `cargo test --lib` |
| 3.2 | Integration tests | `cargo test -p xavier` |
| 3.3 | E2E | `cargo test --test e2e` |
| 3.4 | Benchmarks | `cargo bench` |

### Fase 4: Review

| Step | Acción |
|------|--------|
| 4.1 | Code review via PR |
| 4.2 | Verificar tests passing |
| 4.3 | Verificar clippy passing |
| 4.4 | @architect approval |

### Fase 5: Deploy

| Step | Acción |
|------|--------|
| 5.1 | Merge a main |
| 5.2 | Tag release |
| 5.3 | Docker image build |
| 5.4 | Update CHANGELOG.md |

---

## 5. Convenciones de Commit

```
type(scope): description #issue

Types:
- feat: nueva feature
- fix: bug fix
- docs: documentación
- refactor: refactorización
- test: tests
- chore: mantenimiento

Scopes:
- memory: sistema de memoria
- server: HTTP/MCP server
- agents: runtime de agentes
- workspace: multi-tenant
- security: autenticación/encryption

Ejemplos:
feat(memory): add hybrid search RRF fusion #45
fix(server): resolve token validation timeout #47
docs(api): document /memory/query endpoint #48
```

---

## 6. Labels de Issues

| Label | Uso |
|-------|-----|
| `feat` | Nueva feature |
| `bug` | Bug report |
| `docs` | Documentación |
| `refactor` | Refactorización |
| `test` | Tests |
| `priority-high` | Alta prioridad |
| `good-first-issue` | Good first issue |

---

## 7. Estado de Features

Ver `.gitcore/features.json` para tracking oficial de features.

| Feature | Status | Backend |
|---------|--------|---------|
| Hybrid Search | ✅ Pass | BM25 + Vector RRF |
| Belief Graph | ✅ Pass | In-memory |
| Multi-tenant | ✅ Pass | WorkspaceRegistry |
| MCP Server | ✅ Pass | HTTP + MCP |
| Code Indexing | ✅ Pass | SQLite AST |
| SQLite | ✅ Pass | Graph DB (via sqlite-rtree optional) |

---

## 8. Testing Commands

```bash
# Unit tests
cargo test --lib

# All tests
cargo test -p xavier

# E2E
cargo test --test e2e -- --nocapture

# Benchmarks
cargo bench --bench api_v1

# Lint
cargo clippy -p xavier --all-targets -- -D warnings

# Format
cargo fmt --check
```

---

*Xavier v0.4.1 - SDLC Workflow*
