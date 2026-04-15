---
name: xavier2-http-curl
description: Use Xavier2 over authenticated HTTP with curl instead of MCP. Use when an agent needs to add, search, query, delete, reset, or inspect Xavier2 memory or code-index data through `curl`, Bash, or PowerShell against a running Xavier2 server.
---

# Xavier2 HTTP Curl

Use the HTTP API as the default integration path for external agents.

## Quick start

Set connection variables first.

### Bash

```bash
export XAVIER2_URL="${XAVIER2_URL:-http://localhost:8003}"
export XAVIER2_TOKEN="${XAVIER2_TOKEN:-dev-token}"
```

### PowerShell

```powershell
$env:XAVIER2_URL = $env:XAVIER2_URL ?? "http://localhost:8003"
$env:XAVIER2_TOKEN = $env:XAVIER2_TOKEN ?? "dev-token"
```

All endpoints except `GET /health` require `X-Xavier2-Token`.

## Core recipes

Check health:

```bash
curl "$XAVIER2_URL/health"
```

Add memory:

```bash
curl -X POST "$XAVIER2_URL/memory/add" \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "path": "agent/session-notes",
    "content": "Xavier2 can be used without MCP through HTTP.",
    "metadata": {"source": "curl-skill"}
  }'
```

Search memory:

```bash
curl -X POST "$XAVIER2_URL/memory/search" \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"without MCP","limit":5}'
```

Query runtime:

```bash
curl -X POST "$XAVIER2_URL/memory/query" \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"What does Xavier2 expose?","limit":5}'
```

Delete memory:

```bash
curl -X POST "$XAVIER2_URL/memory/delete" \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path":"agent/session-notes"}'
```

Reset memory:

```bash
curl -X POST "$XAVIER2_URL/memory/reset" \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN"
```

Scan code:

```bash
curl -X POST "$XAVIER2_URL/code/scan" \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path":"E:/scripts-python/xavier2"}'
```

Find symbols:

```bash
curl -X POST "$XAVIER2_URL/code/find" \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"AgentRuntime","limit":10,"kind":"struct"}'
```

Inspect index stats:

```bash
curl -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  "$XAVIER2_URL/code/stats"
```

## Working rules

- Prefer HTTP over MCP for external automation.
- Use `X-Xavier2-Token`, not `Authorization: Bearer`.
- Use only implemented routes. Do not call legacy REST paths like `/memory/` or `/memory/{id}`.
- Read [references/http-api.md](references/http-api.md) if you need exact payload shapes and common responses.
- For PowerShell `curl` compatibility issues, use `Invoke-RestMethod` with the same headers and JSON body.

## Troubleshooting

- `401 Unauthorized`: token missing or wrong; verify `XAVIER2_TOKEN`.
- `404 Not Found`: you likely used an old route shape; switch to `/memory/add`, `/memory/delete`, or `/code/*`.
- Empty search results after add: confirm the `path` and query terms, then retry with a broader query.
- Code search returns nothing: run `/code/scan` first for the target tree.
