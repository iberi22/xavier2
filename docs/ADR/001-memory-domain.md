# ADR-001: QmdMemory como dominio central

*Status: ACCEPTED | Date: 2026-04-25*

---

## Contexto

Xavier tenía una definición de arquitectura hexagonal con:
- `ports/inbound/memory_port.rs` — trait `MemoryQueryPort` definido
- `app/memory_service.rs` — `impl MemoryQueryPort for QmdMemory` (creado en P0)
- `app/pattern_service.rs` — todos los métodos `todo!()`
- `app/security_service.rs` — implementación parcial

La pregunta era: ¿debería `QmdMemory` estar detrás de un port, o debería ser el dominio directamente?

---

## Decisión

**QmdMemory es el dominio directo.** `MemoryQueryPort` existe para permitir swappeo de storage backend (local SQLite → Cortex Enterprise cloud), pero `QmdMemory` NO es un adapter — ES el dominio.

El `QmdMemoryAdapter` creado en P0 NO wrappea `QmdMemory` como si fuera infraestructura — simplemente implementa el trait `MemoryQueryPort` para que la interfaz sea intercambiable.

---

## Razón

1. `QmdMemory` tiene toda la lógica de negocio de memoria (búsqueda híbrida, dedup, cache, timestamps)
2. Wrappear `QmdMemory` detrás de un port sin necesidad real = overhead sin beneficio
3. El port `MemoryQueryPort` se justifica SOLO para permitir múltiples backends de storage (local/cloud)
4. Los services en `app/` con `todo!()` eran código muerto que agregaba complejidad sin valor

---

## Consequences

**Positivos:**
- Dominio claro y concreto — toda la lógica de memoria en un lugar
- Decisiones de diseño centradas en funcionalidad, no en abstracciones
- Menos boilerplate

**Negativos:**
- Si en el futuro necesitamos swappear storage, hay un layer de indirección (el port)
- Podría considerarse "no puramente hexagonal" por dogma

---

## Notas

El `QmdMemoryAdapter` en `src/app/qmd_memory_adapter.rs` делегирует todo a `QmdMemory` — no agrega lógica, solo hace el trait impl posible. Si nunca se necesita swappear storage, este adapter es innecesario y puede ser removido sin impacto en funcionalidad.
