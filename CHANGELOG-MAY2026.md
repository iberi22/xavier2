# Changelog — Mayo 2026

## Visión General

Sprint de mayo 2026 enfocado en resolver deudas técnicas, mejorar estabilidad y preparar la base para fases futuras de SEVIER2.

## Issues Resueltos

| # | Título | PR | Tipo |
|---|--------|-----|------|
| #77 | sync_check_handler crea SessionSyncTask redundante | #158 | P2 |
| #79 | estimate_index_lag() es stub | #158 | P2 |
| #80 | Thresholds hardcoded en SessionSyncTask | #152 | P2 |
| #81 | Tests de integración vacíos | #151 | P2 |
| #83 | AutoVerifier score 0.5 vs threshold 0.8 | #153 | P2 |
| #87 | verify_save_handler nunca vinculado al router | #157 | P1 |
| #91 | Handlers bypass MemoryQueryPort | #161 | arch |
| #93 | TimeMetricsStore sin inbound port (OnceLock) | #162 | arch |
| #94 | SimpleAgentRegistry sin inbound port | #159 | arch |
| #125 | unwrap/expect en http.rs | #156 | fix |
| #127 | Constantes mágicas en gating.rs | #154 | refactor |

## Detalle de Cambios

### #158 — SessionSyncTask improvements
- `estimate_index_lag()` ahora retorna lag real
- `get_last_sync_result()` cacheado para evitar tareas redundantes
- Thresholds configurables via `SEVIER2_INDEX_LAG_THRESHOLD` y `SEVIER2_SESSION_LAG_THRESHOLD`

### #152 — Configurable thresholds
- `SEVIER2_INDEX_LAG_THRESHOLD` (default: 1000)
- `SEVIER2_SESSION_LAG_THRESHOLD` (default: 100)
- `SEVIER2_SYNC_INTERVAL_SECS` (default: 60)

### #151 — Real test logic
- `tests/sevier2_stress_test.rs` ahora tiene lógica real de stress testing
- `SessionSyncTask::new()` requiere ahora `session_store: Arc<dyn SessionStore>`
- Tests de integración con `MockSessionSyncTask` y `MockSevier2Client`

### #153 — AutoVerifier score fix
- Partial match ahora usa `match_score * 1.5` boosting
- Score 0.5 → boosted 0.75, aún bajo threshold 0.8
- Problema abierto: necesita ajuste adicional en matching algorithm

### #157 — verify_save_handler wired
- Handler registrado en `cli.rs::router()`
- Confirma que `verify_save_handler` y `get_sevier2_status_handler` eran el mismo endpoint

### #161 — MCP wired to MemoryQueryPort
- `mcp_server.rs` ahora usa `Arc<dyn MemoryQueryPort>` en vez de `QmdMemory` directo
- `mcp_memory_port.rs` implementa el trait `MemoryQueryPort`

### #159 — AgentRegistry port wiring
- `CliState::agent_registry` ahora usa `Arc<dyn AgentLifecyclePort>`
- `unregister_agent_handler.rs` recibe `Arc<dyn AgentRegistry>` via DI

### #154 — Magic constants extracted
- `RETRIEVAL_BM25_WEIGHT`, `RETRIEVAL_SEMANTIC_WEIGHT`, etc. en `src/retrieval/config.rs`
- `gating.rs` ahora importa de `config`

### #156 — Error handling in http.rs
- `unwrap()` → `map_err()`
- `expect()` → `with_context()`

---

## Hallazgos del Sprint

### Pipeline Issues Descubiertos

1. **OpenCode sandbox fails on Windows** — "valid workspace SID" error
2. **Agent exits 0 but makes no changes** — Silent failure
3. **PR merge conflicts from stale master** — Need fetch+merge before push
4. **Multiple agents on same repo overwrite files** — Need worktrees
5. **ProviderModelNotFoundError** — Model name format wrong (`MiniMax-M2.7` exact)

### Soluciones Implementadas

- `docs/MULTI_AGENT_PIPELINE.md` — Guía completa de desarrollo multi-agente
- `coding-agent/SKILL.md` actualizado con errores conocidos
- Verificación obligatoria post-agente (`git diff HEAD --stat`)

---

## Próximas Fases

### Fase 1: SEVIER2 Core Stability
- #78 — Docker: missing env vars (P1)
- #75 — Missing /agents/{id}/unregister endpoint (P1)
- #74 — Wrong payload for /agents/register (P1)

### Fase 2: Architecture Refinement
- #90 — Hexagonal Architecture: untangle Ports from Infrastructure
- #137 — qmd_memory.rs 105KB modularization (PERF)
- #149 — Gestalt Rust → Xavier2 migration

### Fase 3: Context Regeneration
- #100, #101 — Context Regeneration System Phases 0-5

---

## Estadísticas del Sprint

| Métrica | Valor |
|---------|-------|
| Issues resueltos | 11 |
| PRs mergeados | 9 |
| Líneas añadidas | ~600 |
| Docs creados | 3 |
| Fases identificadas | 3 |

---

## V1 Release Stabilization — 2026-05-05

Esta tanda cierra la fase de estabilización pre-v1 con enfoque en calidad de código, consistencia de configuración y documentación alineada al producto real.

### Cambios Estructurales

- **Config canónica**: `config/xavier2.config.json` como fuente única de runtime no-secretos, con `src/settings.rs` para deserialización + `apply_to_env()`
- **Contrato CLI/HTTP/TUI unificado**: todas las superficies usan `XAVIER2_URL` con fallback al JSON config via `Xavier2Settings::client_base_url()`
- **Storage isolation**: workspaces aislados correctamente en sqlite_store, sqlite_vec_store, workspace.rs — sin fuga de memorias entre workspaces

### Calidad

- `cargo clippy -D warnings` pasa limpio en todo el workspace
- `cargo test --workspace` — 5 tests + 2 doctests OK
- `cargo build --release` — xavier2 + xavier2-tui compilan
- `cargo fmt --check` — sin issues

### Panel UI

- Biome 2 compatibilidad
- Accesibilidad mejorada en App.tsx, DecisionCard, ProjectCard
- CSS y tests alineados

### Scripts

- `release-smoke.ps1` y `release-smoke.sh`: usan `XAVIER2_URL`/`XAVIER2_TOKEN`, sin endpoints hardcodeados
- Pre-commit hooks actualizados

### Documentación

- `docs/FEATURE_STATUS.md` — matriz de readiness 0.6-beta
- README, CLI_REFERENCE, quick-start, installation, API reference sincronizados
- `.github/issues/` — breakdown de EPIC + 10 tareas de estabilización

### Estadísticas

| Métrica | Valor |
|---------|-------|
| Commits en rama | 18 |
| Archivos modificados | 78 |
| Líneas añadidas | +7,151 |
| Líneas eliminadas | -3,948 |
| Nuevos archivos | 14 |
| Gates de calidad | 5/5 |

_Release tag: 0.6.0-beta_

_Fecha: 2026-05-02_