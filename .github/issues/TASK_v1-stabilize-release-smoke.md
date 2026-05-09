---
title: "TASK: Stabilize release smoke scripts against the live server surface"
labels:
  - ai-plan
  - release
  - qa
assignees: []
---

## Description

The current release smoke scripts do not match the verified behavior of the running server. In testing, `scripts/release-smoke.ps1` failed because `/build` returned `404`.

## Acceptance Criteria

- [ ] decide whether `/build` is required for `1.0` or should be removed from smoke coverage
- [ ] make PowerShell and shell smoke scripts test the same contract
- [ ] remove insecure token defaults from smoke scripts
- [ ] make smoke scripts fail only on real release blockers, not stale route assumptions
- [ ] wire smoke validation into the documented release checklist

## Evidence

- `scripts/release-smoke.ps1` failed against the tested runtime on `/build`
