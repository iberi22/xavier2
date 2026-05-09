---
title: Testing Overview
description: Current verification strategy for Xavier
---

# Testing Strategy

This page describes the validation flow that is currently relevant to the repo as it exists today.

## Main Validation Commands

```bash
cargo test --workspace --features ci-safe --exclude xavier-web
cargo build --bin xavier
npm run build --workspace panel-ui
npm run build --workspace docs/site
```

## What These Checks Cover

- Rust unit and integration coverage that is safe for CI
- Main server binary build
- Panel UI production build
- Public docs site build

## Current Test Surface In The Repo

- `tests/e2e.rs`
- `tests/integration.rs`
- `tests/sync_test.rs`
- `tests/integration/*`
- inline module tests under `src/`

The repo also includes validation around:

- HTTP handlers
- memory persistence
- workspace quotas and usage tracking
- code indexing
- agent runtime paths
- panel and release smoke flows in CI

## CI Notes

The current CI flow lives in `.github/workflows/ci.yml` and includes:

- panel check
- cargo check
- clippy
- test
- build
- panel E2E
- release smoke

Docs publishing has its own workflow under `.github/workflows/docs.yml`.

## Important Caveat

Passing tests do not currently mean:

- fully hardened production auth
- benchmark latency under the older `< 500ms` target
- fully closed monitoring and alerting

Those remain separate operational milestones.
