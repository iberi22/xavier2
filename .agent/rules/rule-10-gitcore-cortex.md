# Git-Core + Xavier Workflow Rules

## Canonical Sources

- `AGENTS.md` defines workflow behavior
- `.gitcore/ARCHITECTURE.md` defines binding implementation decisions
- `README.md` is the human/product entrypoint

## Memory Split

- GitHub Issues: task state, progress, checklists, planning
- Xavier: reusable memory, research, architecture context, long-horizon recall

## Forbidden Patterns

- Do not create `TODO.md`, `PLAN.md`, `PROGRESS.md`, workflow `CHANGELOG.md`, or scratch summary files.
- Do not hardcode tokens in repo or global IDE configs.
- Do not fork protocol instructions into multiple conflicting rule files.

## Required Behavior

- Prefer Xavier MCP at `http://localhost:8003/mcp`
- Use environment variables for credentials
- Keep changes atomic
- If an issue conflicts with architecture, architecture wins
