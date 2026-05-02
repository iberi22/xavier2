# Architecture Analysis for Issues #84, #90, #91, #93, and #94

Date: 2026-05-02
Repository baseline inspected: GitHub `master` at commit context `3f590edf816d755aa21a82b6d8d4ebf2544d9b25`

## Executive Summary

The codebase is partway through the hexagonal refactor described in issue #90:

- `CliState.memory` already uses `Arc<dyn MemoryQueryPort>` and is backed by `QmdMemoryAdapter`.
- `TimeMetricsPort` and `TimeMetricsAdapter` exist, but `routes.rs` still uses a module-level `OnceLock`.
- `AgentLifecyclePort` exists and `SimpleAgentRegistry` implements it, but `CliState.agent_registry` is still typed as `Arc<SimpleAgentRegistry>`.
- `routes.rs` still has a stateless `session_event_handler`; `cli.rs` has the stateful handler that persists session events.

The target architecture should make HTTP handlers thin inbound adapters, make shared app state contain only port trait objects, and wire concrete adapters once at startup. No handler should rely on module-level service locators, raw infrastructure types, or duplicate route definitions.

## Issue #84: Duplicate `session_event_handler`

### Handlers in `src/adapters/inbound/http/routes.rs`

`routes.rs` currently defines:

- `create_router()` and `create_router_with_agent_registry(...)`
- `health_handler()`
- `session_event_handler(Json<SessionEventRequest>) -> Json<SessionEventResponse>`
- `verify_save_handler(Json<VerifySaveRequest>) -> Json<VerifySaveResponse>`
- `time_metric_handler(Json<TimeMetricDto>) -> Json<TimeMetricResponse>`
- `sync_check_handler() -> Json<SyncCheckResponse>`
- module-level initializers `init_time_store(...)` and `init_health_port(...)`

The `routes.rs` session event handler is stateless. It accepts `SessionEventRequest`, converts the string event type, creates a `SessionEvent`, calls `map_to_panel_thread(event)`, and returns whether mapping succeeded. It does not receive application state and does not store the mapped thread entry or event in memory.

`verify_save_handler`, `time_metric_handler`, and `sync_check_handler` are currently imported by `cli.rs` and used by the active HTTP server. They are not duplicates in current `master`, but they still carry architecture debt because they use globals or raw HTTP-style collaborators.

### Handlers in `src/cli.rs`

`cli.rs` builds the production HTTP server in `start_http_server(...)`. It defines local handlers for health, readiness, memory search/add/query/stats, code graph operations, security scan, session compact/events/timeline, and agent lifecycle endpoints. It imports `time_metric_handler`, `verify_save_handler`, and `sync_check_handler` from `routes.rs`.

The `cli.rs` `session_event_handler` is the functional one. It takes `State<CliState>` plus a typed `SessionEvent`, maps the event to a panel thread entry, builds the path `sessions/{session_id}/thread`, and persists through `state.memory.add(...)` via `MemoryQueryPort`.

### Duplicated Behavior

The active duplicate is:

- `src/adapters/inbound/http/routes.rs::session_event_handler`
- `src/cli.rs::session_event_handler`

Both map session events with `map_to_panel_thread`, but only the `cli.rs` version persists. Keeping both makes `/xavier2/events/session` ambiguous: in one router it means "mapped only", while in the production router it means "mapped and persisted".

### Best Extraction Strategy

Do not consolidate everything into `cli.rs`. That would fix the duplicate narrowly but leave the binary crate owning HTTP behavior. Instead:

1. Move the stateful session event behavior from `cli.rs` into the HTTP inbound adapter module.
2. Delete the stateless `routes.rs::session_event_handler`.
3. Define shared HTTP state in the library crate, for example `src/adapters/inbound/http/state.rs` or `src/app/state.rs`.
4. Make `create_router()` and the binary server use the same route table and same state type.
5. Keep DTOs in `src/adapters/inbound/http/dto.rs` or endpoint-specific DTO modules.

Target shape:

