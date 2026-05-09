---
name: xavier-http-curl
description: Use Xavier over authenticated HTTP with curl instead of MCP. Use when an agent needs to add, search, query, delete, reset, or inspect Xavier memory or code-index data through `curl`, Bash, or PowerShell against a running Xavier server.
---

# Xavier HTTP Curl

Use the HTTP API as the default integration path for external agents.

## Quick start

Set connection variables first.

### Bash

```bash
export XAVIER_URL="${XAVIER_URL:-http://localhost:8003}"
export XAVIER_TOKEN="${XAVIER_TOKEN:-dev-token}"
```

### PowerShell

```powershell
$env:XAVIER_URL = $env:XAVIER_URL ?? "http://localhost:8003"
$env:XAVIER_TOKEN = $env:XAVIER_TOKEN ?? "dev-token"
```

All endpoints except `GET /health` require `X-Xavier-Token`.

## Core recipes

Check health:

```bash
curl "$XAVIER_URL/health"
```

Add memory:

```bash
curl -X POST "$XAVIER_URL/memory/add" \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "path": "agent/session-notes",
    "content": "Xavier can be used without MCP through HTTP.",
    "metadata": {"source": "curl-skill"}
  }'
```

Search memory:

```bash
curl -X POST "$XAVIER_URL/memory/search" \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"without MCP","limit":5}'
```

Query runtime:

```bash
curl -X POST "$XAVIER_URL/memory/query" \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"What does Xavier expose?","limit":5}'
```

Delete memory:

```bash
curl -X POST "$XAVIER_URL/memory/delete" \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path":"agent/session-notes"}'
```

Reset memory:

```bash
curl -X POST "$XAVIER_URL/memory/reset" \
  -H "X-Xavier-Token: $XAVIER_TOKEN"
```

Scan code:

```bash
curl -X POST "$XAVIER_URL/code/scan" \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path":"E:/scripts-python/xavier"}'
```

Find symbols:

```bash
curl -X POST "$XAVIER_URL/code/find" \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"AgentRuntime","limit":10,"kind":"struct"}'
```

Inspect index stats:

```bash
curl -H "X-Xavier-Token: $XAVIER_TOKEN" \
  "$XAVIER_URL/code/stats"
```

## Working rules

- Prefer HTTP over MCP for external automation.
- Use `X-Xavier-Token`, not `Authorization: Bearer`.
- Use only implemented routes. Do not call legacy REST paths like `/memory/` or `/memory/{id}`.
- Read [references/http-api.md](references/http-api.md) if you need exact payload shapes and common responses.
- For PowerShell `curl` compatibility issues, use `Invoke-RestMethod` with the same headers and JSON body.

## Troubleshooting

- `401 Unauthorized`: token missing or wrong; verify `XAVIER_TOKEN`.
- `404 Not Found`: you likely used an old route shape; switch to `/memory/add`, `/memory/delete`, or `/code/*`.
- Empty search results after add: confirm the `path` and query terms, then retry with a broader query.
- Code search returns nothing: run `/code/scan` first for the target tree.
