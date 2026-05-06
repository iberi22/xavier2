---
title: "TASK: Stabilize MCP contract for Xavier2 1.0"
labels:
  - ai-plan
  - release
  - mcp
assignees: []
---

## Description

The current MCP stdio server works in beta for basic tool calls, but the tool contract is still closer to an internal prototype than a polished `1.0` memory interface.

## Acceptance Criteria

- [ ] define the `1.0` MCP tool naming and argument contract
- [ ] decide whether the contract remains `search`/`add`/`stats` or moves to richer memory-specific names
- [ ] align MCP docs with the actual tool surface
- [ ] ensure security scanning and validation are consistently applied across MCP memory inputs
- [ ] add end-to-end MCP tests covering the final tool contract

## Evidence

- direct MCP testing succeeded for `search`, `add`, and `stats`
- repository docs and review notes still reference richer MemoryFragment-style tooling expectations