```rust
pub struct AppState {
    pub memory: Arc<dyn MemoryQueryPort>,
    pub security: Arc<dyn InputSecurityPort>,
    pub time_metrics: Arc<dyn TimeMetricsPort>,
    pub agent_lifecycle: Arc<dyn AgentLifecyclePort>,
    pub health: Arc<dyn HealthCheckPort>,
    pub workspace_id: String,
    pub auth_token: String,
}
```

## Issue #90: Hexagonal Architecture Epic

Issue #90 is architecturally correct, but some details are stale on current `master`. The refactor has started, but the inbound path is inconsistent.

Current problems:

- HTTP is split between `src/cli.rs` and `src/adapters/inbound/http/routes.rs`.
- `routes.rs::create_router()` is not the same production route table built in `cli.rs`.
- Memory HTTP handlers mostly use `MemoryQueryPort`, but MCP stdio in `cli.rs` still constructs and calls `QmdMemory` directly.
- Security handlers call `xavier2::security::SecurityService` directly even though `SecurityScanPort` and `src/app/security_service.rs` exist.
- Time metrics has a port but reaches it through a global `OnceLock`.
- Agent lifecycle has a port but app state exposes `SimpleAgentRegistry`.
- Session sync and auto verification still use raw HTTP-style behavior and global cached state.

Required direction:

- HTTP handlers call ports or application services only.
- Startup composes concrete adapters and injects them into state.
- Background tasks use ports, not hardcoded URLs or module-level caches.
- CLI/MCP commands either call the same application ports in process or remain explicit external clients; they should not own separate infrastructure wiring.

## Issue #91: Handlers Bypassing `MemoryQueryPort`

### Current State

Current `master` has partially addressed the issue:

- `src/ports/inbound/memory_port.rs` defines `MemoryQueryPort`.
- `src/app/qmd_memory_adapter.rs` implements `MemoryQueryPort` around `QmdMemory`.
- `CliState.memory` is `Arc<dyn MemoryQueryPort>`.
- `search_handler`, `add_handler`, `memory_query_handler`, and `session_event_handler` use `state.memory`.

Remaining problems:

- `src/app/memory_service.rs` still contains a generic `MemoryService<S, E>` whose methods are all `todo!()`.
- `QmdMemoryAdapter::search` ignores filters and hardcodes a limit of `100`.
- `QmdMemoryAdapter::list` returns an empty vector.
- MCP stdio in `cli.rs` still constructs `QmdMemory` directly and calls `search(...)` and `add_document(...)`.
- HTTP/schema filters and domain `MemoryQueryFilters` are not cleanly unified.

### Refactoring Plan

Files affected:

- `src/ports/inbound/memory_port.rs`
- `src/app/qmd_memory_adapter.rs`
- `src/app/memory_service.rs`
- `src/cli.rs`
- `src/adapters/inbound/http/dto.rs` or a new memory DTO module

Interface needed:

```rust
#[async_trait]
pub trait MemoryQueryPort: Send + Sync {
    async fn search(&self, request: MemorySearchRequest) -> anyhow::Result<MemorySearchResponse>;
    async fn add(&self, request: AddMemoryRequest) -> anyhow::Result<AddMemoryResponse>;
    async fn delete(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>>;
    async fn get(&self, id: &str) -> anyhow::Result<Option<MemoryRecord>>;
    async fn list(&self, request: ListMemoryRequest) -> anyhow::Result<Vec<MemoryRecord>>;
}
```

Ports and adapters:

- Keep `MemoryQueryPort`, but make request objects carry `limit`, `filters`, `namespace`, path, and metadata.
- Keep `QmdMemoryAdapter` as the production adapter around `QmdMemory`.
- Implement filter/limit translation in `QmdMemoryAdapter`.
- Implement `list` fully or remove it from the port until supported.
- Remove, complete, or quarantine `MemoryService<S, E>` so no production path can wire a `todo!()` implementation.

Migration steps:

1. Add domain request/response structs for memory operations.
2. Update `QmdMemoryAdapter` to preserve limit, filters, path, and metadata.
3. Update HTTP handlers to map DTOs to domain request structs.
4. Update MCP stdio to use `Arc<dyn MemoryQueryPort>` from the same composition root.
5. Add handler tests with a fake `MemoryQueryPort`.

