---
title: "Private Xavier Continuation Strategy"
type: STRATEGY
id: "strategy-private-xavier-continuation"
created: 2026-03-19
updated: 2026-03-19
agent: codex
model: gpt-5
requested_by: user
summary: |
  Private-continuation plan for Xavier covering repository visibility,
  private mirror strategy, SaaS/private feature isolation, and the next
  implementation steps for hosted sync, quotas, and billing.
keywords: [xavier, private, strategy, github, saas, enterprise, sync]
tags: ["#strategy", "#private", "#xavier", "#saas"]
project: xavier
module: platform
language: markdown
priority: high
status: approved
confidence: 0.94
---

# Xavier Private Continuation Strategy

## Objective

Move active Xavier product work into a private continuation path while keeping the current codebase usable as the implementation base. The private path should support hosted sync, billing, private roadmap work, and SaaS-specific integrations without expanding the number of public-facing projects.

## Current GitHub State

- Public repo: `iberi22/xavier`
- Public upstream repo: `southwest-ai-labs/xavier`
- Desired operating mode:
  - private continuation
  - reduced public exposure
  - keep using the current project as the implementation base where possible

## Recommended Repository Strategy

### Short Term

1. Create a **private repository** under `southwest-ai-labs` as the continuation target.
2. Treat it as the active private line for:
   - hosted product work
   - billing
   - sync premium logic
   - Google-managed embeddings integration
   - private infra manifests
   - roadmap/issues not intended for public visibility
3. If permissions allow, switch the current active GitHub repo to private as well.

### Medium Term

Split by concern, not by long-lived divergent branches:

- Public/self-hosted core:
  - memory engine
  - HTTP/MCP surface
  - local Docker deployment
  - BYO model keys
- Private/cloud layer:
  - hosted control plane
  - quota persistence
  - billing
  - Stripe
  - sync orchestration
  - managed embeddings
  - team/admin controls

## Product Direction for Paid Personal Plan

### Personal ($10/month)

- 500 MB hosted memory
- memory management UI/API
- sync across devices included
- HTTPS access included
- BYO LLM/API key
- basic backup/restore
- quota-regulated API access

### Pro ($20/month)

- 2 GB hosted memory
- higher quotas
- more sync throughput
- better operational priority
- premium add-on eligibility

### Not Included in Base Plans

- bundled LLM usage
- bundled managed Google embeddings
- unlimited heavy sync/import

## API Governance Model

### Burst Limits

- global: `60 req/min` on Personal
- sync: `10 req/min`
- agent runtime: `5 req/min`

### Monthly Quotas

- reads/search: `20,000/month`
- writes/update/delete: `5,000/month`
- sync jobs: `1,000/month`
- agent/runtime queries: `2,000/month`

### Payload Caps

- memory add/update: `256 KB` per document
- sync payload: `5 MB` per request
- deduplicate by content hash before ingest

## Private Workstreams

### Workstream 1: Private GitHub Footprint

- create private continuation repo
- move planning/issues there
- stop using public issues for private commercial roadmap

### Workstream 2: Hosted Persistence

- persist workspace usage counters
- persist quotas and billing state outside process memory
- prepare Cloud Run + external DB model

### Workstream 3: Sync and Billing

- delta sync
- storage accounting
- request units by endpoint type
- Stripe product mapping for `free`, `personal`, `pro`, and storage add-ons

### Workstream 4: Managed Google Add-On

- separate add-on from base plans
- meter embedding calls separately
- keep BYO path as the default

## Immediate Next Steps

1. Create the private continuation repo in `southwest-ai-labs`.
2. Attempt to switch current public repo visibility to private if allowed.
3. Continue implementation privately from the current working tree.
4. Add persistent quota counters and weighted API usage accounting next.
5. Move private planning/issues away from the public repository.

## Operational Note

The current repository already contains hosted-surface groundwork:

- workspace-scoped token auth
- storage quota checks
- usage/limits endpoints
- sync-policy metadata

The next private iteration should focus on durable metering and billing, not on redesigning the surface again.
