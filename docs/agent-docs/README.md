# 📚 Agent Docs - Sistema de Archivado Inteligente

> **"Todo documento generado por IA debe ser trazable y analizable"**

## 📚 Navegación Rápida

### Por Tipo de Documento

- **📐 Especificaciones** - [specs/README.md](./specs/README.md)
  - `SPEC_CONTEXT_PROTOCOL.md` - Protocolo de contexto stateless
  - `SPEC_EVOLUTION_PROTOCOL.md` - Sistema de evolución del protocolo
  - `SPEC_HUMAN_LAYER_PROTOCOL.md` - Human-in-the-loop para operaciones críticas
  - `SPEC_MICRO_AGENTS.md` - Sistema de personas por rol
  - `SPEC_NON_BLOCKING_EXECUTION.md` - Ejecución no bloqueante

- **📊 Análisis** - [analysis/README.md](./analysis/README.md)
  - `ANALYSIS_TELEMETRY_SCALE.md` - Escalabilidad de telemetría
  - `ANALYSIS_WORKFLOW_RUST_MIGRATION.md` - Migración a Rust

- **📈 Reportes** - [reports/README.md](./reports/README.md)
  - `REPORT_GLOBAL_SELFHEALING_DIAGNOSIS.md` - Diagnóstico de auto-sanación
  - `REPORT_PROJECT_AUDIT.md` - Auditoría de proyecto
  - `REPORT_WORKFLOW_OPTIMIZATION.md` - Optimización de workflows

- **🔬 Investigación** - [research/README.md](./research/README.md)
  - `RESEARCH_LLM_TOOL_CALLING.md` - Tool calling en LLMs
  - `RESEARCH_SELFHEALING_CICD.md` - CI/CD auto-sanador
  - `RESEARCH_STACK_CONTEXT.md` - Contexto de stack tecnológico

- **💡 Prompts** - [prompts/README.md](./prompts/README.md)
  - `PROMPT_PROTOCOL_INSTALLER.md` - Instalación de protocolo

---

## 🗂️ Estructura de Carpetas

```
docs/agent-docs/
├── 📋 README.md                    # Este archivo
├── 🔧 .gitkeep                     # Mantener carpeta en git
│
├── 📁 specs/                       # Especificaciones técnicas
│   ├── SPEC_*.md                   # Documentos de diseño
│   └── README.md                   # Índice de specs
│
├── 📁 prompts/                     # Prompts para agentes
│   ├── PROMPT_*.md                 # Prompts reutilizables
│   └── README.md                   # Catálogo de prompts
│
├── 📁 research/                    # Investigaciones
│   ├── RESEARCH_*.md               # Análisis técnicos
│   └── README.md                   # Índice de research
│
├── 📁 sessions/                    # Archivo de sesiones ⭐ NUEVO
│   ├── YYYY-MM/                    # Organizado por mes
│   │   ├── SESSION_*.md            # Sesiones archivadas
│   │   └── METRICS.json            # Métricas mensuales
│   └── README.md                   # Cómo usar sesiones
│
├── 📁 reports/                     # Reportes generados
│   ├── REPORT_*.md                 # Auditorías, análisis
│   └── README.md                   # Índice de reportes
│
├── 📁 analysis/                    # Análisis y optimizaciones
│   ├── ANALYSIS_*.md               # Estudios de mejora
│   └── README.md                   # Índice de análisis
│
└── 📁 archive/                     # Documentos obsoletos ⭐ NUEVO
    ├── YYYY-MM/                    # Por fecha de archivado
    └── README.md                   # Política de retención
```

---

## 📝 Convenciones de Nombres

| Tipo | Prefijo | Ejemplo |
|------|---------|---------|
| Especificación | `SPEC_` | `SPEC_AUTH_OAUTH_FLOW.md` |
| Prompt | `PROMPT_` | `PROMPT_JULES_TASK_ASSIGNMENT.md` |
| Investigación | `RESEARCH_` | `RESEARCH_LLM_CONTEXT_LIMITS.md` |
| Sesión | `SESSION_` | `SESSION_2025-12-06_protocol-v3-1.md` |
| Reporte | `REPORT_` | `REPORT_MONTHLY_VELOCITY.md` |
| Análisis | `ANALYSIS_` | `ANALYSIS_AGENT_EFFICIENCY.md` |

