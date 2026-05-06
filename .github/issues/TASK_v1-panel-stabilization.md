---
title: "TASK: Stabilize panel build pipeline and route expectations"
labels:
  - ai-plan
  - release
  - panel
assignees: []
---

## Description

The repository documents `/panel` and related API routes as part of the product surface, but the tested runtime returned `404` for `/panel`, and the frontend toolchain is still failing around Biome and build alignment.

## Acceptance Criteria

- [ ] fix `panel-ui` toolchain alignment so local and CI checks run from a fresh install
- [ ] make panel shell behavior explicit when frontend assets are missing
- [ ] ensure documented panel routes match the actual server behavior
- [ ] add panel smoke coverage that distinguishes asset-missing from route-missing failures
- [ ] update public docs to state whether panel is stable, beta, or optional

## Evidence

- direct runtime test returned `404` for `/panel`
- prior validation already showed `panel-ui` check failure from Biome config drift
