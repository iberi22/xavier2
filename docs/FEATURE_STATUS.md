# Xavier2 Feature Status

Current product label: `0.6 beta usable`

This matrix is the operational truth for the repository as of the latest Xavier2 usage review. It is intentionally stricter than roadmap copy: if a feature is not reproducibly usable from the current repo, it is not marked release-ready here.

## Release Status

| Surface | Current status | Notes |
|---|---|---|
| HTTP health/readiness | Beta | `/health` works. `/readiness` responds, but current output still exposes backend noise that should be cleaned before 1.0. |
| HTTP memory add/search/stats | Beta usable | Authenticated memory write and search work in the current server. |
| Canonical runtime config | In progress | `config/xavier2.config.json` is now the intended source of non-secret runtime configuration, with `.env` reserved for credentials and secrets. |
| CLI add/search/stats | Beta usable | These commands currently act as HTTP clients. They require a running server and use `XAVIER2_URL` as the canonical client endpoint, falling back to the JSON config server address. |
| Public Dataset Export | Planned | `xavier2 export --public` is now a core planned feature. It should generate `xavier-dataset/` with manifest, memory, graph, timeline, git, code symbol, code relation, and CK metrics NDJSON files for public agent context. |
| MCP stdio | Beta usable | `initialize`, `tools/list`, `tools/call create_memory`, `tools/call search_memory`, and legacy aliases `add`/`search` work. |
| Panel shell/API | Experimental | Panel routes exist, but the shell requires built frontend assets and is not consistently release-ready in the current repo state. |
| Release smoke scripts | Unstable | Current smoke scripts still assume endpoints and defaults that do not always match the running server. |
| Workspace/storage isolation | Needs hardening | Local usage review showed memory results bleeding into the default workspace instead of staying fully isolated under the intended temporary test setup. |
| Public docs consistency | Needs hardening | README, CLI docs, smoke scripts, and server behavior are not fully aligned yet. |

## What Was Verified

### Confirmed working

- `xavier2 http`
- `POST /memory/add`
- `POST /memory/search`
- `GET /memory/stats`
- `GET /health`
- auth gate on protected routes
- `xavier2 mcp`
- MCP `tools/list`
- MCP `tools/call` for `add` and `search`

### Confirmed not 1.0-ready

- CLI commands do not behave like a purely embedded local memory tool; they depend on the HTTP server.
- `scripts/release-smoke.ps1` expects `/build`, but the tested server path returned `404`.
- Panel routes require built frontend assets and are not yet a complete release-ready surface.
- The current repo still contains insecure or stale references in scripts and docs that a public 1.0 release should not carry.
- The codebase still has many direct `std::env::var(...)` reads that need to finish migrating behind the canonical JSON config contract.

## Definition Of `1.0`

Xavier2 should only be labeled `1.0` when all of the following are true:

- one canonical server contract exists for CLI, HTTP, MCP, panel, and smoke scripts
- token and port behavior are documented and consistent
- release smoke passes without manual patching
- workspace and storage isolation are reproducible
- panel build and route expectations are either stable or clearly scoped out
- public dataset export emits reproducible read-only context with documented schema versions
- public docs describe the real product surface, not the aspirational one
- remaining `dev-token` and insecure-default references are removed from production-facing surfaces