**Formato de nombres:**
```
{PREFIX}_{TOPIC}_{OPTIONAL-DETAIL}.md
```

---

## 📊 YAML Frontmatter Obligatorio

Todo documento **DEBE** tener este frontmatter para indexación:

```yaml
---
title: "Título legible"
type: SPEC | PROMPT | RESEARCH | SESSION | REPORT | ANALYSIS
created: 2025-12-06
author: agent-name | human
project: git-core-protocol | software-factory | synapse
tags:
  - tag1
  - tag2
status: draft | active | archived
---
```

### Campos Adicionales por Tipo

**Para SESSIONS:**
```yaml
---
# ... campos base ...
session_id: "uuid-o-timestamp"
duration_minutes: 45
model: claude-sonnet-4 | gemini-pro | gpt-4
tokens_used: 12500
files_modified: 8
commits_made: 3
issues_touched:
  - "#42"
  - "#43"
next_actions:
  - "Implementar tests"
  - "Actualizar docs"
---
```

**Para RESEARCH:**
```yaml
---
# ... campos base ...
sources:
  - url: "https://example.com/article"
    consulted_at: 2025-12-06
confidence: high | medium | low
---
```

---

## 🔄 Workflow de Archivado

### 1. Sesiones Activas → Archivo

```
docs/prompts/SESSION_*.md  →  docs/agent-docs/sessions/YYYY-MM/
```

Las sesiones activas viven en `docs/prompts/` temporalmente.
Después de usarse, se mueven a `docs/agent-docs/sessions/` para análisis.

### 2. Documentos Obsoletos → Archive

```
docs/agent-docs/specs/SPEC_OLD.md  →  docs/agent-docs/archive/YYYY-MM/
```

Cuando un documento ya no es relevante, se archiva con fecha.

### 3. Migración de Archivos Prohibidos

Los archivos previamente prohibidos por el protocolo se migran así:

| Archivo Antiguo | Nuevo Destino |
|-----------------|---------------|
| `TASK.md` | → GitHub Issues (NO archivo) |
| `PLANNING.md` | → `.gitcore/ARCHITECTURE.md` (secciones) |
| `IMPLEMENTATION.md` | → `specs/SPEC_*.md` |
| `SUMMARY.md` | → `reports/REPORT_*.md` |
| `NOTES.md` | → `sessions/SESSION_*.md` |

---

## 📈 Métricas de Sesión (sessions/METRICS.json)

Cada mes se genera un archivo `METRICS.json`:

```json
{
  "month": "2025-12",
  "sessions_count": 24,
  "total_duration_minutes": 1080,
  "models_used": {
    "claude-sonnet-4": 15,
    "gemini-pro": 6,
    "gpt-4": 3
  },
  "tokens_total": 450000,
  "issues_resolved": 18,
  "files_modified_total": 142,
  "avg_session_duration_minutes": 45,
  "top_tags": ["auth", "refactor", "docs"],
  "efficiency_score": 0.85
}
```

---

## 🚀 Comandos Útiles

### Archivar sesiones antiguas
```powershell
./scripts/archive-sessions.ps1 -OlderThanDays 30
```

### Generar métricas mensuales
```powershell
./scripts/generate-session-metrics.ps1 -Month "2025-12"
```

### Buscar en documentos
```powershell
./scripts/search-agent-docs.ps1 -Query "OAuth" -Type "RESEARCH"
```

---

## 🔗 Referencias

- [SESSION_EXPORT.md](../SESSION_EXPORT.md) - Cómo exportar sesiones
- [AGENTS.md](../../AGENTS.md) - Configuración de agentes
- [copilot-instructions.md](../../.github/copilot-instructions.md) - Instrucciones del protocolo
