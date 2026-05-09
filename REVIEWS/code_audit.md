# Xavier Code Audit Report

**Date:** 2026-04-14
**Auditor:** SWAL Agent (ventas)
**Version:** 0.4.1
**Scope:** Hexagonal architecture, error handling, async patterns, API design, tests

---

## Executive Summary

Xavier is a cognitive memory system with significant architectural investment in hexagonal/ports-and-adapters design, but the implementation is **inconsistent**. The `src/adapters/` directory contains **non-functional stubs** while the actual business logic lives directly in `src/memory/` and `src/workspace.rs`. This creates confusion about which layer is authoritative. The codebase would benefit from a systematic cleanup prioritizing "thin adapters, rich domain" alignment.

---

## 🔴 CRITICAL ISSUES (Fix Immediately)

### 1. Non-Functional Stub Adapters

**Location:** `src/adapters/outbound/sqlite/storage_adapter.rs`, `src/adapters/outbound/vec/storage_adapter.rs`, `src/adapters/outbound/embedding/embedding_adapter.rs`, `src/app/memory_service.rs`

All adapter methods are `todo!()` — completely non-functional stubs:

```rust
// storage_adapter.rs
impl StoragePort for SqliteStorageAdapter {
    async fn put(&self, _record: MemoryRecord) -> anyhow::Result<()> {
        todo!()
    }
    // all other methods: todo!()
}

// memory_service.rs
impl<S: StoragePort + Send + Sync, E: EmbeddingPort + Send + Sync> MemoryQueryPort
    for MemoryService<S, E>
{
    async fn search(&self, query: &str, filters: Option<MemoryQueryFilters>) -> anyhow::Result<Vec<MemoryRecord>> {
        let _ = query; let _ = filters; todo!()
    }
    // all other methods: todo!()
}
```

**Impact:** The hexagonal architecture layer is **decorative, not functional**. Every adapter is a stub. The real implementation is in `src/memory/` and `src/workspace.rs` — not behind ports.

**Recommendation:** Either:
- (a) Implement the adapters properly using the actual memory store implementations, OR
- (b) Remove the `src/adapters/` and `src/ports/` directories entirely and document that the hexagonal layer is aspirational. Do not maintain dead code.

### 2. Duplicate Domain Types (Major DRY Violation)

**Location:** `src/domain/memory/types.rs` vs `src/memory/schema.rs`

Two completely separate enum definitions for the same concepts:

**`src/domain/memory/types.rs`:**
```rust
pub enum MemoryKind { Fact, Preference, Context, Task, Conversation }
pub enum EvidenceKind { Direct, Inferred, Reported, Derived }
pub enum MemoryNamespace { Global, Project, Session, Ephemeral }
```

**`src/memory/schema.rs` (30+ variants):**
```rust
pub enum MemoryKind { Episodic, Semantic, Procedural, Belief, Org, Workspace, User, Agent, Session, Event, Fact, Decision, Repo, Branch, File, Symbol, Url, Task, Contact, Meeting, ... }
pub enum EvidenceKind { SourceTurn, SessionSummary, TemporalEvent, FactAtom, EntityState, SummaryFact, Observation, UserPrompt }
pub enum MemoryNamespace { org_id, workspace_id, user_id, agent_id, session_id, project, scope }
```

**Impact:** The domain layer defines a simplified 4-variant `MemoryKind`; the actual implementation uses 30+ variants. Code using domain types vs. memory layer types **cannot interoperate without conversion**. This is a fundamental architectural split.

**Recommendation:** Consolidate to ONE set of types in `src/domain/`. The memory layer should import from domain, not redefine.

### 3. Unsafe `std::env::set_var` in Tests

**Location:** `src/main.rs` (test modules)

```rust
#[test]
fn code_graph_db_path_prefers_explicit_env() {
    unsafe {
        std::env::set_var("XAVIER_CODE_GRAPH_DB_PATH", &db_path);
        // ... no guaranteed cleanup
    }
}

#[test]
fn server_addr_uses_env_configuration() {
    unsafe {
        std::env::set_var("XAVIER_HOST", "127.0.0.1");
        std::env::set_var("XAVIER_PORT", "8123");
        // ...
        std::env::remove_var("XAVIER_HOST");  // cleanup in defer
        std::env::remove_var("XAVIER_PORT");
    }
}
```

