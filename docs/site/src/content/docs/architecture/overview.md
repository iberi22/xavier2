---
title: Architecture Overview
description: Current runtime architecture for Xavier
---

# Architecture Overview

This page describes the **current runtime architecture that is actually present in the repository**, not an idealized future architecture.

## Main Runtime Components

```text
Rust server
  -> HTTP routes
  -> optional MCP transport
  -> panel routes
  -> workspace registry
  -> memory runtime
  -> code-graph sidecar
```

## Current Durable Storage Story

- Default validated backend: `FileMemoryStore`
- Runtime cache/search layer: `QmdMemory`
- Code indexing: SQLite sidecar via `code-graph`
- Embeddings: external embedding service, commonly `pplx-embed`
- SurrealDB: present in the codebase and Docker setup, but not the default validated backend in the current deployment story

## Important Distinction

There are two valid architectural views in this repo:

1. **Current runtime truth**
   - what the binary exposes today
   - what current docs and CI should describe

2. **Architectural direction**
   - future hosted storage and broader agentic workflows
   - optional SurrealDB-backed durable memory
   - additional hardening and operational layers

Public docs should prioritize the first view unless a section is explicitly labeled as roadmap or future direction.

## Main Source Files

- `src/main.rs` wires the HTTP server, auth middleware, panel, MCP, memory, v1 API, and code routes.
- `src/server/http.rs` contains the main HTTP handlers.
- `src/server/v1_api.rs` contains the v1 REST-style memory API.
- `src/workspace.rs` contains workspace config, usage tracking, and durable store selection.
- `src/memory/` contains memory, belief graph, indexing, and store implementations.

## Current Constraints

- Auth is currently token-based through `X-Xavier-Token`.
- JWT/RBAC code exists in `src/security/`, but it is not the active server auth path.
- Latest benchmark quality is strong, but latency still exceeds the older `< 500ms` target.
- Monitoring exists as a Compose profile and should not be over-described as a fully closed operational system yet.
