# Xavier Project Context

- Repository: `xavier`
- Runtime: Rust + Tokio
- Memory store: SurrealDB
- Agent memory endpoint: `http://localhost:8003/mcp`
- Task state lives in GitHub Issues, not in local markdown trackers

## Read Order

1. `AGENTS.md`
2. `.gitcore/ARCHITECTURE.md`
3. `.gitcore/features.json`
4. `README.md`

## Project Intent

Use Xavier as the reusable memory substrate for agentic workflows. Prefer storing durable knowledge in Xavier and keep repo files focused on product code, architecture, and user-facing documentation.
