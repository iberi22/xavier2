---
title: "TASK: Finish migrating runtime configuration to one canonical JSON plus .env secrets"
labels:
  - ai-plan
  - release
  - configuration
assignees: []
---

## Description

Xavier now has a canonical runtime config file at `config/xavier.config.json` and `.env.example` has been reduced to credentials and secrets. The remaining work is to migrate the rest of the codebase off scattered direct environment reads for non-secret operational settings.

## Acceptance Criteria

- [ ] all non-secret runtime settings resolve from `config/xavier.config.json`
- [ ] `.env` is reserved for credentials, secrets, and explicit overrides only
- [ ] direct non-secret `std::env::var(...)` reads are removed or funneled through the central settings loader
- [ ] docs and scripts stop teaching multiple competing configuration paths
- [ ] tests cover config-file loading and override behavior

## Notes

- A compatibility loader already exists to project `config/xavier.config.json` into current `XAVIER_*` paths at startup.
- This issue tracks the full migration from compatibility mode to a genuinely centralized configuration model.
