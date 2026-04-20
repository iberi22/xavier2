# CLAUDE.md - Xavier2 Cognitive Memory System

> Guía de desarrollo, pruebas y auditoría para Xavier2.
> Última auditoría: 2026-04-14

## Proyecto: xavier2

**Descripción:** Motor de memoria cognitiva para agentes IA - Rust binario monolítico
**Stack:** Rust, Tokio, SQLite + SQLite-vec, React/Vite, Axum
**Tipo:** library/daemon/api

**Versión:** 0.4.1
**License:** MIT (Dual: MIT + Enterprise)

---

## 🚀 Quick Start

```bash
# Instalar dependencias de Rust
cargo build --release

# Ejecutar servidor Xavier2
./target/release/xavier2

# Tests principales
cargo test --lib -p xavier2

# Benchmarks
cargo bench --bench api_v1
```

## 📁 Estructura del Proyecto

```
xavier2/
├── src/
│   ├── agents/          # Agent runtime (System 1-2-3 layers)
│   │   ├── system1.rs   # Fast retrieval
│   │   ├── system2.rs   # Reasoning
│   │   └── system3.rs   # Oversight & action
│   ├── memory/          # QMD Memory + Belief Graph
│   │   ├── qmd_memory.rs
│   │   ├── belief_graph.rs
│   │   └── store.rs     # Multi-backend (File/SQLite/Vec/Surreal)
│   ├── server/          # HTTP API + MCP server
│   │   ├── http.rs
│   │   └── mcp_server.rs
│   ├── workspace.rs     # Multi-tenant isolation + WorkspaceRegistry
│   ├── sync/            # Chunk-based sync protocol
│   ├── security/        # E2E encryption + pattern detection
│   └── lib.rs
├── code-graph/          # AST indexing (SQLite sidecar)
├── panel-ui/            # React/Vite UI for memory browser
├── docs/site/           # Astro documentation site
├── tests/
│   ├── integration.rs   # 12+ integration test modules
│   ├── sync_test.rs
│   └── e2e.rs
├── benches/
│   ├── api_v1.rs
│   ├── hybrid_search.rs
│   └── cortex.rs
└── .gitcore/
    └── ARCHITECTURE.md  # Critical decisions & tech stack
```

## 🔧 Comandos Principales

| Comando | Descripción |
|---------|-------------|
| `cargo build --release` | Build de producción |
| `cargo test --lib` | Tests unitarios |
| `cargo test -p xavier2` | Todos los tests |
| `cargo test --test e2e -- --nocapture` | E2E tests |
| `cargo clippy --all-targets -- -D warnings` | Linting strict |
| `cargo bench --bench api_v1` | Benchmarks de API |
| `docker compose up` | Run via Docker |

## 🏗️ Arquitectura

Xavier2 es un **motor de memoria cognitiva** para agentes IA, inspirado en el **"System 3 paradigm"**:
- **System 1**: Recuperación rápida (lexical + vector search + belief graph)
- **System 2**: Razonamiento deliberado (Chain of Thought)
- **System 3**: Oversight meta-cognitivo (validación de alucinaciones)

### Características Clave

| Feature | State | Backend | Notes |
|---------|-------|---------|-------|
| **Hybrid Search** | ✅ Pass | BM25 + Vector | RRF fusion, FTS5 indexing |
| **Belief Graph** | ✅ Pass | In-memory | Relaciones conceptuales |
| **Multi-tenant** | ✅ Pass | WorkspaceRegistry | Per-workspace isolation |
| **Checkpoints** | ✅ Pass | Durable store | Recovery & versioning |
| **MCP Server** | ✅ Pass | HTTP + MCP | LLM client integration |
| **Code Indexing** | ✅ Pass | code-graph SQLite | AST-backed symbol search |
| **E2E Encryption** | ✅ Impl | AES-GCM | Cloud tier security |
| **SQLite Storage** | ✅ Pass | SQLite + sqlite-vec | Production-ready default |

### Patrones Clave
- **Hexagonal Architecture**: Separación domain/adapters/ports/app
- **Arc<RwLock<T>>**: Shared state con concurrencia safe
- **Workspace Isolation**: Token-scoped memory per customer
- **Usage Tracking**: Granular quotas (Read/Write/AgentRun/Code)

