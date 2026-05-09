# ADR-002: Cuándo crear un port

*Status: ACCEPTED | Date: 2026-04-25*

---

## Contexto

El codebase de Xavier tenía 6 ports definidos:
- `MemoryQueryPort` (inbound)
- `PatternDiscoverPort` (inbound)
- `SecurityScanPort` (inbound)
- `StoragePort` (outbound)
- `EmbeddingPort` (outbound)
- `AgentRuntimePort` (outbound)

Ninguno estaba siendo usado por los handlers. Todos los application services (`app/*.rs`) tenían métodos `todo!()`.

La pregunta era: ¿cuándo vale la pena crear un port?

---

## Decisión

**Ports pragmáticos — no dogmáticos.**

Un port se crea cuando se cumplen al menos 2 de estos 3 criterios:

| Criterio | Descripción |
|----------|-------------|
| **Múltiples implementaciones reales** | Hay o se planifica +1 implementación del mismo contrato |
| **Testing real** | Necesitamos mockear para tests de integración/unitarios que requieren múltiples scenarios |
| **Dominio complejo aislado** | La lógica de negocio del port es lo suficientemente compleja como para merecer su propio módulo |

**NO se crea un port cuando:**
- Solo hay una implementación concreta
- Es "por si acaso en el futuro"
- El overhead de mantenerlo no justifica el beneficio actual

---

## Aplicación en Xavier

| Port | ¿Crear/Eliminar? | Razón |
|------|-------------------|-------|
| `MemoryQueryPort` | ✅ **Mantener** | Prepárate para Cortex Enterprise cloud + local SQLite |
| `SecurityScanPort` | ⚠️ **Evaluar** | Wiring roto en cli.rs — o arreglar bien o eliminar |
| `AgentLifecyclePort` | ⚠️ **Depende** | Útil si hay múltiples registries (in-memory, Redis, cloud) |
| `HealthCheckPort` | ❌ **Eliminar** | Solo hay HTTP adapter — el port es overhead |
| `PatternDiscoverPort` | ❌ **Eliminar** | Stub con `todo!()`, no hay implementaciones |
| `EmbeddingPort` | ❌ **Eliminar** | Stub con `todo!()`, no hay implementaciones |
| `AgentRuntimePort` | ❌ **Eliminar** | Stub con `todo!()`, no hay implementaciones |

---

## Consequences

**Positivos:**
- Menos código muerto
- Cada port tiene una razón de existir verificable
- Desarrollo más rápido — menos boilerplate

**Negativos:**
- Si el negocio evoluciona y ahora necesita un port que no existe, hay que crearlo sobre la marcha
- Puede haber presión para "no crear ports" incluso cuando sí son necesarios

---

## Notes

La decisión se revisa cuando:
- Se agrega un nuevo storage backend (ej: Cortex Enterprise)
- Se necesitan integration tests con múltiples scenarios
- El dominio crece y un componente se vuelve suficientemente complejo
