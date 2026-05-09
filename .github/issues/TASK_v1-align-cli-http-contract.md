---
title: "TASK: Align CLI client behavior with the HTTP server contract"
labels:
  - ai-plan
  - release
  - cli
assignees: []
---

## Description

The current CLI `add`, `search`, and `stats` commands behave as HTTP clients but still present themselves like local-first direct memory commands. During usage testing, they only worked reliably when a matching `XAVIER_PORT` was set and did not behave as a clean `XAVIER_URL`-driven client surface.

## Acceptance Criteria

- [ ] choose one canonical client configuration contract: `XAVIER_URL` or `XAVIER_HOST` + `XAVIER_PORT`
- [ ] update CLI code to honor that contract consistently
- [ ] remove misleading output such as hardcoded `localhost:8006` messages when another port is active
- [ ] update CLI docs and README examples to match real behavior
- [ ] add regression coverage for non-default port usage

## Evidence

- direct usage testing required `XAVIER_PORT=8016` for the CLI path to hit the intended server
- the CLI still printed `localhost:8006` while talking to another port
