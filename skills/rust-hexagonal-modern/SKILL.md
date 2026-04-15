---
name: rust-hexagonal-modern
description: Advanced Rust architecture and crate-guidance skill for hexagonal or ports-and-adapters systems. Use when Codex is designing, reviewing, refactoring, or implementing Rust backends and CLIs with crates such as `tokio`, `axum`, `tower-http`, `reqwest`, `serde`, `thiserror`, `tracing`, `clap`, `ratatui`, `rusqlite`, or `parking_lot`, especially for "arquitectura hexagonal", "puertos y adaptadores", package modernization, async correctness, or workspace-scale code quality improvements.
---

# Rust Hexagonal Modern

Start by grounding the change in the actual workspace, not in a generic Rust template.

## Workflow

1. Read `Cargo.toml`, the nearest Cargo workspace manifests, and the repo architecture authority.
2. Run `python skills/rust-hexagonal-modern/scripts/inventory_workspace.py .` from the repo root to see which crates and versions are actually in play.
3. Read [references/crate-radar.md](references/crate-radar.md) when the task involves crate selection, upgrades, or package modernization.
4. Read [references/hexagonal-playbook.md](references/hexagonal-playbook.md) when the task involves layering, ports, adapters, or refactors.
5. Read [references/review-checklist.md](references/review-checklist.md) when reviewing a PR or tightening an existing Rust module.

## Non-negotiable rules

- Keep the domain layer free of `axum`, `reqwest`, `rusqlite`, file-system details, HTTP DTOs, and lock types.
- Put transport and persistence DTOs at the adapter boundary, then map them into domain types or application commands.
- Use `thiserror` for domain and adapter error enums. Reserve `anyhow` for binaries, orchestration shells, and test glue.
- Build shared services at the composition root with `Arc`, then inject ports into use cases. Do not hide mutable globals behind convenience APIs.
- Reuse `reqwest::Client`, DB handles, and shared state. Do not recreate them per request.
- Prefer structured async ownership and explicit shutdown paths over detached `tokio::spawn` tasks with implicit lifetimes.
- Keep HTTP handlers thin: parse input, call a use case, map output to response.

## Crate-specific defaults

- `axum`: prefer `State<AppState>`, typed extractors, `Router::with_state`, and centralized error-to-response mapping.
- `tokio`: use `tokio::sync` primitives for async coordination, `timeout` around remote calls, and `spawn_blocking` for blocking adapters.
- `tower-http`: keep tracing, compression, CORS, and request-ID middleware at the HTTP adapter layer.
- `reqwest`: create one client per app or bounded subsystem, set timeouts and user agent, and deserialize into transport DTOs.
- `serde`: serialize and deserialize transport models, not rich domain behavior. Use `rename_all` deliberately and strict parsing where contracts matter.
- `clap`: treat the CLI as an inbound adapter that converts command-line input into application commands.
- `rusqlite`: isolate synchronous database work inside a storage adapter; do not leak `Connection` or transaction details upward.
- `ratatui`: render from view models and UI state, not from business logic that mutates domain services directly.

## Output expectations

When using this skill for a substantial change, produce these artifacts in the code or explanation:

- Target layer placement for each new type or module
- Port and adapter boundaries for outbound dependencies
- Error-boundary plan
- Test plan split into domain, port-contract, adapter, and end-to-end coverage
