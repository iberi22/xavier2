---
title: "TASK: Make release quality gates pass for Rust core"
labels:
  - ai-plan
  - release
  - rust
assignees: []
---

## Description

The repository is not `1.0`-ready while `cargo clippy -- -D warnings` and doctests still fail. Current failures include dead code, stale docs, and mechanical lints across core modules.

## Acceptance Criteria

- [ ] eliminate the current `clippy -D warnings` failures without papering over them
- [ ] fix stale doctests such as the `WorkingMemory::new(10)` example
- [ ] keep `cargo check`, `cargo test`, and `cargo build --release` green after cleanup
- [ ] record any justified suppressions with narrow scope and explanation

## Evidence

- current validation shows `clippy -D warnings` failures across context, memory, server, and workspace modules
- current doctests still fail in `memory::working`
