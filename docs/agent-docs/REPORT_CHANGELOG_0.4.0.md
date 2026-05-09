---
title: "Xavier 0.4.0 Release Changelog"
type: REPORT
id: "report-changelog-0-4-0"
created: 2026-03-11
updated: 2026-03-11
agent: codex
model: gpt-5
requested_by: user
summary: |
  Initial 0.4.0 changelog entry for benchmark infrastructure,
  test system migration, and first LoCoMo baseline reporting.
keywords: [xavier, changelog, benchmarks, locomo, swe-bench]
tags: ["#xavier", "#release", "#benchmarks", "#locomo", "#0.4.0"]
topics: [memory, benchmarking, agentic-workflows]
project: Xavier
module: benchmarks
language: markdown
priority: high
status: draft
confidence: 0.92
token_estimate: 400
complexity: moderate
---

# Xavier 0.4.0

## Added

- Standardized Cargo test layout with dedicated integration, E2E, and benchmark targets.
- GitHub Actions workflows for `LoCoMo` and `SWE-bench` benchmarking.
- `LoCoMo` benchmark runner that clones the official dataset to a temporary workspace and scores Xavier on QA recall quality.
- `SWE-bench` evaluation harness wrapper for self-hosted Linux runners with Docker and sufficient disk.
- `/memory/reset` endpoint to reset in-memory state between benchmark conversations.

## Changed

- Architecture clarified to position Xavier as a memory substrate for bidirectional agentic/RAG execution.
- CI updated to use valid Cargo targets for integration tests, E2E tests, and benches.
- Benchmark strategy split by viability:
  - `LoCoMo`: continuous benchmark for conversational memory quality.
  - `SWE-bench`: agent-stack benchmark, only valid on a dedicated self-hosted runner.

## Initial Benchmark Result

- `LoCoMo` baseline:
  - Samples: `10`
  - Questions: `50`
  - Exact Match: `0.0`
  - Token F1: `0.023191227171872334`
  - Category 1 Token F1: `0.02108634723074047`
  - Category 2 Token F1: `0.007495590828924161`
  - Category 3 Token F1: `0.055128205128205134`
  - Category 4 Token F1: `0.1142857142857143`

This first number is intentionally preserved as the starting baseline for future improvement rounds.

## Improvement Round 1

- `LoCoMo` benchmark after retrieval and response heuristics update:
  - Samples: `10`
  - Questions: `50`
  - Exact Match: `0.0`
  - Token F1: `0.06021547660628042`

This moved Xavier from the initial baseline of `0.023191227171872334` Token F1 to `0.06021547660628042`, a `2.60x` improvement on the same slice.
