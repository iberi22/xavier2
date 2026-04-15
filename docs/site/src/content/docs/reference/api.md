---
title: API Reference
description: Complete API endpoints for Xavier2
---

# API Reference

Complete reference for the Xavier2 HTTP endpoints implemented by the Rust server.

## Base URL

```text
http://localhost:8003
```

## Authentication

All endpoints except `GET /health` and `GET /readiness` require the `X-Xavier2-Token` header. This is the current minimal auth path implemented by the server.

```bash
curl -H "X-Xavier2-Token: <your-token>" ...
```

All responses include an `X-Request-Id` header for operational tracing.

JWT/RBAC code exists in the repository, but it is not the active production auth path in the current server router.

## Endpoints Overview

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/readiness` | Runtime and dependency readiness |
| GET | `/build` | Build and provider metadata |
| POST | `/memory/add` | Add memory |
| POST | `/memory/delete` | Delete memory by `id` or `path` |
| POST | `/memory/reset` | Reset in-memory state |
| POST | `/memory/search` | Search memories |
| POST | `/memory/query` | Query through runtime |
| GET | `/memory/graph` | Get belief graph |
| POST | `/agents/run` | Run the agent runtime |
| POST | `/sync` | Report sync count |
| GET | `/v1/account/usage` | Authenticated usage, quotas, and optimization counters |
| GET | `/v1/account/limits` | Workspace limits |
| GET | `/v1/sync/policies` | Sync policy metadata |
| GET | `/v1/providers/embeddings/status` | Embedding provider status |
| POST | `/code/scan` | Index a source tree |
| POST | `/code/find` | Search indexed symbols |
| GET | `/code/stats` | Code index statistics |
| GET/POST/DELETE | `/mcp` | MCP transport surface |
| GET | `/panel` | Panel shell |
| GET/POST/DELETE | `/panel/api/*` | Panel API |
| GET/POST/PUT/DELETE | `/v1/memories*` | V1 memory API |

## Health

### GET /health

Returns a minimal service health payload.

### GET /readiness

Returns readiness information for workspace, memory store, code graph, embeddings, and configured model provider.

### GET /build

Returns build metadata plus memory-store and provider details.

## Memory

### POST /memory/add

```json
{
  "content": "Memory content",
  "path": "project/context",
  "metadata": {
    "tags": ["tag1", "tag2"]
  }
}
```

### POST /memory/search

```json
{
  "query": "search term",
  "limit": 10
}
```

### POST /memory/query

```json
{
  "query": "What is Xavier2?",
  "limit": 10
}
```

### POST /memory/delete

Delete by `id` or `path`.

### POST /memory/reset

Reset the current in-memory document set for the workspace.

### GET /memory/graph

Returns nodes and relations from the belief graph.

## Account and Workspace

### GET /v1/account/usage

Returns workspace usage, request units, and optimization counters.

### GET /v1/account/limits

Returns workspace storage and request limits.

### GET /v1/sync/policies

Returns current and supported sync policy metadata.

## Code Index

### POST /code/scan

Rebuilds the code index for a target path.

### POST /code/find

Searches indexed symbols with optional kind filtering.

### GET /code/stats

Returns indexed file and symbol counts.

## Errors

Auth failures return status `401` with:

```text
Unauthorized: Invalid or missing X-Xavier2-Token
```
