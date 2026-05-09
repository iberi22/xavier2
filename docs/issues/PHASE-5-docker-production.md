# PHASE 5: Docker Production Ready + Feature Parity

**Status:** 🔴 IN PROGRESS
**Created:** 2026-04-16
**Labels:** docker, production, enhancement

---

## Context

Xavier es el sistema de memoria open source de SWAL. Necesita estar listo para:
1. **Despliegue Docker** - contenedor production-ready
2. **Paridad de features** con Cortex (enterprise)
3. **Mejoras de retrieval** - reducir alucinaciones, mejorar relevancia

### Benchmark Actual (vs Cortex)

| Metric | Xavier | Cortex | Target |
|--------|---------|--------|--------|
| Latency Avg | 356ms | 969ms | <200ms |
| Latency P95 | 404ms | 2418ms | <400ms |
| Relevancia | 75% | 75% | >85% |
| Sin Alucinaciones | ✅ | ✅ | ✅ |

### SWAL Memory Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    SWAL MEMORY LAYER                        │
├─────────────────────────────────────────────────────────────┤
│  Gestalt-Rust (Agent Executor)                              │
│  └── SurrealDB: Gestiona agentes que orchgesta             │
│                                                             │
│  Synapse (Internet Evolve)                                  │
│  └── Su propio sistema de memoria                          │
│                                                             │
│  Xavier (CORE - Memory Principal)                          │
│  └── SQLite-vec + RRF fusion                               │
│  └── 100% local, MIT license                                │
│                                                             │
│  Cortex (Legacy - Enterprise)                              │
│  └── Docker-based, usa file backend                        │
│  └── Mantener para backwards compat                         │
└─────────────────────────────────────────────────────────────┘
```

---

## Objetivo 1: Docker Production Ready ✅

### Tasks

- [ ] **Multi-stage Dockerfile optimizado**
  - Usar `rust:1.89-slim` en vez de `bookworm`
  - Tamaño objetivo < 100MB
  - Multi-platform (amd64, arm64)

- [ ] **docker-compose.yml production-ready**
  - Health checks robustos
  - Resource limits
  - Log rotation
  - Restart policies

- [ ] **.env.example actualizado**
  - Variables documentadas
  - Valores por defecto seguros

- [ ] **CI/CD básico**
  - GitHub Actions para build Docker
  - Push to ghcr.io

- [ ] **Test de smoke post-deploy**
  - Verificar que el contenedor levanta
  - Verificar endpoints básicos

---

## Objetivo 2: Feature Parity con Cortex

### Enterprise Features de Cortex

| Feature | Cortex | Xavier | Status |
|---------|--------|---------|--------|
| Memory Decay (Ebbinghaus) | ✅ | ⚠️ Partial | Implementar |
| Memory Consolidation | ✅ | ✅ | Hecho |
| Belief Graph | ✅ | ❌ | Implementar |
| Quality Scoring | ✅ | ⚠️ Partial | Mejorar |
| Auto-Archive | ✅ | ❌ | Implementar |
| RRF Fusion | ✅ | ✅ | Hecho |
| Multi-layer Memory | ✅ | ✅ | Hecho |

### Tasks - Memory Decay

- [ ] Implementar decay basado en Ebbinghaus
  - Priorities: Critical (0%), High (2%/dia), Medium (5%), Low (15%), Ephemeral (50%)
  - Aplicar decay durante consolidate

### Tasks - Belief Graph

- [ ] Crear `src/memory/belief_graph.rs`
- [ ] Verificar facts contra knowledge graph
- [ ] Confidence scoring basado en verificaciones

### Tasks - Quality Scoring

- [ ] Mejorar composite score
  - 40% relevance
  - 25% accuracy
  - 20% freshness
  - 15% completeness

### Tasks - Auto-Archive

- [ ] Archivar memorias antiguas automáticamente
- [ ] Mover a tabla `archives` en SQLite

---

## Objetivo 3: Mejora de Retrieval

### Problemas Identificados

1. **75% relevancia** - necesita mejorar a >85%
2. **Query "ManteniApp pricing"** no retorna $499
3. **Irrelevante en algunos casos** - retorna dato conocido en vez de "no encontrado"

### Tasks

- [ ] **Mejorar embedding quality**
  - Probar nomic-embed-text vs pplx-embed
  - Fine-tuning de similarity threshold

- [ ] **Agregar re-ranking mejorado**
  - BM25 para keyword matching
  - Combinar con vector search (RRF)

- [ ] **Relevance filtering**
  - Threshold configurable
  - Retornar "no encontrado" si confidence < threshold

- [ ] **Benchmark suite**
  - LOCOMO-style queries
  - BEAM-style queries
  - Registrar resultados

---

## Referencias

- Dockerfile actual: `Dockerfile.simple`
- docker-compose: `docker-compose.yml`
- Benchmark results: `E:\scripts-python\xavier-benchmark\`

---

## Commands Útiles

```bash
# Build Docker
docker build -t xavier:0.4.1 .

# Run
docker compose up -d

# Logs
docker compose logs -f xavier

# Benchmark
powershell -File scripts/benchmark_runner.py
```

---

## Checklist Final

- [ ] Docker image < 100MB
- [ ] Healthcheck pasa
- [ ] Latency avg < 200ms
- [ ] Relevancia > 85%
- [ ] No alucinaciones en queries fuera de contexto
- [ ] CI/CD configurado
- [ ] Docs actualizados