## 📋 Convenciones de Desarrollo

### Git Workflow (GitCore Protocol)
- Issues: `.github/issues/` + GitHub Issues (sincronización automática)
- Commits: `type(scope): description #issue`
- Branches: `type/short-description-#issue`
- PRs: Reference issue, describe impact on System 1/2/3 layers if applicable

### Código Rust
- Linting: `cargo clippy --all-targets -- -D warnings`
- Formatting: `cargo fmt --check`
- Testing: `cargo test --lib` antes de PR
- Docs: `///` comments en tipos/traits públicos
- Error handling: `anyhow::Result<T>` con contexto

### Async Patterns
- Use `tokio::spawn` con `JoinSet` para múltiples tareas
- `Arc<RwLock<T>>` para state compartida
- `Arc<Mutex<T>>` para modificación única
- Avoid `.await` en loops tight - use stream adapters

### Multi-tenant Safety
- Todos los queries deben pasar `workspace_id`
- Validar token en middleware HTTP
- Usar WorkspaceRegistry para aislamiento
- Tracking uso con UsageEvent (Read/Write/AgentRun/Code)

## 🧪 Testing & Validation

### Cobertura

```
tests/integration.rs     → 12 test modules
├─ a2a_test.rs         → A2A protocol
├─ agents_test.rs      → Agent lifecycle
├─ belief_graph_test.rs → Graph operations
├─ memory_test.rs      → QMD memory
└─ server_test.rs      → HTTP API

tests/e2e.rs           → Server startup, /health endpoint
benches/*.rs           → LoCoMo benchmark, hybrid search, API latency
```

### Comandos
- `cargo test --lib` - Unit tests rápidos
- `cargo test -p xavier2` - Todos los tests
- `cargo test --test e2e -- --nocapture` - E2E con output
- `cargo bench` - Benchmarks completos
- Linting: `cargo clippy -p xavier2 --all-targets -- -D warnings`

---

## 🔐 Security & Robustez

### ✅ Implementado
| Componente | Mecanismo |
|------------|-----------|
| **Auth** | Token header `X-Xavier2-Token`, per-workspace |
| **Isolation** | WorkspaceRegistry, separate Arc<QmdMemory> por workspace |
| **Encryption** | AES-GCM para cloud tier, sha256 para audit chain |
| **Quotas** | Storage limits, request rate limits, per PlanTier |
| **Audit** | Hash chain tamper-evident en memory ops |

### 🎯 Auditoría de Robustez (2026-04-14)

**Status: 8.5/10 - LISTO PARA PRODUCCIÓN**

| Área | Score | Notas |
|------|-------|-------|
| Arquitectura Cognitive | 9/10 | System 3 implementado, validación alucinaciones |
| Multi-tenant | 9/10 | WorkspaceRegistry enforced |
| Memory Integrity | 8/10 | Checkpoints + migrations, SQLite production-ready |
| Tests | 7/10 | Good coverage, stubs para trabajo futuro |
| Security | 8/10 | Token auth, encryption, pattern matching |
| **Feature Status** | 6/6 | All features passing |

**Key Finding**: SQLite + sqlite-vec is the production-ready default backend since v0.4.x. All features validated and passing.

## 📝 Notas para Agentes

- **Leer antes de trabajar**: `.gitcore/ARCHITECTURE.md` (decisiones críticas)
- **Features tracking**: `.gitcore/features.json` (status oficial)
- **Integration points**: HTTP API en `src/server/http.rs`, MCP en `src/server/mcp_server.rs`
- **Memory backend**: Default es `Vec` (sqlite-vec), configurable via `XAVIER2_MEMORY_BACKEND`
- **Multi-tenant**: Todos los accesos deben pasar `workspace_id` y validar token
- **Async-first**: Tokio runtime, `Arc<RwLock<T>>` para state, `.await` pattern
- **AgentRuntime**: Accesible via `WorkspaceState::runtime`, System 1/2/3 orchestrated
- **Belief Graph**: Validación de consistencia, `SharedBeliefGraph = Arc<RwLock<BeliefGraph>>`

---

*Última actualización: 2026-04-14*
*Auditoría: Revisión integral de robustez agentic + tests + feature status*
*Generado automáticamente por CI Agent Review System*
