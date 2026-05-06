---
title: API Reference
description: Complete API endpoints for Xavier2
---

# API Reference

Complete reference for the Xavier2 HTTP endpoints implemented by the Rust server.

This page describes the broader HTTP surface implemented in the repository. The current `xavier2 http` entry point is still converging toward this full contract, so some routes should be treated as beta or conditional until the `1.0` stabilization pass is complete.

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
| GET | `/build` | Build and provider metadata, when exposed by the active server surface |
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
| GET | `/v1/public/*` | Read-only public dataset files and metadata |
| POST | `/code/scan` | Index a source tree |
| POST | `/code/find` | Search indexed symbols |
| GET | `/code/stats` | Code index statistics |
| GET/POST/DELETE | `/mcp` | MCP transport surface |
| GET | `/panel` | Panel shell when frontend assets are built |
| GET/POST/DELETE | `/panel/api/*` | Panel API when panel support is enabled |
| GET/POST/PUT/DELETE | `/v1/memories*` | V1 memory API |

## Health

### GET /health

Returns a minimal service health payload.

### GET /readiness

Returns readiness information for workspace, memory store, code graph, embeddings, and configured model provider.

### GET /build

Returns build metadata plus memory-store and provider details when the active server surface exposes the route.

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

## Beta Notes

- The current release line is `0.6 beta usable`, not `1.0`.
- CLI memory commands still behave as HTTP clients against the running server.
- Panel shell availability depends on built frontend assets.
- Release smoke coverage is still being aligned with the live server contract.

## Account and Workspace

### GET /v1/account/usage

Returns workspace usage, request units, and optimization counters.

### GET /v1/account/limits

Returns workspace storage and request limits.

### GET /v1/sync/policies

Returns current and supported sync policy metadata.

## Public Dataset

The `/v1/public/*` surface exposes the generated `xavier-dataset/` export as read-only, cacheable project context. These endpoints are intended for agents and public documentation tooling that need repository intelligence without cloning, indexing, or authenticating against the private memory API.

Public dataset endpoints are:

- read-only
- rate-limited
- safe for unauthenticated access when explicitly enabled by deployment policy
- backed by the same files produced by `xavier2 export --public`

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/v1/public/manifest` | Returns `dataset_manifest.json` with schema versions, export timestamp, and aggregate stats |
| GET | `/v1/public/memories` | Streams `memories.ndjson` |
| GET | `/v1/public/entities` | Streams `entities.ndjson` |
| GET | `/v1/public/entity-edges` | Streams `entity_edges.ndjson` |
| GET | `/v1/public/timeline-events` | Streams `timeline_events.ndjson` |
| GET | `/v1/public/git-commits` | Streams `git_commits.ndjson` |
| GET | `/v1/public/code-symbols` | Streams `code_symbols.ndjson` |
| GET | `/v1/public/code-relations` | Streams `code_relations.ndjson` |
| GET | `/v1/public/ck-metrics` | Streams `ck_metrics.ndjson` |

Example:

```bash
curl http://localhost:8003/v1/public/manifest
curl http://localhost:8003/v1/public/memories | head -n 20
```

Deployments may serve these files directly from GitHub raw, object storage, a CDN, or the Xavier2 HTTP server. The API contract is the same: immutable export files, stable schema versions, and no mutation through `/v1/public/*`.

## Code Index

### POST /code/scan

Rebuilds the code index for a target path.

### POST /code/find

Searches indexed symbols with optional kind filtering.

### GET /code/stats

Returns indexed file and symbol counts.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `XAVIER2_URL` | from `config/xavier2.config.json` | Canonical client base URL for HTTP API calls |
| `XAVIER2_TOKEN` | required | Auth token for protected routes |
| `XAVIER2_PORT` | `8006` | HTTP bind port |
| `XAVIER2_HOST` | `0.0.0.0` | Bind address |
| `XAVIER2_CONFIG_PATH` | `config/xavier2.config.json` | Path to runtime JSON config |
| `XAVIER2_LOG_LEVEL` | `info` | Log verbosity |

## Errors

Auth failures return status `401` with:

```text
Unauthorized: Invalid or missing X-Xavier2-Token
```