## Issue #93: `TimeMetricsStore` and Module-Level `OnceLock`

### Current State

Current `master` has started the extraction:

- `src/ports/inbound/time_metrics_port.rs` defines `TimeMetricsPort`.
- `src/adapters/inbound/http/time_metrics_adapter.rs` wraps `TimeMetricsStore`.
- `src/time/mod.rs` contains SQLite-backed `TimeMetricsStore`.

Remaining problem:

- `src/adapters/inbound/http/routes.rs` still defines `static TIME_STORE: OnceLock<Arc<dyn TimeMetricsPort>>`.
- `init_time_store(...)` is called from `cli.rs` startup.
- `time_metric_handler(...)` has no `State`; it reads the global.
- `CliState.time_store: Option<Arc<TimeMetricsStore>>` remains concrete and is not the handler dependency.
- `workspace_id` is read from `XAVIER2_WORKSPACE_ID` inside the handler instead of state.

### Refactoring Plan

Files affected:

- `src/ports/inbound/time_metrics_port.rs`
- `src/adapters/inbound/http/time_metrics_adapter.rs`
- `src/adapters/inbound/http/routes.rs`
- `src/adapters/inbound/http/state.rs` or `src/app/state.rs`
- `src/cli.rs`
- `src/time/mod.rs`

Interface needed:

```rust
#[async_trait]
pub trait TimeMetricsPort: Send + Sync {
    async fn save_time_metric(&self, metric: TimeMetricRecord, workspace_id: &str) -> anyhow::Result<()>;
    async fn get_metrics(&self, filters: TimeMetricFilters) -> anyhow::Result<Vec<TimeMetricRecord>>;
    async fn get_typical_duration(
        &self,
        provider: Option<&str>,
        model: Option<&str>,
        task_category: Option<&str>,
    ) -> anyhow::Result<Option<TypicalDuration>>;
}
```

Ports and adapters:

- Keep `TimeMetricsPort`, but move it toward domain DTOs rather than HTTP DTOs.
- Keep `TimeMetricsStore` as SQLite infrastructure.
- Move `TimeMetricsAdapter` out of `adapters/inbound/http` because it wraps persistence; better locations are `src/adapters/outbound/sqlite/time_metrics_adapter.rs` or `src/time/adapter.rs`.

Migration steps:

1. Add `time_metrics: Arc<dyn TimeMetricsPort>` to shared `AppState`.
2. Change `time_metric_handler` to take `State<AppState>`.
3. Remove `TIME_STORE` and `init_time_store`.
4. Remove `CliState.time_store` or replace it with the port in shared state.
5. Source `workspace_id` from state.
6. Add a fake `TimeMetricsPort` handler test.

## Issue #94: `SimpleAgentRegistry` Direct Concrete Use

### Current State

Current `master` has partially resolved this issue:

- `src/ports/inbound/agent_lifecycle_port.rs` exists.
- `src/coordination/agent_registry.rs` implements `AgentLifecyclePort for SimpleAgentRegistry`.

Remaining problem:

- `CliState.agent_registry` is still `Arc<SimpleAgentRegistry>`.
- Agent handlers call the concrete registry through state.
- `routes.rs::create_router_with_agent_registry(...)` also takes `Arc<SimpleAgentRegistry>`.

The in-memory `RwLock<HashMap<...>>` is fine as an adapter implementation detail. The problem is that request handlers are typed against it.

### Refactoring Plan

Files affected:

- `src/ports/inbound/agent_lifecycle_port.rs`
- `src/coordination/agent_registry.rs`
- `src/cli.rs`
- `src/adapters/inbound/http/routes.rs`
- `src/agents.rs` if unregister handler state is defined there

Interface needed:

```rust
#[async_trait]
pub trait AgentLifecyclePort: Send + Sync {
    async fn register(&self, agent_id: String, session_id: String, metadata: AgentMetadata) -> bool;
    async fn unregister(&self, agent_id: &str) -> bool;
    async fn heartbeat(&self, agent_id: &str) -> bool;
    async fn get_active_agents(&self) -> Vec<AgentEntry>;
    async fn get(&self, agent_id: &str) -> Option<AgentEntry>;
}
```

