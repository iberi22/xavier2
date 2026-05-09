# SRC Configuration - Xavier

**Versión:** 1.0
**Fecha:** 2026-04-14
**Proyecto:** Xavier Cognitive Memory System

---

## 1. Propósito

Este archivo define la configuración del sistema SRC (Software Requirements Specification) para Xavier, un motor de memoria cognitiva para agentes IA.

---

## 2. Estructura SRC

```
XAVIER/
├── docs/
│   ├── SRC/
│   │   ├── index.md              ← Entry point
│   │   ├── REQUIREMENTS.md       ← Requisitos funcionales
│   │   ├── NON-FUNCTIONAL.md    ← Rendimiento, seguridad
│   │   ├── INTERFACES.md        ← APIs, integraciones
│   │   ├── DATABASE.md           ← Modelo de datos
│   │   └── GLOSSARY.md          ← Definiciones
│   ├── site/                     ← Starlight docs site
│   ├── ARCHITECTURE/
│   └── DEPLOY/
└── .gitcore/
    └── (configuración existente)
```

---

## 3. Módulos Principales (src/)

| Módulo | Descripción |
|--------|-------------|
| `src/agents/` | System 1-2-3 cognitive layers |
| `src/memory/` | QMD Memory + Belief Graph |
| `src/server/` | HTTP API + MCP Server |
| `src/workspace.rs` | Multi-tenant isolation |
| `src/sync/` | Chunk-based sync protocol |
| `src/security/` | E2E encryption + pattern detection |

---

## 4. Convenciones de Requirements

### IDs de Requisitos

```
XAVIER-[TIPO]-[NÚMERO]

Ejemplos:
XAVIER-FUN-001  → Requisito funcional #1
XAVIER-NF-001   → Requisito no funcional #1
XAVIER-INT-001  → Interface #1
XAVIER-DB-001   → Entidad de base de datos #1
```

### Estados

| Estado | Descripción |
|--------|-------------|
| `draft` | Borrador |
| `review` | En revisión |
| `approved` | Aprobado |
| `implemented` | Implementado |
| `validated` | Validado con tests |

---

## 5. Integración con Planning

Los requisitos SRC se linkean con `.gitcore/planning/TASK.md`:

```
| ID            | Requirement          | Status      | Issue |
|---------------|----------------------|-------------|-------|
| XAVIER-FUN-001 | Memory persistence | implemented | #123  |
```

---

## 6. Feature Tracking

Ver `.gitcore/features.json` para tracking oficial.

---

## 7. Metadata

```yaml
src_version: "1.0"
project: "Xavier"
version: "0.4.1"
created: "2026-04-14"
```

---

*Configuración SRC para Xavier*
