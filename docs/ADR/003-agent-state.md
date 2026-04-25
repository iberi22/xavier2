# ADR-003: Estado compartido — statics vs CliState

*Status: ACCEPTED | Date: 2026-04-25*

---

## Contexto

El `SessionSyncTask` usaba `static Mutex` y `static AtomicU64` para compartir estado entre el cron task (que escribe) y el `sync_check_handler` (que lee):

```rust
// session_sync_task.rs — ANTES (problemático)
static LAST_CHECK_TIMESTAMP_MS: AtomicU64 = AtomicU64::new(0);
static LAST_CHECK_LAG_MS: AtomicU64 = AtomicU64::new(0);
static LAST_CHECK_SAVE_OK_RATE: Mutex<f64> = Mutex::new(0.0);
static LAST_CHECK_MATCH_SCORE: Mutex<f64> = Mutex::new(0.0);
static LAST_CHECK_ACTIVE_AGENTS: AtomicUsize = AtomicUsize::new(0);
```

El code review identificó un **data race potencial**: `get_last_sync_result()` lee los valores atómicos sin coordinar con el writer (cron task), lo que puede producir snapshots inconsistentes (nuevo `lag_ms` con viejo `save_ok_rate`).

La pregunta era: ¿cómo manejamos estado compartido en Xavier2?

---

## Decisión

**Estado compartido vive en `CliState` via `Arc<T>`, NO como `static` globales.**

```rust
// ✅ CORRECTO — CliState ownership
struct CliState {
    agent_registry: Arc<dyn AgentLifecyclePort>,
    time_store: Option<Arc<dyn TimeMetricsPort>>,
    sync_state: Arc<SyncState>,  // ← estado compartido
}

// ✅ CORRECTO — Arc<RwLock<T>> para lectura concurrente
struct SyncState {
    lag_ms: u64,
    save_ok_rate: f64,
    match_score: f64,
    active_agents: usize,
    last_update: Instant,
}
```

Los `static` solo se usan para:
1. Singletons de proceso único (ej: global logger config)
2. Late-initialization de un recurso global que no puede vivir en `CliState` (y se inicializa UNA vez al startup, no se modifica después)

---

## Razón

1. **Data race:** incluso con `Mutex`, leer campos individuales sin lock produce snapshots inconsistentes
2. **Debugging:** statics globales son invisibles en el tipo de `CliState` — no puedes inspeccionar su estado
3. **Testing:** un `CliState` con `Arc<SyncState>` se puede inyectar en tests; statics globales no
4. **Rust idiomatic:** el sistema de ownership de Rust brilla cuando el estado tiene un owner claro

---

## Consequences

**Positivos:**
- Estado compartido visible en `CliState` — no surprises
- Testing fácil con estado inyectable
- No data races porque el owner es claro

**Negativos:**
- `CliState` puede crecer si hay muchos estados compartidos
- Los handlers existentes que leen `static` globales necesitan refactorizarse

---

## Notas

El fix para `get_last_sync_result()` requiere leer todos los valores bajo UN lock o usar un `RwLock<SyncState>` completo:

```rust
impl SyncState {
    pub fn snapshot(&self) -> SyncCheckResult {
        SyncCheckResult {
            lag_ms: self.lag_ms,
            save_ok_rate: self.save_ok_rate,
            // ... todos los campos leídos del mismo snapshot
        }
    }
}
```

Esto debe resolverse antes de v1.0.