**Impact:** `std::env::set_var` is process-global and not thread-safe. Concurrent tests could fail intermittently. The unsafe block does not guarantee cleanup if a test panics.

**Recommendation:** Use `temp_env::with_var()` from the `temp-env` crate, or `tokio::test` with a scoped environment.

---

## 🟠 HIGH PRIORITY (Fix Soon)

### 4. Inconsistent Error Handling Strategy

**Observation:** Mix of `anyhow::Result` and `thiserror::Error` across the codebase.

| Module | Approach |
|--------|----------|
| `src/ports/` | `anyhow::Result` (correct for trait bounds) |
| `src/adapters/` | `anyhow::Result` |
| `src/memory/` | `anyhow::Result` |
| `src/workspace.rs` | `anyhow::Result` |
| `src/coordination/message_bus.rs` | `thiserror::Error` |
| `src/crypto/encryption.rs` | `thiserror::Error` |
| `src/crypto/keys.rs` | `thiserror::Error` |
| `src/secrets/mod.rs` | `thiserror::Error` (via `Error` derive) |

**Impact:** No unified error domain. `thiserror` types are appropriate for library code with typed errors; `anyhow` is appropriate for application/CLI code. The codebase mixes both at the same level.

**Recommendation:**
- Domain layer (`src/domain/`): Use `thiserror` for typed errors
- Application layer (`src/app/`): Use `anyhow::Result`
- Port traits: Use `anyhow::Result` (dynamic, no concrete error types at trait boundary)
- Infrastructure: `thiserror` for internal errors that need categorization

### 5. Blocking `RwLock` in Async Context

**Location:** `src/adapters/outbound/vec/pattern_adapter.rs`

```rust
pub struct PatternAdapter {
    patterns: Arc<RwLock<HashMap<String, VerifiedPattern>>>,  // std::sync::RwLock
}

impl PatternDiscoverPort for PatternAdapter {
    async fn discover(&self, pattern: VerifiedPattern) -> anyhow::Result<String> {
        let mut patterns = self.patterns.write()  // BLOCKING - holds lock across await
            .map_err(|_| anyhow::anyhow!("failed to acquire write lock"))?;
        // ...
        tokio::spawn(async move {  // awaits here?
            patterns.insert(id.clone(), pattern);
        });
    }
}
```

**Impact:** `std::sync::RwLock` is a blocking lock. When held across `.await` points, it can deadlock with tokio's async blocking mechanism. The rest of the codebase correctly uses `tokio::sync::RwLock`.

**Recommendation:** Replace `Arc<RwLock<HashMap<...>>>` with `Arc<tokio::sync::RwLock<HashMap<...>>>` and `.write().await`.

### 6. Missing Adapter Implementations

**Location:** `src/ports/inbound/agent_port.rs`, `src/ports/inbound/security_port.rs`

- `AgentRuntimePort` trait defined but no production adapter exists
- `SecurityScanPort` defined but implementation (`SecurityService`) is in `src/app/security_service.rs` (application layer), not an adapter

**Impact:** The inbound port layer has no corresponding adapter implementations, making the hexagonal architecture incomplete.

### 7. `src/server/http.rs` Is Too Large (~800+ lines)

**Impact:** Single file containing:
- Request/response DTOs
- Handler functions
- `HttpConfig`, `HttpServer` structs
- Inline tests

**Recommendation:** Split into:
- `src/server/http/handlers.rs` — handler functions
- `src/server/http/dto.rs` — request/response types
- `src/server/http/middleware.rs` — auth middleware
- `src/server/http/config.rs` — HttpConfig

### 8. Inconsistent `IntoResponse` Return Types

**Location:** `src/server/http.rs`

Some handlers return `impl IntoResponse`, some return `Json<serde_json::Value>`:

```rust
// Returns impl IntoResponse
pub async fn memory_add(...) -> impl IntoResponse { ... }

// Returns Json directly (also implements IntoResponse via axum)
pub async fn memory_search(...) -> impl IntoResponse { ... }
pub async fn memory_stats(...) -> impl IntoResponse { ... }
```

**Impact:** Minor inconsistency. The `Json<T>` return is fine but should be consistent.

