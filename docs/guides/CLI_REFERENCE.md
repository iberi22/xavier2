# CLI Reference

This reference describes the CLI that is currently present in the repository.

## Command Model

The current CLI is split into two modes:

- server commands: start Xavier2 services
- client commands: call the running HTTP service

That means `add`, `search`, and `stats` are not purely embedded local-memory commands today. They require a running Xavier2 HTTP server and a valid `XAVIER2_TOKEN`.

## Commands

### `xavier2 http`

Starts the HTTP server.

```bash
xavier2 http
xavier2 http 8006
```

### `xavier2 mcp`

Starts the MCP stdio server.

```bash
xavier2 mcp
```

### `xavier2 search <QUERY> [LIMIT]`

Searches memories through the running HTTP service.

```bash
xavier2 search "design decisions"
xavier2 search "release readiness" 10
```

### `xavier2 add <CONTENT> [TITLE]`

Adds a memory through the running HTTP service.

```bash
xavier2 add "Remember to review PRs on Fridays" "PR reminder"
xavier2 add "Architecture decision: use SQLite-vec" "adr-001"
```

### `xavier2 stats`

Fetches current memory statistics through the running HTTP service.

```bash
xavier2 stats
```

### `xavier2 session-save`

Saves current session context to Xavier2.

### `xavier2 spawn`

Spawns multiple agents with provider routing.

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `XAVIER2_URL` | derived from `config/xavier2.config.json` | Canonical client base URL for `add`, `search`, `stats`, and session save operations |
| `XAVIER2_PORT` | `8006` | HTTP bind port for `xavier2 http`; also used to derive the client URL when `XAVIER2_URL` is unset |
| `XAVIER2_HOST` | `0.0.0.0` | Bind address for `xavier2 http` |
| `XAVIER2_TOKEN` | required for client commands | Auth token |
| `XAVIER2_DEV_MODE` | `false` | Development-only mode that permits an auto-generated token when starting `xavier2 http` without `XAVIER2_TOKEN` |
| `XAVIER2_LOG_LEVEL` | `info` | Log level |

## Current Gaps

- CLI docs from older revisions referenced flags like `--title`, `--path`, and `code` subcommands that are not part of the current help output.
- The current CLI still behaves like an HTTP client for memory operations.
- The runtime still contains older environment-based compatibility paths internally while the JSON config rollout is completed.
