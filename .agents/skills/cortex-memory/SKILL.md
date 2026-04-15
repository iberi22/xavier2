---
name: xavier2-memory
description: Use Xavier2 cognitive memory as an MCP-backed durable knowledge layer for persistent research, architecture context, and reusable agent memory. Use when the host requires MCP transport and the task needs Xavier2 tools such as `create_memory`, `search_memory`, `get_memory`, `list_projects`, or `get_project_context`.
---

# Xavier2 Memory MCP Skill

Use Xavier2 through MCP when the host tool expects MCP transport. For local scripts and operational automation, prefer the HTTP skill under `skills/xavier2-http-curl`.

## Endpoint

Use `http://localhost:8003/mcp` with `streamable-http` transport.

## Preconditions

- Xavier2 should be running locally.
- `GET /health` should respond before blaming the MCP host.
- Use GitHub Issues for task state and Xavier2 for durable knowledge.

## Current MCP tools

- `create_memory`
- `search_memory`
- `get_memory`
- `list_projects`
- `get_project_context`
- `sync_gitcore`

## Working rules

- Search before storing new knowledge.
- Use stable `path` values and meaningful metadata.
- Do not store secrets or ephemeral scratch notes.
- If the tool list appears different, inspect `src/server/mcp_server.rs` and update this skill instead of guessing.
