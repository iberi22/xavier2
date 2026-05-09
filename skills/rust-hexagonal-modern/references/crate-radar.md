# Crate Radar

Use this reference when choosing crates, reviewing dependency drift, or tightening coding patterns around the Rust stack already present in the workspace.

## Upstream snapshot

Verified against docs.rs latest pages on 2026-03-29 for the crates listed below.

| Crate | Repo manifest | Upstream snapshot | Guidance |
|---|---|---|---|
| `axum` | `0.8` | `0.8.8` | Stay on the `0.8` line. Favor typed extractors, `State`, and response mapping at the HTTP edge. |
| `tokio` | `1.50.0` or `1` | `1.50.0` | Good baseline. `full` is acceptable for binaries; narrow features for reusable libraries. |
| `reqwest` | `0.12` | `0.13.2` | The workspace is behind the latest major line. Upgrade intentionally, not casually, when touching HTTP client code. |
| `tower-http` | `0.6` | `0.6.8` | Keep middleware in adapters. Prefer `TraceLayer` and compression over custom wrapper code. |
| `serde` | `1.0.228` or `1` | `1.0.228` | Stable choice. Keep DTOs separate from domain entities. |
| `thiserror` | `2.0.18` or `2` | `2.0.18` | Use for boundary-specific error enums and explicit conversion chains. |
| `clap` | `4` | `4.6.0` | Stay on derive-first CLI definitions and parse into application commands. |

## Practical defaults

### `axum`

- Use `State<Arc<AppState>>` instead of global statics or request extensions by default.
- Keep handlers as translation layers from HTTP to use-case calls.
- Centralize HTTP error mapping so domain and application errors do not know status codes.

### `tokio`

- Treat `tokio::spawn` as an ownership decision, not as a shortcut.
- Wrap remote I/O in `tokio::time::timeout`.
- Move blocking adapters behind `spawn_blocking` or a dedicated blocking boundary.
- Prefer channels, `RwLock`, `Mutex`, or `JoinSet` only where concurrency is real; do not add async coordination primitives as decoration.

### `reqwest`

- Reuse a single `Client` to benefit from connection pooling.
- Set explicit timeouts, headers, and retry behavior in one construction point.
- Decode into transport structs, then map to domain data.
- Check status codes before assuming successful deserialization.

### `tower-http` and `tracing`

- Put request tracing, CORS, compression, and request IDs in middleware, not in handlers.
- Emit structured fields with `tracing`, not interpolated log strings.
- Instrument use cases and adapter calls that help explain latency or failures.

### `serde`

- Use `#[serde(rename_all = \"...\")]` for external contracts only when the contract demands it.
- Consider `deny_unknown_fields` for external request DTOs that should fail fast on drift.
- Avoid deriving `Serialize` and `Deserialize` on every domain object by habit.

### `thiserror` and `anyhow`

- Use `thiserror` inside reusable modules to preserve error shape.
- Use `anyhow` at binary edges, test setup, and top-level orchestration where call-site context matters more than a stable enum.
- Keep conversion paths explicit with `#[from]` only when the semantic mapping is obvious.

### `clap`

- Parse command-line input into adapter DTOs, then hand off to application services.
- Keep side effects out of parser types.
- Split large CLIs into subcommands instead of boolean-flag matrices.

### `rusqlite` and `parking_lot`

- Hide sync storage and lock choices inside adapters.
- Never let a domain service require a specific lock or connection type.
- If lock contention shows up in reviews, move coordination to a smaller critical section before changing the architecture.

## Upgrade heuristics

- Patch and minor line updates are default maintenance work.
- Major upgrades need one concrete payoff: security, correctness, simpler code, performance, or unblocking another dependency.
- Run tests and at least one smoke path after crate upgrades. Avoid speculative churn.
