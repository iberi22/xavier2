# SWAL Benchmark + New Agent: "Gentleman Dev"

## Plan Completo

### 1. XAVIER2 DOCKER (Prioridad Alta)
- [x] Multi-stage Dockerfile creado
- [ ] Build corriendo (session: fast-lobster)
- [ ] docker-compose.yml
- [ ] Push a Docker Hub
- [ ] Test local

### 2. SYNAPSE BENCHMARK
- [ ] Dataset listo en HuggingFace ✅
- [ ] Notebook Colab configurado
- [ ] Fine-tune Gemma 4 E2B
- [ ] Comparativa vs GPT-4o-mini
- [ ] Métricas: LOCOMO, coherence, latency

### 3. NEW AGENT: "Gentleman Dev" (Telegram)
- **Stack:** gentle-ai + engram + Telegram bot
- **Propósito:** Competir con Cortex y Xavier2
- **Workflow:** SDD (Spec-Driven Development)

### Arquitectura:
```
┌─────────────────────────────────────────────────────────┐
│                    SWAL Agent Ecosystem                  │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │   Xavier2   │  │   Cortex    │  │   Engram    │     │
│  │  (Memory)   │  │  (Memory)   │  │  (Memory)   │     │
│  │             │  │             │  │             │     │
│  │ Rust-based  │  │ Python/API  │  │ Gentle-ai   │     │
│  │ Multi-layer │  │ SWAL prod   │  │ Cross-agent │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
│          │               │               │               │
│          └───────────────┴───────────────┘               │
│                          │                               │
│                          ▼                               │
│              ┌─────────────────────┐                     │
│              │  SWAL Benchmark     │                     │
│              │  - LOCOMO Score     │                     │
│              │  - Coherence        │                     │
│              │  - Latency          │                     │
│              │  - Accuracy         │                     │
│              └─────────────────────┘                     │
│                          │                               │
│                          ▼                               │
│              ┌─────────────────────┐                     │
│              │   Telegram Bot      │                     │
│              │   "Gentleman Dev"   │                     │
│              │                     │                     │
│              │ -gentle-ai stack   │                     │
│              │ -engram memory     │                     │
│              │ -SDD workflow      │                     │
│              │ -OpenCode/Codex    │                     │
│              └─────────────────────┘                     │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

---

## NEW AGENT: Gentleman Dev

### Características:
- **Nombre:** SWAL-Gentleman o "Gentleman Dev"
- **Canal:** Telegram Bot
- **Stack:**
  - gentle-ai ecosystem
  - engram (persistent memory)
  - SDD workflow (Spec-Driven Development)
  - Telegram integration
  - Codex/Claude Code como backend

### Engram vs Cortex vs Xavier2:

| Feature | Engram | Cortex | Xavier2 |
|---------|--------|--------|---------|
| **Type** | Memory | Memory | Memory |
| **API** | CLI/MCP | REST | REST |
| **Language** | Go | Python | Rust |
| **Sync** | Machine-to-machine | Centralized | Centralized |
| **SDD** | Native | No | No |
| **Use case** | Dev workflow | SWAL prod | Research |

### Ventajas de Engram:
1. **Go-based** - extremadamente rápido
2. **MCP native** - integrable con OpenClaw
3. **Machine sync** - compite con Cortex
4. **SDD workflow** - desarrollo guiado por specs
5. **Open source** - gentle-ai ecosystem

---

## Pasos de Implementación:

### Phase 1: Docker Build (corriendo ahora)
```bash
# Status: Building...
docker build --platform linux/amd64 -t iberi22/xavier2:latest
```

### Phase 2: Install gentle-ai + engram
```bash
# En la máquina de producción
scoop bucket add gentleman https://github.com/Gentleman-Programming/scoop-bucket
scoop install gentle-ai

# Verificar engram
engram --version
```

### Phase 3: Telegram Bot Setup
```bash
# Crear bot via BotFather
# Obtener token

# Configurar gentle-ai con Telegram
gentle-ai config set telegram.bot_token TU_TOKEN
gentle-ai config set telegram.enabled true
```

### Phase 4: Benchmark Framework
```bash
# Scripts de benchmark en:
# E:\scripts-python\xavier2\scripts\benchmark_*.py
```

---

## Benchmark Plan

### Métricas a Medir:

| Métrica | Herramienta | Target |
|---------|-------------|--------|
| LOCOMO Score | Custom eval | >95% |
| Coherence | LLM evaluation | >0.9 |
| Latency | Time per query | <500ms |
| Memory recall | Hit rate | >85% |
| Cross-agent sync | Engram vs Cortex | Parity |

### Datasets:
- SWAL Synapse Dataset (HuggingFace)
- LOCOMO benchmark
- Custom SWAL scenarios

---

## Status:

| Componente | Status | Owner |
|------------|--------|-------|
| Xavier2 Docker | 🔄 Building | Auto |
| Synapse Dataset | ✅ Done | Auto |
| Colab Notebook | ✅ Done | Auto |
| Benchmark Framework | ⏳ Pending | - |
| gentle-ai | ⏳ Install | Manual |
| engram | ⏳ Install | Manual |
| Telegram Bot | ⏳ Pending | - |

---

## Próximo Paso:

Cuando termine el build de Docker, subir a Docker Hub:
```bash
docker push iberi22/xavier2:latest
```

*Last updated: 2026-04-16*