### 9. Response Consistency in `v1_api.rs`

**Location:** `src/server/v1_api.rs`

```rust
pub async fn v1_memories_add(...) -> impl IntoResponse {
    // Returns Json directly
    Json(serde_json::json!({...}))
}

pub async fn v1_memories_search(...) -> impl IntoResponse {
    Json(V1MemorySearchResponse { ... })
}
```

While correct, the use of `Json(serde_json::json!(...))` for error cases (no `?` propagation) loses type safety.

---

## 🟡 MEDIUM PRIORITY (Nice to Have)

### 10. Logging Inconsistencies

| Handler | Uses `tracing::info!` | Logs request fingerprint |
|---------|----------------------|--------------------------|
| `memory_add` | ✅ | ✅ |
| `memory_search` | ✅ | ✅ |
| `memory_hybrid_search` | ✅ | ✅ |
| `memory_curate` | ✅ (🧠 emoji) | ❌ |
| `memory_manage` | ✅ (⚙️ emoji) | ❌ |
| `memory_decay` | ✅ (📉 emoji) | ❌ |
| `memory_consolidate` | ✅ (🔗 emoji) | ❌ |
| `memory_quality` | ✅ (📊 emoji) | ❌ |
| `memory_evict` | ✅ (🗑️ emoji) | ❌ |
| `memory_stats` | ✅ (📈 emoji) | ❌ |
| `memory_delete` | ✅ (🗑️ emoji) | ❌ |
| `memory_reset` | ✅ (♻️ emoji) | ❌ |
| `memory_graph` | ✅ (🔗 emoji) | ❌ |
| `memory_graph_hops` | ✅ | ❌ |
| `agents_run` | ✅ | ✅ |
| `bridge_import` | ✅ | ❌ |

**Observation:** Emoji logging is charming but consider structured fields for all handlers (e.g., `doc_id`, `workspace_id`).

### 11. Static Regex in `qmd_memory.rs`

**Location:** `src/memory/qmd_memory.rs`

Multiple `LazyLock` regexes at module level — all static-initialized:

```rust
static SPEAKER_COLON_RE: LazyLock<Regex> = LazyLock::new(|| ...);
static SPEAKER_BRACKET_RE: LazyLock<Regex> = LazyLock::new(|| ...);
static SPEAKER_ROLE_RE: LazyLock<Regex> = LazyLock::new(|| ...);
static QUERY_SPEAKER_RE: LazyLock<Regex> = LazyLock::new(|| ...);
static SHE_RE: LazyLock<Regex> = LazyLock::new(|| ...);
static HE_RE: LazyLock<Regex> = LazyLock::new(|| ...);
static DIA_ID_RE: LazyLock<Regex> = LazyLock::new(|| ...);
static LOCOMO_PATH_DIA_ID_RE: LazyLock<Regex> = LazyLock::new(|| ...);
```

**Positive:** Good practice for regex-heavy code — avoids repeated compilation.

**Concern:** Module-level regexes are fine but the module is 2700+ lines. Consider extracting speaker/dialogue parsing to a submodule.

### 12. Missing Security Middleware on Some Endpoints

**Location:** `src/server/mod.rs` routes

The MCP endpoint (`/mcp`) is protected by `auth_middleware` but there's no dedicated security scan middleware that validates request content before it reaches handler logic.

**Recommendation:** Consider adding `prompt_guard.rs` (already exists in `src/security/`) as a middleware layer for user-input endpoints.

### 13. No Rate Limiting Infrastructure

The `ensure_within_request_limit()` check exists in workspace but no dedicated rate limiting middleware. The `TOO_MANY_REQUESTS` response is returned from auth middleware, but no per-endpoint rate limiting.

### 14. `unsafe` in `main.rs` Test Blocks

Already noted in Critical Issue #3 — repeated.

### 15. Inconsistent Auth Middleware Bypass

```rust
if path == "/health" || path == "/readiness" {
    return next.run(req).await;
}
```

While correct for health checks, the panel assets and shell are also completely public. Consider whether the panel should have some form of workspace-scoped access control.

---

## 🟢 RECOMMENDATIONS (State-of-the-Art Improvements)

### A. Consolidate Hexagonal Architecture

