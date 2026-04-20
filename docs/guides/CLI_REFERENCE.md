# CLI Reference

## Commands

### `xavier2 http`

Start the HTTP server.

```bash
xavier2 http [OPTIONS]

OPTIONS:
  -p, --port <PORT>  Port to listen on (default: 8006)
```

**Example:**
```bash
xavier2 http --port 8080
```

### `xavier2 mcp`

Start in MCP-stdio mode for AI client integration.

```bash
xavier2 mcp
```

### `xavier2 search`

Search memories by query.

```bash
xavier2 search [OPTIONS] <QUERY>

ARGUMENTS:
  <QUERY>    Search query

OPTIONS:
  -l, --limit <LIMIT>  Maximum results (default: 5, max: 100)
```

**Example:**
```bash
xavier2 search "design decisions"
xavier2 search "architecture" --limit 10
```

### `xavier2 add`

Add a new memory.

```bash
xavier2 add <CONTENT> [OPTIONS]

ARGUMENTS:
  <CONTENT>  Memory content

OPTIONS:
  -t, --title <TITLE>  Memory title
  -p, --path <PATH>    Memory path/path identifier
```

**Example:**
```bash
xavier2 add "Remember to review PRs on Fridays" --title "PR reminder"
xavier2 add "Architecture decision: use SQLite-vec" --path "decisions/001"
```

### `xavier2 stats`

Show memory statistics.

```bash
xavier2 stats
```

### `xavier2 code`

Code graph operations.

```bash
xavier2 code <SUBCOMMAND>

SUBCOMMANDS:
  scan    Scan directory for code symbols
  find    Find code symbols
  stats   Show code graph statistics
```

**Example:**
```bash
xavier2 code scan ./src
xavier2 code find "search_memories"
xavier2 code stats
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `XAVIER2_PORT` | `8006` | HTTP server port |
| `XAVIER2_HOST` | `0.0.0.0` | Bind address |
| `XAVIER2_TOKEN` | `dev-token` | Authentication token |
| `XAVIER2_DEV_MODE` | `false` | Skip auth (dev only) |
| `XAVIER2_LOG_LEVEL` | `info` | Log level |

## Exit Codes

- `0` - Success
- `1` - Error (invalid arguments, network failure, security block, etc.)