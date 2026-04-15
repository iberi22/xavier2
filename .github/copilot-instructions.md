# GitHub Copilot Instructions

This repository follows Git-Core Protocol with Xavier2 as the shared memory backend.

## Canonical Read Order

1. `AGENTS.md`
2. `.gitcore/ARCHITECTURE.md`
3. `.gitcore/features.json`
4. `README.md`
5. `docs/agent-docs/RESEARCH_STACK_CONTEXT.md` for dependency work

## Memory Model

- GitHub Issues are the source of truth for task state and progress.
- Xavier2 is the source of truth for reusable project memory, research, and long-horizon agent context.
- Do not create local tracking files such as `TODO.md`, `PLAN.md`, `PROGRESS.md`, or workflow `CHANGELOG.md`.

## Required Workflow

1. Read the canonical files in order.
2. Run a health check before new feature work.
3. Follow `.gitcore/ARCHITECTURE.md` if issue text presents conflicting stack choices.
4. Prefer `gc` for protocol-aware operations and `gh` for GitHub-specific actions.
5. Keep commits atomic and reference the relevant issue.

## IDE / MCP Rules

- Prefer Xavier2 MCP at `http://localhost:8003/mcp`.
- Keep credentials in machine environment variables only.
- Do not hardcode access tokens into config or command arguments stored in repo files.

## Repo Hygiene

- `README.md` is the product entrypoint.
- `AGENTS.md` is the workflow contract.
- `docs/agent-docs/` is for persistent agent docs only.
- Root-level scratch outputs, test logs, and exported chat artifacts do not belong in version control.
