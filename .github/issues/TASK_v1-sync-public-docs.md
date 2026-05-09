---
title: "TASK: Sync public documentation to the verified beta surface"
labels:
  - ai-plan
  - release
  - documentation
assignees: []
---

## Description

The repository mixed aspirational product claims with behavior that could not be reproduced directly in current usage tests. Public docs should describe the verified beta surface first and the `1.0` target separately.

## Acceptance Criteria

- [ ] keep README, CLI docs, API docs, and feature status aligned
- [ ] distinguish verified beta behavior from planned `1.0` behavior
- [ ] document panel, smoke, and CLI caveats explicitly until they are fixed
- [ ] remove stale examples that reference missing flags or unsupported routes
- [ ] publish the `1.0` gap list as a clear release backlog

## Evidence

- previous docs referenced flags and subcommands not present in current CLI help
- route documentation and smoke scripts did not fully match the tested runtime
