# Jules Workflow — Mayo 2026

## Contexto

Jules está asignado a múltiples issues en `iberi22/xavier`. Este documento explica cómoinvocarlo efectivamente con los hallazgos del sprint.

## Invocación Directa

### Opción 1: Por issue (rápido)
```
/jules fix #78
```

### Opción 2: Múltiples issues en paralelo
```
/jules fix #78 #75 #74
```

### Opción 3: Phase completa
```
/jules implement from docs/JULES_PROMPTS_MAY2026.md phase 1
```

## Fases Sugeridas

### Phase 1: SEVIER Core (crítico)
- #78 Docker env vars
- #75 unregister endpoint
- #74 wrong payload

### Phase 2: Architecture Cleanup
- #76 graceful shutdown
- #84 duplicate handlers
- #93 TimeMetrics OnceLock

### Phase 3: Performance
- #137 qmd_memory modularization

## Verificación Post-Jules

Después de que Jules termine un fix:
```bash
git fetch origin main
git diff HEAD --stat
gh pr merge <N> --squash
```

## Si Jules tiene problemas de selección de tareas

Ver document in `docs/JULES_PROMPTS_MAY2026.md` — cada prompt es auto-contenido.

## Changelog

Todo lo resuelto está en `CHANGELOG-MAY2026.md` — Jules puede consultarlo para contexto.
