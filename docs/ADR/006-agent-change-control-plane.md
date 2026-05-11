# ADR-006: Agent Change Control Plane

**Status:** Proposed
**Date:** 2026-05-11
**Author:** Xavier2 CEO + BELA
**Deciders:** BELA

---

## Context

Xavier currently provides passive memory capabilities: store, search, recall, graph, reflection. The coordination module (`src/coordination/`) has AgentRegistry, MessageBus, and DistributedLock — but these operate only in-memory and don't govern _which_ files agents can modify or _how_ changes relate to architectural decisions.

With multiple AI agents (Jules, Codex, subagents) working on the same codebase concurrently, we need Xavier to become an **active coordination layer** — not just remembering what happened, but **governing how agents change the project**.

## Decision

Add a **Change Control domain** (`src/domain/change_control/`) responsible for:

1. **Task Scope** — What files an agent intends to touch
2. **File Leases** — Temporary file ownership with TTL
3. **Conflict Detection** — Detecting overlapping scopes between concurrent agents
4. **Risk Scoring** — Semantic risk based on code-graph impact analysis
5. **Policy Enforcement** — Executable architectural rules (`.gitcore/change-control.yaml`)
6. **Operational Memory** — Reusable summaries of completed changes
7. **Merge Planning** — Safe ordering of concurrent changes

### Architecture (Hexagonal)

```
src/
├── domain/change_control/       ← Pure domain types
│   ├── task.rs                  ← AgentTask, ChangeScope
│   ├── lease.rs                 ← FileLease, LeaseMode, LeaseStatus
│   ├── conflict.rs              ← ConflictReport, ConflictType
│   └── policy.rs                ← ChangePolicy, LayerRule, ValidationRule
│
├── ports/inbound/
│   └── change_control_port.rs   ← trait ChangeControlPort
│
├── app/
│   └── change_control_service.rs ← Business logic
│
├── adapters/inbound/http/handlers/
│   └── change_control.rs        ← HTTP handlers
│
└── coordination/ (extend)
    └── lease_registry.rs        ← DistributedLock → FileLease with TTL
```

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| POST | `/change/tasks` | Create task with scope |
| GET | `/change/tasks/{id}` | Get task details |
| POST | `/change/leases/claim` | Claim file lease |
| POST | `/change/leases/release` | Release file lease |
| GET | `/change/leases/active` | List active leases |
| POST | `/change/conflicts/check` | Check scope conflicts |
| POST | `/change/validate` | Run pre-commit validations |
| POST | `/change/complete` | Complete task + consolidate memory |
| GET | `/change/merge-plan` | Get safe merge ordering |

### Separation of Concerns

| What | Where | Duration |
|------|-------|----------|
| Task checklist | GitHub Issues | Ephemeral |
| Decision records | Xavier (QmdMemory) | Permanent |
| File leases | Xavier (in-memory + TTL) | Temporary |
| Conflict history | Xavier | Permanent/summarized |
| Architecture rules | `.gitcore/change-control.yaml` | Permanent |
| PR results | Xavier | Permanent |

### Integration with System 1 / 2 / 3

```
S1 (Fast): Search memories → Query code-graph → Detect active leases
S2 (Slow): Decide scope safety → Calculate dependencies → Recommend split
S3 (Oversight): Block high-risk → Require approval → Validate architecture
```

## Consequences

### Positive
- Safer parallelism for multiple agents modifying the same codebase
- Reusable operational memory from previous code changes
- Executable architecture rules prevent accidental boundary violations
- Conflict detection reduces PR chaos
- Merge planner optimizes CI/CD throughput

### Negative
- New state model to maintain (leases, scopes, policies)
- Requires lease expiry and cleanup (TTL-based)
- Requires clear distinction between temporary task state and durable knowledge
- Initial overhead for agents to declare scope before modifying files

### Neutral
- Does NOT replace Git — Git remains source of truth for version history
- Does NOT replace GitHub Issues — Issues remain the collaboration surface
- Xavier becomes the **memory + coordination control plane**

## Implementation Phases

| Phase | Content | Issue | Lines Est. | Dependencies |
|-------|---------|-------|------------|--------------|
| 1a | Domain types | [#222](https://github.com/iberi22/xavier/issues/222) | ~250 | None |
| 1b | Port + Service | [#223](https://github.com/iberi22/xavier/issues/223) | ~300 | #222 |
| 1c | HTTP Handlers | [#224](https://github.com/iberi22/xavier/issues/224) | ~300 | #223 |
| 2 | Policy engine | [#225](https://github.com/iberi22/xavier/issues/225) | ~250 | #222 |
| 3 | Memory consolidation | [#226](https://github.com/iberi22/xavier/issues/226) | ~300 | #223 |
| 4 | Code-graph integration | [#227](https://github.com/iberi22/xavier/issues/227) | ~400 | #222, #223 |
| 5 | Merge planner | [#228](https://github.com/iberi22/xavier/issues/228) | ~200 | #222-#227 |

---

_Xavier2 CEO 🧠 — ADR-006 — May 11, 2026_