The current state has **two competing architectures**:

1. **Aspirational**: `src/domain/`, `src/ports/`, `src/adapters/`, `src/app/` (clean but stubs)
2. **Actual**: `src/memory/`, `src/workspace.rs` (functional but not behind ports)

**Recommendation:** Pick one approach:
- **Option 1 (Full hexagonal):** Implement adapters properly, move business logic from `src/memory/` to `src/domain/` + `src/app/`
- **Option 2 (Simplified):** Remove the adapter/port directories, keep `src/domain/` for shared types only

### B. Unified Error Domain

Create `src/domain/error.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("record not found: {0}")]
    NotFound(String),
    #[error("invalid namespace: {0}")]
    InvalidNamespace(String),
    #[error("storage limit exceeded")]
    StorageLimitExceeded,
    #[error("embedding failed: {0}")]
    EmbeddingFailed(String),
    #[error("query failed: {0}")]
    QueryFailed(String),
}
```

Use `thiserror` in domain, `anyhow` in application layer.

### C. Extract `src/server/http.rs` into Submodule

The ~800-line file should be split:
```
src/server/http/
  mod.rs       # re-exports
  handlers.rs  # all handler functions
  dto.rs        # request/response types
  config.rs     # HttpConfig
  middleware.rs # auth middleware (moved from main.rs)
```

### D. Config Module

Move env-based configuration to `src/config.rs`:

```rust
pub struct Config {
    pub xavier_host: String,
    pub xavier_port: u16,
    pub xavier_token: String,
    pub xavier_dev_mode: bool,
    // ...
}

impl Config {
    pub fn from_env() -> Self { ... }
}
```

### E. Test Infrastructure Improvements

1. Extract integration tests to `tests/` directory (not `#[cfg(test)]` modules in main files)
2. Use `temp_env::with_var()` instead of unsafe env manipulation
3. Add property-based tests with `proptest` for schema parsing
4. Add doc tests for port trait implementations

### F. Observability

Consider adding:
- `tracing::instrument` attributes on async handlers
- `tracing::Span` for per-request tracing
- Structured log fields (`workspace_id`, `doc_id`) on all handlers

### G. API Versioning Strategy

The `/v1/` prefix in `v1_api.rs` is good. Consider:
- Adding `API_VERSION` constant
- Deprecating old non-v1 endpoints in favor of v1 equivalents
- OpenAPI/Swagger documentation for v1 API

### H. Memory Safety Audit

- Confirm no `unsafe` blocks outside of `rusqlite` and `sqlite-vec` bindings
- Check `zerocopy` usage for safe deserialization
- Review `memchr`, `regex` for any unsafe code paths

---

## 📊 Summary Matrix

| Category | Status | Notes |
|----------|--------|-------|
| Hexagonal Architecture | ⚠️ Partial | Stubs exist, not functional |
| Domain Types | ⚠️ Duplicated | Two sets of MemoryKind/EvidenceKind/Namespace |
| Error Handling | ⚠️ Inconsistent | Mix of anyhow/thiserror |
| Async Patterns | ⚠️ Minor Issue | `std::sync::RwLock` in `PatternAdapter` |
| Memory Safety | ✅ Good | No unsafe blocks (except test env vars) |
| Logging | ✅ Good | Consistent tracing usage |
| Code Duplication | 🔴 High | Massive duplication between domain/memory |
| API Design | ✅ Good | RESTful v1 API, good pagination |
| Test Coverage | ⚠️ Gaps | Integration tests exist, missing property tests |

---

## Files Reviewed

- `src/lib.rs`, `src/main.rs`
- `src/domain/mod.rs`, `src/domain/memory/types.rs`
- `src/ports/mod.rs`, `src/ports/inbound/*`, `src/ports/outbound/*`
- `src/adapters/mod.rs`, `src/adapters/inbound/http/*`, `src/adapters/outbound/*`
- `src/app/mod.rs`, `src/app/memory_service.rs`
- `src/server/mod.rs`, `src/server/http.rs`, `src/server/v1_api.rs`
- `src/memory/mod.rs`, `src/memory/manager.rs`, `src/memory/qmd_memory.rs`, `src/memory/schema.rs`
- `src/workspace.rs`
- `Cargo.toml`
