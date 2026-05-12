# 🧠 CODE REVIEW — Xavier v0.6.0-beta
## Mayo 2026 | Xavier2 CEO + Codex Deep Review

> **Goal:** Bordarlo al 100% — producción-grade, zero papercuts, auditoría externa lista.

---

## 📊 MÉTRICAS GENERALES

| Métrica | Valor | Estado |
|----------|-------|--------|
| Archivos `.rs` en `src/` | 237 | — |
| Líneas de código | 61,753 | — |
| Tests | 571 (427 lib + 11 bin + 136 integration) | ✅ Pasando |
| Clippy (all-targets) | 0 warnings, 0 errors | ✅ |
| Cargo check | Clean | ✅ |
| Dependabot PRs abiertos | 0 | ✅ |
| Issues Jules abiertos | 0 | ✅ |

---

## 🔴 CRITICAL (BLOQUEANTES PRODUCCIÓN)

### C1 — 682 `.unwrap()` calls
**Severidad:** 🔴 CRITICAL  
**Riesgo:** Panics en runtime que tumban el servidor entero.

| Archivo | Unwraps | Acción |
|---------|---------|--------|
| `security/prompt_guard.rs` | 82 | Sanitizar — muchos son Regex::new que se pueden precompilar |
| `server/mcp_server.rs` | 62 | Refactorizar — propagar errores con `?` y Result |
| `memory/sqlite_vec_store.rs` | 52 | Refactorizar — usar anyhow/thiserror en DB ops |
| `server/v1_api.rs` | 36 | Refactorizar — handlers deben devolver Response, no panic |
| `coordination/message_bus.rs` | 32 | Refactorizar |
| `adapters/outbound/vec/pattern_adapter.rs` | 26 | Refactorizar |
| `memory/belief_graph.rs` | 21 | Refactorizar |

**Plan:** Eliminar TODOS los unwrap en código de producción. Solo permitir en:
- `main.rs` / entry points (donde panic es aceptable)
- Tests
- Construcción de Regex estáticos (reemplazar con `lazy_static!` o `OnceLock`)

**Estimación:** 8-12 horas

### C2 — 59 `.expect()` calls
**Severidad:** 🔴 CRITICAL  
**Riesgo:** Idem unwrap — panics con mensaje, pero sigue tumbando el server.

**Plan:** Mismo approach que unwrap. Convertir a propagación de errores con `?` + `context()`.

**Estimación:** 3-5 horas

### C3 — 3 bloques `unsafe`
**Severidad:** 🟡 HIGH (justificados pero necesitan auditoría)

| Archivo | Línea | Uso |
|---------|-------|-----|
| `sqlite_vec_store.rs` | 208-209 | FFI: función SQLite extern "C" |
| `coordination/mod.rs` | 937 | Unsafe block |
| `utils/crypto.rs` | 15 | `from_utf8_unchecked` |

**Plan:** Documentar cada bloque, añadir `// SAFETY:` comments, auditar condiciones.

**Estimación:** 1-2 horas

---

## 🟡 HIGH (DEUDA TÉCNICA)

### H1 — Archivos monolíticos (>1000 líneas)
**Severidad:** 🟡 HIGH  
**Riesgo:** Inmantenibles, difíciles de testear, acoplan concerns.

| Archivo | Líneas | Acción |
|---------|--------|--------|
| `cli.rs` | 3,162 | Split en subcomandos: `cli/memory.rs`, `cli/agent.rs`, `cli/chronicle.rs` |
| `sqlite_vec_store.rs` | 2,795 | Separar: vec_store, search, indexing, maintenance |
| `system3.rs` | 2,224 | Split en módulos por responsabilidad |
| `mcp_server.rs` | 1,999 | Separar handlers por recurso |
| `workspace.rs` | 1,634 | Extraer tipos a `workspace/types.rs` |
| `message_bus.rs` | 1,257 | Separar routing, delivery, subscription |
| `server/http.rs` | 1,270 | Extraer middleware, handlers, config |

