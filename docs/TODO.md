# docs/TODO.md — Xavier2 Status (Updated 2026-04-26)

---

## ✅ RESOLVED (this session)

| # | Task | Resolution |
|---|------|-----------|
| #89 | SessionContext dead code | CLOSED — it IS used (cli.rs:1650) |
| #92 | Security service port stub | CLOSED — removed in 4c9006a, inherent methods |
| #95 | SessionSyncTask bypass ports | CLOSED — intentional per ADR-002 |
| Data race | get_last_sync_result() | FIXED in commit 4c9006a (RwLock<SyncState>) |
| Dead code | app/memory_service.rs | DELETE PENDING |
| Dead code | app/pattern_service.rs | DELETE PENDING |

---

## 🔴 CRITICAL BLOCKERS

| # | Task | Status | Notes |
|---|------|--------|-------|
| 1 | GitHub push blocked | BLOQUEADO | Groq API key in eb0eb43 history |
| 2 | Docker OOM | BLOQUEADO | 4GB RAM no enough for cargo check |

---

## 🟡 MEDIUM PRIORITY — Open Issues

| # | Issue | Priority |
|---|-------|----------|
| #74 | Test script wrong payload (agent register) | 🔄 IN-PROGRESS |
| #75 | Missing /xavier2/agents/{id}/unregister | 🔄 IN-PROGRESS |
| #76 | Graceful shutdown in SessionSyncTask cron | 🔄 IN-PROGRESS |
| #77 | sync_check_handler creates redundant SessionSyncTask | 🔄 IN-PROGRESS |
| #78 | Docker missing env vars | ✅ FIXED (5538995) |
| #79 | estimate_index_lag() stub | 🔄 IN-PROGRESS |
| #80 | Thresholds hardcoded (not env vars) | 🔄 IN-PROGRESS |
| #81 | Integration tests are empty scaffolds | P3 |
| #82 | Dead code: create_router() and TIME_STORE | P2 |
| #83 | AutoVerifier max 0.5 vs 0.8 threshold | 🔄 IN-PROGRESS |
| #84 | Duplicate session_event_handler definitions | P3 |
| #86 | CliState.time_store never read | P2 |
| #87 | verify_save_handler returns 404 (not wired) | 🔄 IN-PROGRESS |
| #88 | AgentHeartbeatPayload never constructed | P3 |
| #90 | Hexagonal arch refactor incomplete | P3 |

---

## v1.0 Release Criteria

- [x] cargo check --lib → 0 errors ✅ (2 warnings only)
- [ ] Todos los endpoints SEVIER2 verificados con curl
- [x] Data race fix commiteado ✅ (4c9006a)
- [ ] Dead code ports/app eliminados
- [x] docs/ADR/ creado y commiteado ✅ (e71fe1a)
- [ ] GitHub push exitoso (BLOQUEADO)
- [x] Docker image build + run exitoso ✅
- [ ] Integration tests compilan (ignore=True)

---

## Post v1.0

| Task | Priority | Status |
|------|----------|--------|
| Cortex Enterprise plugin (ADR-004) | HIGH | PROPOSED |
| Integration tests reales (no scaffolds) | MEDIUM | P3 |
| Gestures/Gestalt integration | MEDIUM | 40% |
| BPM proactive system | HIGH | ACTIVE |

---

*Updated: 2026-04-26 16:50 GMT-5*
*12-agent swarm launched to resolve remaining issues*
