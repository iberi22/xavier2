# ADR-004: Cortex Enterprise Cloud Plugin

*Status: PROPOSED | Date: 2026-04-25*

---

## Context

El CEO pidió explorar la conexión de Xavier a Cortex Enterprise (cloud hosted). Actualmente:
- Xavier corre como memory backend local en puerto 8006
- Cortex tiene su propio sistema de memoria (v0.4.1)
- NO existe plugin, adapter, ni integración definida
- La pregunta: ¿cómo conectamos Xavier a Cortex Enterprise cloud?

---

## Decision

**Propuesta: Cortex como storage backend adapter de Xavier.**

Xavier es el "brain" local. Cortex Enterprise es el storage/backend externo. La integración es: Xavier guarda localmente Y sincroniza a Cortex cloud.

```
┌─────────────────────────────────────────┐
│           AGENT (via OpenClaw)         │
├─────────────────────────────────────────┤
│ Xavier (local brain, :8006)          │
│  • QmdMemory (dominio local)           │
│  • AgentRegistry (sesiones)            │
│  • TimeMetrics (operación)             │
│  • AutoVerifier (ciclo save/verify)    │
├─────────────────────────────────────────┤
│         Sync →                         │
├─────────────────────────────────────────┤
│ Cortex Enterprise (cloud storage)       │
│  • Persistent memory backup             │
│  • Multi-agent shared memory           │
│  • Enterprise search/analytics         │
└─────────────────────────────────────────┘
```

**Interfaz de sync:**
- `POST /xavier/sync/push` — push local memory a Cortex
- `POST /xavier/sync/pull` — pull Cortex memory a local
- `SyncState` en `CliState` — coordina sync status

**Environment variables necesarias:**
```bash
CORTEX_ENTERPRISE_URL=https://cortex.company.com  # Cortex cloud base URL
CORTEX_TOKEN=<api_token>                           # Auth token
CORTEX_SYNC_INTERVAL_MS=300000                    # 5 min default
CORTEX_AUTO_SYNC=true                             # enable sync
```

---

## Reason

1. **No reinventar la rueda** — Cortex ya existe como storage empresarial. Xavier no necesita replicar funcionalidad de storage distribuido.
2. **Arquitectura pragmática** — Xavier es el "thinking engine", Cortex es el "storage engine". Separación de concerns.
3. **Mercado Enterprise** — empresas ya tienen Cortex. Integración = venta más fácil.

---

## Consequences

**Positivos:**
- Xavier puede operar offline (local memory) y sincronizar cuando hay conexión
- Shared memory entre múltiples agentes via Cortex
- Enterprise-ready desde el inicio

**Negativos:**
- La integración Cortex Enterprise real aún no existe — esto es una propuesta
- Sync puede crear conflictos de merge si ambos stores se modifican simultáneamente
- Requiere API de Cortex Enterprise documentada

---

## Open Questions

1. ¿Cortex tiene API documented para recibir memory updates desde un cliente externo?
2. ¿El sync es unidireccional (Xavier → Cortex) o bidireccional (Xavier ↔ Cortex)?
3. ¿Qué pasa cuando hay conflicto de versión (local vs cloud)?

---

## Notes

Este ADR está en estado **PROPOSED** — necesita validación técnica con el equipo de Cortex antes de implementación.

Acciones:
- [ ] Confirmar con Leonardo si Cortex Enterprise tiene API documented
- [ ] Diseñar formato de sync payload
- [ ] Definir estrategia de conflict resolution
- [ ] Crear issue en GitHub para trackear
