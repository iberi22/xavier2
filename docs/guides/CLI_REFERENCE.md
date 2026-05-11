# CLI Reference

This reference describes the CLI that is currently present in the repository.

## Command Model

The current CLI is split into two modes:

- server commands: start Xavier services
- client commands: call the running HTTP service

That means `add`, `search`, and `stats` are not purely embedded local-memory commands today. They require a running Xavier HTTP server and a valid `XAVIER_TOKEN`.

## Commands

### `xavier http`

Starts the HTTP server.

```bash
xavier http
xavier http 8006
```

### `xavier mcp`

Starts the MCP stdio server.

```bash
xavier mcp
```

### `xavier search <QUERY> [LIMIT]`

Searches memories through the running HTTP service.

```bash
xavier search "design decisions"
xavier search "release readiness" 10
```

### `xavier add <CONTENT> [TITLE]`

Adds a memory through the running HTTP service.

```bash
xavier add "Remember to review PRs on Fridays" "PR reminder"
xavier add "Architecture decision: use SQLite-vec" "adr-001"
```

### `xavier stats`

Fetches current memory statistics through the running HTTP service.

```bash
xavier stats
```

### `xavier session-save`

Saves current session context to Xavier.

### `xavier spawn`

Spawns multiple agents with provider routing.

---

## Standalone Context Commands

These commands operate **without a running server**. They work directly on a local SQLite database.

### `xavier init`

Initializes a `.xavier/` directory in the current project.

```bash
xavier init
```

### `xavier index [--path <DIR>] [--output <DB>]`

Scans the project directory, indexes `.md` and `.rs` files into a local memory database, then runs the HCE engine to build the hierarchical context tree.

```bash
xavier index                          # indexes current directory -> .xavier/memory.db
xavier index --path ./my-project
xavier index --path . --output .xavier/memory.db
```

### `xavier export --workspace <ID> [--output <FILE>] [--format tree|db]`

Exports workspace memories to a portable artifact.

- `--format db` (default): exports a SQLite snapshot via `VACUUM INTO`
- `--format tree`: exports a hierarchical `context-tree.json` for agent context loading

```bash
xavier export --workspace myproject --format tree --output context-tree.json
xavier export --workspace myproject --format db   --output backup.db
```

### `xavier import --file <DB> [--workspace <ID>]`

Non-destructively merges an external Xavier database into the local store via `ATTACH DATABASE`. Does not overwrite existing records.

```bash
xavier import --file team-brain.db --workspace shared
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `XAVIER_URL` | derived from `config/xavier.config.json` | Canonical client base URL for `add`, `search`, `stats`, and session save operations |
| `XAVIER_PORT` | `8006` | HTTP bind port for `xavier http`; also used to derive the client URL when `XAVIER_URL` is unset |
| `XAVIER_HOST` | `0.0.0.0` | Bind address for `xavier http` |
| `XAVIER_TOKEN` | required for client commands | Auth token |
| `XAVIER_DEV_MODE` | `false` | Development-only mode that permits an auto-generated token when starting `xavier http` without `XAVIER_TOKEN` |
| `XAVIER_LOG_LEVEL` | `info` | Log level |

## Current Gaps

- CLI docs from older revisions referenced flags like `--title`, `--path`, and `code` subcommands that are not part of the current help output.
- The current CLI still behaves like an HTTP client for memory operations.
- The runtime still contains older environment-based compatibility paths internally while the JSON config rollout is completed.