Recommended refinements:

- Return `anyhow::Result<...>` or a domain error enum instead of bare `bool` where failures matter.
- Move `AgentEntry` and `AgentMetadata` into `src/domain/agent.rs` so the port does not depend on the concrete registry module.
- Add `list_ids` only if a handler needs it.

Ports and adapters:

- Keep `AgentLifecyclePort`; do not reuse `AgentRuntimePort`.
- Keep `SimpleAgentRegistry` as the production in-memory adapter.
- Future SQLite or distributed registries can implement the same port.

Migration steps:

1. Change `CliState.agent_registry` to `Arc<dyn AgentLifecyclePort>`.
2. Change route builder state to shared `AppState`, not `Arc<SimpleAgentRegistry>`.
3. Update tests to inject a fake `AgentLifecyclePort` or `SimpleAgentRegistry` behind the trait.
4. Move agent domain types if broader reuse is expected.

## Additional Architecture Debt from Issue #90

### Security Port Bypass

`src/ports/inbound/security_port.rs` and `src/app/security_service.rs` exist, but HTTP handlers in `cli.rs` still use `Arc<security::SecurityService>` and call `process_input(...)` directly. The existing `SecurityScanPort` returns scan reports, not the allow/block/sanitize result handlers need before processing memory and code requests.

Create an input-gating port:

```rust
#[async_trait]
pub trait InputSecurityPort: Send + Sync {
    async fn process_input(&self, input: &str) -> anyhow::Result<ProcessedInput>;
    async fn process_output(&self, output: &str) -> anyhow::Result<String>;
}
```

Keep `SecurityScanPort` for explicit `/security/scan` report endpoints. Implement both with an adapter around `security::SecurityService`.

### Session Sync and Auto Verifier

`verify_save_handler` constructs a `reqwest::Client` and calls `AutoVerifier::verify_save(...)`. `SessionSyncTask` is wired from `cli.rs`, and `sync_check_handler` reads cached global state from `session_sync_task`.

Recommended ports:

```rust
#[async_trait]
pub trait VerificationPort: Send + Sync {
    async fn verify_save(&self, path: &str, content: &str) -> anyhow::Result<VerifySaveResult>;
}

#[async_trait]
pub trait SessionSyncPort: Send + Sync {
    async fn check(&self) -> anyhow::Result<SyncCheckResult>;
    async fn last_result(&self) -> SyncCheckResult;
}
```

`HttpVerificationAdapter` can wrap existing `AutoVerifier` behavior initially. `SessionSyncTask` should become an instance-backed adapter/service implementing `SessionSyncPort` instead of exposing module-level cached state.

## Merged Architecture Vision

Target shape:

```text
bin/cli.rs
  - parses CLI
  - builds concrete adapters
  - calls http::router(app_state)
  - starts background tasks

src/adapters/inbound/http/
  - handlers map HTTP DTOs to domain/application requests
  - handlers depend only on AppState port trait objects
  - no global OnceLock
  - no direct SQLite, QmdMemory, SimpleAgentRegistry, or raw reqwest construction

src/app/
  - application services/orchestrators
  - no Axum types

src/ports/inbound/
  - MemoryQueryPort
  - InputSecurityPort / SecurityScanPort
  - TimeMetricsPort
  - AgentLifecyclePort
  - VerificationPort
  - SessionSyncPort

src/adapters/outbound/
  - QmdMemoryAdapter or memory adapter around QmdMemory
  - SqliteTimeMetricsAdapter
  - SimpleAgentRegistry or InMemoryAgentLifecycleAdapter
  - SecurityServiceAdapter
  - HttpHealthAdapter
  - HttpVerificationAdapter
```

Unified state:

