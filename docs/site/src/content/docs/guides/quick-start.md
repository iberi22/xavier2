---
title: Quick Start
description: Get started with Xavier in 5 minutes
---

# Quick Start Guide

Get Xavier running and make your first authenticated HTTP call in under 5 minutes.

The recommended local path is HTTP with `curl`. MCP is optional and should only be enabled when your IDE specifically needs it.

## Start the Server

Run Xavier directly:

```bash
cargo run
```

Or start the container stack:

```bash
docker compose up -d
```

Then verify health:

```bash
curl http://localhost:8006/health
```

## Prepare Auth

Non-secret runtime settings belong in `config/xavier.config.json`.
Secrets belong in `.env`.

```bash
export XAVIER_URL="${XAVIER_URL:-http://localhost:8006}"
export XAVIER_TOKEN="${XAVIER_TOKEN:?set-a-long-random-token-first}"
```

## Your First API Calls

### 1. Add Memory

```bash
curl -X POST "$XAVIER_URL/memory/add" \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Xavier is a cognitive memory system for AI agents",
    "path": "test/intro",
    "metadata": {"tags": ["ai", "memory"]}
  }'
```

**Response:**

```json
{
  "status": "ok",
  "message": "Document added to memory"
}
```

### 2. Search Memory

```bash
curl -X POST "$XAVIER_URL/memory/search" \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "cognitive memory system",
    "limit": 5
  }'
```

**Response:**

```json
{
  "status": "ok",
  "query": "cognitive memory system",
  "results": [
    {
      "id": "memory_abc123",
      "content": "Xavier is a cognitive memory system for AI agents",
      "path": "test/intro",
      "metadata": {"tags": ["ai", "memory"]}
    }
  ]
}
```

### 3. Query Through the Runtime

```bash
curl -X POST "$XAVIER_URL/memory/query" \
  -H "X-Xavier-Token: $XAVIER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "What is Xavier?",
    "limit": 5
  }'
```

### 4. Inspect Code Index Stats

```bash
curl -H "X-Xavier-Token: $XAVIER_TOKEN" \
  "$XAVIER_URL/code/stats"
```

## Optional MCP Integration

Only use this when a local IDE or tool host requires MCP transport. For local automation and validation, stay on HTTP/`curl`.

Add to your OpenClaw config:

```json
{
  "tools": {
    "mcp": {
      "servers": {
        "xavier": {
          "enabled": true,
          "url": "http://localhost:8006/mcp"
        }
      }
    }
  }
}
```

## Next Steps

- [Architecture Overview](/architecture/overview/) - Deep dive
- [Memory Module](/modules/memory/) - Understand memory operations
- [API Reference](/reference/api/) - Full payload reference
- [Testing](/testing/overview/) - Verify your setup
