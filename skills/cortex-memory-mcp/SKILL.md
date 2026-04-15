---
name: xavier2-memory-mcp
description: Use Xavier2 through MCP when the host IDE or tool runner requires streamable HTTP MCP transport instead of direct HTTP calls. Use when Codex needs Xavier2 MCP tools such as `create_memory`, `search_memory`, `get_memory`, `list_projects`, `get_project_context`, or `sync_gitcore`.
---

# Xavier2 Memory MCP

Use MCP only when the host requires MCP transport. For local automation and scripts, prefer the HTTP skill.

## Endpoint

Configure Xavier2 as an MCP server at `http://localhost:8003/mcp` with `streamable-http` transport.

```json
{
  "mcpServers": {
    "xavier2-memory": {
      "url": "http://localhost:8003/mcp",
      "transport": "streamable-http"
    }
  }
}
```

## Verify availability

Check `GET /health` before assuming MCP failures are tool-host problems.

## Current MCP tools

- `create_memory`: store a memory with `path`, `content`, and optional metadata, kind, namespace, and provenance.
- `search_memory`: search memory with `query`, `limit`, and optional filters.
- `get_memory`: fetch a memory by `id`.
- `list_projects`: list Xavier2 projects.
- `get_project_context`: retrieve project context by `project_id`.
- `sync_gitcore`: sync Git-Core documentation from a project path.

## Working rules

- Use MCP for IDE-native interactions, not as the default automation path.
- Keep task progress in GitHub Issues; use Xavier2 for reusable knowledge.
- Prefer stable, meaningful `path` values so later searches remain predictable.
- If MCP behavior looks stale or incomplete, inspect the implementation in `src/server/mcp_server.rs` before documenting or relying on extra tools.
