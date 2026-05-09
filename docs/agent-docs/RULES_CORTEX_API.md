---
title: "Xavier API: Rules & Guidelines for AI Agents"
type: GUIDE
id: "guide-xavier-api"
created: 2026-03-21
updated: 2026-03-26
agent: antigravity
model: gemini-2.0-pro
requested_by: user
summary: |
  Comprehensive documentation for AI agents to interact with the Xavier Memory System.
  Includes RPC, REST (v1), Code Intelligence, and Agent Runtime endpoints.
keywords: [api, curl, agents, guidelines, memory, rag, xavier]
tags: ["#api", "#memory", "#agents", "#v2.0"]
---

# 🧠 Xavier API: Guidelines for AI Agents

AI agents should use Xavier as the primary memory and reasoning backend. This document outlines the protocols for context retrieval, task verification, and long-term memory storage.

## 🛡️ Authentication & Environment
Xavier requires a valid token for all requests (except health checks).

1. **Header**: `X-Xavier-Token: <your_token>`
2. **Local Development**:
   - Default Token: `dev-token`
   - Default Port: `8003`
   - Ensure `XAVIER_DEV_MODE=1` is set in your environment to use the default `dev-token` without explicit configuration.

## 📡 Endpoint Categories

### 1. RPC Memory Operations (High Efficiency)
Use these for direct manipulation of the cognitive memory buffer.

- **Add Memory** (`POST /memory/add`)
  ```bash
  curl -X POST http://127.0.0.1:8003/memory/add \
    -H "X-Xavier-Token: dev-token" \
    -H "Content-Type: application/json" \
    -d '{
      "content": "Verified: Branch cleanup task completed successfully.",
      "path": "tasks/cleanup/status",
      "metadata": { "status": "done", "timestamp": "2026-03-26T01:40:00Z" }
    }'
  ```

- **Semantic Query** (`POST /memory/query`)
  ```bash
  curl -X POST http://127.0.0.1:8003/memory/query \
    -H "X-Xavier-Token: dev-token" \
    -H "Content-Type: application/json" \
    -d '{ "query": "What was the result of the last branch cleanup?" }'
  ```

- **Search (Keyword/BM25)** (`POST /memory/search`)
  ```bash
  curl -X POST http://127.0.0.1:8003/memory/search \
    -H "X-Xavier-Token: dev-token" \
    -H "Content-Type: application/json" \
    -d '{ "query": "cleanup", "limit": 5 }'
  ```

### 2. Knowledge & Belief Graph
Interactions with the structured reasoning layer.

- **Retrieve Graph** (`GET /memory/graph`)
- **Manual Curation** (`POST /memory/curate`): `{ "id": "<doc_id>" }`
- **Auto-Manage** (`POST /memory/manage`): Triggers structured organization of memories.

### 3. Agent Runtime
Trigger complex reasoning chains or specialized sub-agents.

- **Run Agent Task** (`POST /agents/run`)
  ```bash
  curl -X POST http://127.0.0.1:8003/agents/run \
    -H "X-Xavier-Token: dev-token" \
    -H "Content-Type: application/json" \
    -d '{
      "query": "Analyze the potential impact of removing the legacy auth module.",
      "session_id": "impact-analysis-001"
    }'
  ```

### 4. Code Intelligence
Interface with the workspace AST and code-graph.

- **Find Code Symbols** (`POST /code/find`)
  ```bash
  curl -X POST http://127.0.0.1:8003/code/find \
    -H "X-Xavier-Token: dev-token" \
    -H "Content-Type: application/json" \
    -d '{ "query": "auth_middleware", "kind": "function", "limit": 1 }'
  ```
- **Code Stats** (`GET /code/stats`): Returns indexed file and symbol counts.

### 5. Standard REST API (v1)
Standardized endpoints for compatibility with Mem0/Zep patterns.

- `POST /v1/memories`: Add a new memory.
- `GET /v1/memories`: List all memories with pagination.
- `GET /v1/memories/search`: Semantic search.
- `GET/PUT/DELETE /v1/memories/{id}`: Direct document management.

---

## 🤖 Agent Usage Patterns

### Task Verification
Always store a summary of completed tasks in Xavier to ensure persistent recall across sessions.
> **Action**: `POST /memory/add` with `type: task_verification` in metadata.

### Context Awareness
Before starting a complex refactor, query Xavier for relevant "Crucial Decisions" or "Architectural Patterns".
> **Action**: `POST /memory/query` with reasoning enabled.

### System Health
Use `/health` or `/readiness` to verify that the memory system and embedding providers are online before performing bulk operations.
