---
title: "EPIC: Xavier v1.0 release stabilization backlog"
labels:
  - ai-plan
  - release
  - architecture
assignees: []
---

## Description

Track the concrete work required to move Xavier from `0.6 beta usable` to a defensible `1.0` release.

This epic is grounded in repository review plus direct usage testing of the current memory system through CLI, HTTP, MCP, and release smoke scripts.

## Acceptance Criteria

- [ ] CLI, HTTP, MCP, panel, and smoke scripts agree on one canonical runtime contract
- [ ] public docs describe the current product surface accurately
- [ ] release smoke passes without local patching
- [ ] no production-facing `dev-token` or insecure defaults remain
- [ ] workspace and storage isolation are reproducible
- [ ] panel build and route expectations are stable
- [ ] remaining `1.0` blockers are tracked as closed child issues

## Child Issues

- [ ] CLI and server contract alignment
- [ ] storage isolation and path handling
- [ ] release smoke parity
- [ ] panel build and route stabilization
- [ ] security defaults cleanup
- [ ] config centralization to one JSON plus `.env` secrets
- [ ] docs sync and feature inventory
- [ ] `clippy` and doctest release gate cleanup
- [ ] MCP contract stabilization