**Estimación:** 15-20 horas (fase 2)

### H2 — ~30 TODOs de Dead Code
**Severidad:** 🟡 HIGH  

Categorías:
| Categoría | Archivos | Acción |
|-----------|----------|--------|
| Dead code sin wire | `file_indexer.rs` (8), `scheduler/mod.rs` (3), `secrets/mod.rs` (4), `memory/mod.rs`, `checkpoint/mod.rs` | Eliminar o implementar |
| HexArch violations | `qmd_memory_adapter.rs`, `security_service.rs` | Crear puertos y adaptadores |
| Jules pending impl | `devlog/generator.rs`, `devlog/mod.rs` | Completar implementación |
| TUI placeholders | `board.rs`, `card.rs`, `state.rs` | Eliminar o completar |

**Plan:** Fase 1: eliminar dead code confirmado. Fase 2: completar o eliminar.

**Estimación:** 4-6 horas

### H3 — 647 `.clone()` calls
**Severidad:** 🟡 HIGH  
**Riesgo:** Overhead de memoria, GC pressure, latencia.

**Plan:** Auditar clones en hot paths:
- Pasar references en vez de owned values
- Usar `Arc`/`Rc` donde tiene sentido
- `Cow<'_, str>` para strings

**Estimación:** 5-8 horas

### H4 — 30 `#[allow(...)]` attributes
**Severidad:** 🟡 MEDIUM  
**Riesgo:** Suprimen warnings legítimos, esconden bugs.

**Plan:** Revisar cada uno, eliminar los innecesarios, arreglar el código subyacente donde sea posible.

**Estimación:** 2-3 horas

---

## 🟢 MEDIUM (MEJORAS)

### M1 — Hexagonal Architecture Gaps
Algunos adaptadores dependen directamente de implementaciones concretas en vez de puertos:
- `qmd_memory_adapter.rs` → debería usar `MemoryPort`
- `security_service.rs` → debería usar `SecurityPort`

**Estimación:** 3-4 horas

### M2 — Test Coverage Gaps
- 571 tests pasando, pero cobertura desigual
- `cli.rs` (3,162 líneas) tiene poca cobertura de tests de integración
- `mcp_server.rs` (1,999 líneas) necesita tests de protocolo
- `prompt_guard.rs` necesita tests de fuzzing

**Estimación:** 6-8 horas

### M3 — Documentación de API
- Endpoints HTTP documentados con `utoipa` → ✅
- Faltan ejemplos de uso en docs/api/
- MCP tools necesitan mejor descripción

**Estimación:** 3-4 horas

### M4 — Dependabot Vulnerabilities
5 vulnerabilidades reportadas (3 moderate, 2 low). Revisar si son falsos positivos o requieren acción.

**Estimación:** 1-2 horas

---

## 🔵 LOW (PULIDO)

### L1 — Comentarios en español/inglés mezclados
Algunos comentarios están en español, otros en inglés. Estandarizar a inglés.

**Estimación:** 2-3 horas

### L2 — Naming inconsistencies
- `qmd_memory` vs `memory` — nomenclatura confusa
- `system1`, `system2`, `system3` — nombres no descriptivos

**Estimación:** 1-2 horas

### L3 — Logging levels
Algunos `debug!` deberían ser `trace!`, algunos `info!` deberían ser `debug!`.

**Estimación:** 1 hora

### L4 — Cargo.toml cleanup
- Dependencias opcionales no documentadas
- Feature flags poco claros (`ci-safe = []`?)

**Estimación:** 1 hora

---

## 🗺️ PLAN DE MEJORA POR FASES

