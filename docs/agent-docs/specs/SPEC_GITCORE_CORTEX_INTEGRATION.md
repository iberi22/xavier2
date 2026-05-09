---
title: "GitCore + Xavier Integration Specification"
type: SPEC
id: "spec-gitcore-xavier-integration"
created: 2026-03-17
updated: 2026-03-17
agent: codex
model: gpt-5
requested_by: user
summary: |
  Canonical integration contract for using Xavier as the shared memory backend
  across Git-Core agent workflows and IDE MCP configurations.
keywords: [gitcore, xavier, mcp, memory, agents]
tags: ["#gitcore", "#xavier", "#mcp", "#memory"]
project: xavier
module: integration
language: rust
priority: high
status: approved
confidence: 0.93
complexity: moderate
---

# GitCore + Xavier Integration

## Purpose

Use Xavier as the shared memory backend for agents while keeping GitHub Issues as the only task-tracking substrate.

## Canonical Rules

- Agents read `AGENTS.md`, `.gitcore/ARCHITECTURE.md`, `.gitcore/features.json`, and `README.md` in that order.
- GitHub Issues store task state, planning, and progress.
- Xavier stores reusable memory, research context, architecture recall, and long-horizon agent context.
- IDE rule files are adapters; they must not redefine protocol behavior independently.

## Runtime Contract

- Local endpoint: `http://localhost:8003`
- MCP endpoint: `http://localhost:8003/mcp`
- Compose services: `xavier`, `surrealdb`
- SurrealDB readiness check uses `/surreal is-ready --endpoint ws://localhost:8000`

## Global MCP Expectations

- Antigravity should expose `xavier-memory`
- GitHub and Supabase MCP servers remain available
- Credentials are sourced from machine environment variables only

## Repo Hygiene

- Root contains product entrypoints and runtime files only
- Archived strategy/report material moves under `docs/agent-docs/archive/`
- Scratch outputs and local test artifacts stay ignored

## Sync Protocol (Git-Chunk)

Xavier implements a decentralized synchronization protocol inspired by Engram:

1. **Chunking**: Memory documents are batched into immutable JSONL chunks.
2. **Hashing**: Each chunk is named by its SHA256 content hash (e.g., `<hash>.jsonl.gz`).
3. **Compression**: Chunks are Gzip-compressed to minimize repository bloat.
4. **Manifest**: A `manifest.json` tracks the active chunks and their contained document IDs.
5. **Conflict Resolution**: Since chunks are immutable and content-addressed, Git merges handle synchronization naturally without line-level conflicts in a single large database file.

## Current Tool Surface

The current Xavier MCP/server documentation should match the existing implementation surface:

- `create_memory`
- `search_memory`
- `get_memory`
- `list_projects`
- `get_project_context`
- `sync_gitcore`
- `export_chunks`
- `import_chunks`
