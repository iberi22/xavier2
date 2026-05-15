# Jules Agent Operations Skill

This skill provides protocols for managing autonomous coding tasks using the Jules AI agent.

## Core Protocols

### 1. Task Delegation
When delegating a task to Jules:
- Ensure the codebase is stabilized and pushed to the `main` branch.
- Create a GitHub issue with a clear description of the problem, solution, and relevant files.
- Add the `jules` label to the issue to trigger the autonomous agent.

### 2. Session Synchronization
If changes are pushed to `main` while Jules is working, or if a previous Jules session failed:
- Jules typically operates on a snapshot/clone created at the start of a session.
- To ensure Jules sees the latest code, it is recommended to:
    1. Remove the `jules` label from the issue.
    2. Ensure `main` is up-to-date.
    3. Re-add the `jules` label to trigger a fresh session.

### 3. Monitoring Progress
Use the following commands to monitor Jules:
- `jules remote list --session`: List active sessions and their IDs.
- `jules remote pull --session <ID>`: View logs or results for a session.

## Integrator Protocol (High Efficiency)

To handle high volumes of tasks (e.g., 100+ tasks/day), use the **Autonomous Integrator** workflow to prevent code conflicts and bottlenecking on manual reviews.

### 1. Hierarchy of Integration
Rank PRs by conflict risk:
- **Level 1 (Core)**: Architectural changes, core schema updates (e.g., `src/workspace.rs`).
- **Level 2 (Features)**: New modules, adapters, or isolated logic.
- **Level 3 (Maintenance)**: Documentation, Clippy fixes, dead code removal.

**Merge Rule**: Always integrate Level 1 first, then Level 2, then Level 3. Maintenance PRs should be rebased frequently.

### 2. The Integration Branch
1. Maintain an `integration` branch.
2. Automate the merge process:
    - Use `scripts/jules/integrate-all.js` to fetch all Jules PRs.
    - Attempt sequential merges into `integration`.
    - Run `cargo check` and `cargo test` after each successful merge.
3. If a conflict occurs, the Integrator (Antigravity) performs a semantic resolution or delegates a "Conflict Resolution Task" back to Jules.

### 3. Verification Pipeline
- Only merge the `integration` branch into `main` after a 100% test pass rate.
- Automated security scans must return 0 vulnerabilities before any `main` deployment.

## Automated Scripts
Location: `scripts/jules/`
- `list-sessions.js`: Lists all active Jules sessions.
- `trigger-jules.js`: Programmatically labels an issue to trigger Jules.
- `check-api.js`: Validates the `JULES_API_KEY` from `.env`.
- `integrate-all.js`: Fetches and merges multiple Jules PRs into an integration branch.
