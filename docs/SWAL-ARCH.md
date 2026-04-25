# SWAL Memory Architecture — Xavier2

*Versión: 0.1 | Última actualización: 2026-04-25*

---

## 1. Principio central

> **Dominio primero. Abstracciones solo cuando hay swappeo real.**

El dominio de Xavier2 es `QmdMemory` — toda la lógica de memoria está ahí. No se wrappea por wrapear. Los ports existen SOLO cuando hay intención real de tener múltiples implementaciones.

---

## 2. Capas reales (Xavier2)

```
┌─────────────────────────────────────────────────────┐
│              INBOUND ADAPTERS                       │
│  Handlers HTTP (cli.rs, routes.rs)                  │
│  Session handlers, agent handlers, metric handlers  │
├─────────────────────────────────────────────────────┤
│                  DOMINIO                             │
│  QmdMemory (memoria híbrida vector+keyword)        │
│  SimpleAgentRegistry (registro de agentes)          │
│  TimeMetricsStore (métricas de operación)          │
│  SecurityService (detección prompt injection)      │
│  AutoVerifier (ciclo save/retrieve verification)  │
├─────────────────────────────────────────────────────┤
│                   PORTS                              │
│  SOLO donde hay swappeo real de implementación:     │
│  • MemoryQueryPort → si necesitamos storage        │
│    intercambiable (local SQLite / Cortex cloud)    │
│  • AgentLifecyclePort → si necesitamos múltiples  │
│    registries (in-memory / Redis / cloud)         │
│  • HealthCheckPort → si necesitamos diferentes    │
│    health checkers (local / remote)                │
│  NO donde solo hay una implementación concreta      │
├─────────────────────────────────────────────────────┤
│                 OUTBOUND ADAPTERS                    │
│  Implementaciones concretas de cada port:          │
│  • VecSqliteMemoryStore → StoragePort adapter      │
│  • HttpHealthAdapter → HealthCheckPort adapter    │
│  • TimeMetricsAdapter → TimeMetricsPort (si se usa)│
│  • QmdMemoryAdapter → MemoryQueryPort (si se usa)  │
└─────────────────────────────────────────────────────┘
```

---

## 3. Estado compartido

**Regla:** Todo estado compartido entre tasks (cron + HTTP handler) vive en `CliState` via `Arc<T>`, NO como `static Mutex` global.

```rust
// ✅ CORRECTO
struct CliState {
    agent_registry: Arc<dyn AgentLifecyclePort>,
    health_check: Arc<dyn HealthCheckPort>,
}

// ❌ INCORRECTO — data race potential
static LAST_CHECK_LAG_MS: AtomicU64 = ...;
static LAST_CHECK_SAVE_OK_RATE: Mutex<f64> = ...;
```

---

## 4. Cuándo crear un port

Un port se crea cuando:

1. **Hay +1 implementación real** — no预感, no "por si acaso"
2. **Necesitamos mockear para tests** — testing real que requiere múltiples scenarios
3. **El dominio tiene lógica compleja** que debe estar aislada del handler

Un port NO se crea cuando:

1. Solo hay una implementación y no se espera otra
2. Es "por si después necesitamos cambiar"
3. El overhead de mantenerlo no justifica el beneficio

---

## 5. Proyecto:docs/

```
docs/
├── SWAL-ARCH.md        ← Este archivo
├── ADR/                ← Architecture Decision Records
│   ├── 001-memory-domain.md
│   ├── 002-ports-when.md
│   ├── 003-agent-state.md
│   └── 004-cortex-plugin.md
└── TODO.md             ← Pendientes arquitectónicos
```

---

## 6. Stack tecnológico (Xavier2)

| Componente | Tecnología |
|-----------|-------------|
| HTTP | axum 0.8 + tower |
| Memory | QmdMemory (vector + keyword search) |
| Storage | SQLite (rusqlite + sqlite-vec) |
| Security | prompt injection detection |
| Serialization | serde + bincode |
| Concurrency | parking_lot::Mutex, tokio |
| CLI | clap + tracing |

---

## 7. Conventions

- **Módulo dominio:** `src/memory/qmd_memory.rs` — toda lógica de memoria
- **Módulo agents:** `src/coordination/agent_registry.rs` — registro y lifecycle
- **Módulo metrics:** `src/time/mod.rs` — métricas operativas
- **Ports:** `src/ports/inbound/` y `src/ports/outbound/`
- **Adapters:** `src/adapters/inbound/` y `src/adapters/outbound/`
- **Handlers:** `src/cli.rs` (routing) + `src/routes.rs` (handlers SEVIER2)

---

*Este documento evoluciona. Cada cambio arquitectónico significativo requiere un ADR.*