```rust
#[derive(Clone)]
pub struct AppState {
    pub memory: Arc<dyn MemoryQueryPort>,
    pub security: Arc<dyn InputSecurityPort>,
    pub security_scan: Arc<dyn SecurityScanPort>,
    pub time_metrics: Arc<dyn TimeMetricsPort>,
    pub agent_lifecycle: Arc<dyn AgentLifecyclePort>,
    pub verification: Arc<dyn VerificationPort>,
    pub session_sync: Arc<dyn SessionSyncPort>,
    pub health: Arc<dyn HealthCheckPort>,
    pub workspace_id: String,
    pub auth_token: String,
}
```

Sequenced plan:

1. Create shared HTTP `AppState` and router builder in `src/adapters/inbound/http`.
2. Move the stateful `session_event_handler` from `cli.rs` to the HTTP adapter and delete the stateless duplicate.
3. Finish memory port semantics: request structs, filter/limit preservation, real list behavior or port pruning, MCP using the same port.
4. Remove `TIME_STORE` and inject `TimeMetricsPort` through state.
5. Change agent handlers and route tests to depend on `Arc<dyn AgentLifecyclePort>`.
6. Add security input-processing port and change handlers to use it.
7. Introduce `VerificationPort` and `SessionSyncPort`; remove raw HTTP construction and global sync result reads from handlers.
8. Move `cli.rs` back to composition and command handling only.

## File-by-File Summary

| File | Current Problem | Target Change |
| --- | --- | --- |
| `src/cli.rs` | Owns production HTTP handlers and concrete-ish state | Keep CLI/bootstrap only; use shared `AppState`; wire ports |
| `src/adapters/inbound/http/routes.rs` | Duplicate stateless session handler; global `OnceLock`; split router | Own single production router and stateful handlers; no globals |
| `src/ports/inbound/memory_port.rs` | Port exists but lacks request semantics for limit/filter | Add request/response structs or complete semantics |
| `src/app/qmd_memory_adapter.rs` | Drops filters, hardcodes limit, empty list | Implement full port contract |
| `src/app/memory_service.rs` | Dead `todo!()` implementation | Remove, complete, or keep out of production wiring |
| `src/ports/inbound/time_metrics_port.rs` | Write-only and tied to HTTP DTO | Move toward domain DTOs; add read methods when needed |
| `src/adapters/inbound/http/time_metrics_adapter.rs` | SQLite wrapper placed under inbound HTTP | Move to outbound/infrastructure adapter location |
| `src/time/mod.rs` | Concrete SQLite store is visible to state | Keep as infrastructure behind `TimeMetricsPort` |
| `src/ports/inbound/agent_lifecycle_port.rs` | Exists, but domain types live in concrete module | Keep; move types to domain if reused |
| `src/coordination/agent_registry.rs` | Good implementation, but handlers still type against it | Keep as adapter behind `AgentLifecyclePort` |
| `src/app/security_service.rs` | Existing scan wrapper does not cover input gating use case | Add `InputSecurityPort` or extend security port design |
| `src/tasks/session_sync_task.rs` | Global cached state and task-specific access pattern | Wrap behind `SessionSyncPort` instance |
| `src/verification/auto_verifier.rs` | Raw HTTP verification utility | Wrap behind `VerificationPort` |

## Recommended Close Criteria

### #84

- Only one `/xavier2/events/session` handler remains.
- The remaining handler has `State<AppState>` and persists through `MemoryQueryPort`.
- `create_router()` and production server use the same handler.

### #91

- All HTTP and MCP memory operations use `MemoryQueryPort`.
- `QmdMemoryAdapter` preserves limit, filters, path/provenance, and metadata.
- No production-wired `todo!()` memory service remains.

### #93

- No `TIME_STORE` static or `init_time_store(...)` exists.
- `time_metric_handler` receives state and calls `TimeMetricsPort`.
- `workspace_id` comes from state.

### #94

- `CliState`/`AppState` stores `Arc<dyn AgentLifecyclePort>`.
- Handler and route tests are typed against the port.
- `SimpleAgentRegistry` remains only as the in-memory adapter implementation.

### #90

- The HTTP inbound layer depends on ports and app services only.
- Binary startup composes adapters once.
- Background tasks and verification use ports instead of raw HTTP/global state.
- Dead or stubbed port implementations are removed or made impossible to wire accidentally.