### 🚀 FASE 1 — CRITICAL FIXES (AHORA MISMO)
| Tarea | Esfuerzo | Prioridad |
|-------|----------|-----------|
| **1.1** Eliminar unwrap en `prompt_guard.rs` (82) — precompilar Regex | 2h | 🔴 |
| **1.2** Eliminar unwrap en `mcp_server.rs` (62) — propagar errores | 3h | 🔴 |
| **1.3** Eliminar unwrap en `v1_api.rs` (36) — convertir handlers | 2h | 🔴 |
| **1.4** Eliminar unwrap en `sqlite_vec_store.rs` (52) — DB errors | 3h | 🔴 |
| **1.5** Auditoría unsafe + SAFETY comments | 1h | 🟡 |
| **1.6** Eliminar dead code confirmado (~15 TODOs) | 2h | 🟡 |
| **1.7** Revisar expect() calls (59) | 1h | 🔴 |
| **TOTAL FASE 1** | **14h** | |

### ⚡ FASE 2 — QUICK WINS (ESTA SEMANA)
| Tarea | Esfuerzo | Prioridad |
|-------|----------|-----------|
| **2.1** Eliminar unwrap restantes (resto de archivos, ~420) | 6h | 🔴 |
| **2.2** Split `cli.rs` en submódulos | 4h | 🟡 |
| **2.3** Split `sqlite_vec_store.rs` | 3h | 🟡 |
| **2.4** Split `system3.rs` | 3h | 🟡 |
| **2.5** Auditar `#[allow(...)]` attributes | 2h | 🟡 |
| **2.6** HexArch adapter fixes | 3h | 🟡 |
| **TOTAL FASE 2** | **21h** | |

### 🏗️ FASE 3 — REFACTORS (ESTE MES)
| Tarea | Esfuerzo | Prioridad |
|-------|----------|-----------|
| **3.1** Split archivos monolíticos restantes (>500 líneas) | 8h | 🟡 |
| **3.2** Eliminar clones innecesarios (~647 audits) | 6h | 🟡 |
| **3.3** Completar DevLog SSG (Jules pendiente) | 4h | 🟢 |
| **3.4** Mejorar test coverage (mcp_server, cli, prompt_guard) | 8h | 🟢 |
| **3.5** Documentar API y MCP tools | 3h | 🟢 |
| **TOTAL FASE 3** | **29h** | |

### ✨ FASE 4 — POLISH (PRÓXIMO SPRINT)
| Tarea | Esfuerzo | Prioridad |
|-------|----------|-----------|
| **4.1** Estandarizar comentarios a inglés | 2h | 🔵 |
| **4.2** Limpiar naming (`system1/2/3`, `qmd_memory`) | 2h | 🔵 |
| **4.3** Ajustar logging levels | 1h | 🔵 |
| **4.4** Cargo.toml cleanup + feature docs | 1h | 🔵 |
| **4.5** Dependabot vuln audit | 2h | 🔵 |
| **TOTAL FASE 4** | **8h** | |

---

## 📈 GRAN TOTAL

| Fase | Horas | Impacto |
|------|-------|---------|
| Fase 1 — Critical | 14h | 🔴 Bloqueante para producción |
| Fase 2 — Quick Wins | 21h | 🟡 Deuda técnica inmediata |
| Fase 3 — Refactors | 29h | 🟢 Mejora estructural |
| Fase 4 — Polish | 8h | 🔵 Calidad final |
| **TOTAL** | **72h** | ~2 semanas full-time |

---

## 🎯 QUICK WINS (< 1 HORA CADA UNO)

1. **Precompilar Regex en prompt_guard** — 45 min, elimina 50+ unwraps
2. **Eliminar dead code en file_indexer.rs** — 30 min, 8 TODOs resueltos
3. **SAFETY comments en unsafe blocks** — 30 min, 3 bloques documentados
4. **Eliminar TODO dead code en scheduler/mod.rs** — 20 min
5. **Eliminar TODO dead code en secrets/mod.rs** — 20 min
6. **Agregar `.context()` a los 59 expects** — 45 min
7. **Convertir handlers v1_api de unwrap a Result** — 45 min
8. **Lazy static Regex en security/layers** — 30 min

---

_Revisión por Xavier2 CEO + Codex | Mayo 2026_
