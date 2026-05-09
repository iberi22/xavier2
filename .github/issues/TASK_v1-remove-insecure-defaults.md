---
title: "TASK: Remove insecure defaults from production-facing docs and scripts"
labels:
  - ai-plan
  - release
  - security
assignees: []
---

## Description

The repository still contains widespread production-facing references to `dev-token`, hardcoded token examples, and stale defaults in scripts and docs. That is not acceptable for a professional `1.0` release.

## Acceptance Criteria

- [ ] remove `dev-token` defaults from production-facing scripts and public docs
- [ ] replace insecure examples with required secure-token setup
- [ ] keep any intentionally insecure examples isolated to explicit local-development notes only
- [ ] add guardrails so new insecure defaults cannot be committed again
- [ ] audit benchmark and migration scripts for stale auth defaults

## Evidence

- `dev-token` references remain across scripts, docs, and auxiliary tooling
