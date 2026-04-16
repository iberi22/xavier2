# Xavier2 Docker Deployment - SKILL.md

## Priority: HIGH 🔴

Xavier2 en Docker es la base para todo el sistema de memoria. Sin esto, Synapse no tiene donde almacenar su entrenamiento continuo.

---

## Plan Completo

### Fase 1: Docker Container (PRIORIDAD AHORA)
- [ ] Build Linux binary de Xavier2
- [ ] Crear Dockerfile optimizado
- [ ] docker-compose.yml con healthcheck
- [ ] Push a Docker Hub (iberi22/xavier2)

### Fase 2: Implementar Multi-Layer Memory (Part 3 que falló)
- [ ] src/retrieval/gating.rs - Adaptive retrieval gating
- [ ] src/consistency/regularization.rs - Retention regularizer
- [ ] src/consistency/mod.rs - Consistency module
- [ ] src/retrieval/mod.rs - Retrieval module exports
- [ ] Endpoint /memory/retrieve con multi-layer search

### Fase 3: Fine-tuning Synapse (después de Docker)
- [ ] Dataset en HuggingFace ✅ (ya existe)
- [ ] Notebook configurado
- [ ] Fine-tune Gemma 4 E2B
- [ ] Test y feedback
- [ ] Deploy a Ollama local

---

## Comandos Rápidos

### Build Docker
```bash
cd E:\scripts-python\xavier2
bash scripts/build-docker.sh
```

### Run Docker
```bash
cd E:\scripts-python\xavier2\docker
docker-compose up -d
docker logs -f xavier2-memory
```

### Verify
```bash
curl http://localhost:8006/health
```

---

## Docker Configuration

### Dockerfile optimizado (17MB base)
```dockerfile
FROM rust:1.77-slim
# Minimal deps, static binary
```

### docker-compose
- Xavier2 en puerto 8006
- Redis opcional para vector storage
- Healthcheck configurado
- Volumes persistentes

---

## Status

| Component | Status |
|-----------|--------|
| Dockerfile | ✅ Creado |
| docker-compose | ✅ Creado |
| build-docker.sh | ✅ Creado |
| Binary build | ⏳ Pendiente |
| Push to Hub | ⏳ Pendiente |
| Multi-Layer Memory Part 3 | ⏳ Pendiente |

---

*Last updated: 2026-04-16*
*Priority: Xavier2 Docker > Multi-Layer Memory > Synapse Fine-tune*