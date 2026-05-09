# Investigación de Embeddings para Xavier

> Documentación de referencia para la implementación de embeddings en Xavier.
> Última actualización: 2026-05-03

## Estado Actual del Código

Xavier tiene **tres caminos de embedding**, con redundancia y código heredado:

### 1. Embedder Port + Adapter (HexArch, pero incompleto)

| Archivo | Estado |
|---|---|
| `src/ports/outbound/embedding_port.rs` | ✅ Trait `EmbeddingPort` definido |
| `src/adapters/outbound/embedding/embedding_adapter.rs` | ❌ `todo!()` — sin implementación |
| `src/embedding/mod.rs` | ✅ `Embedder` trait + OpenAI + Noop |
| `src/embedding/openai.rs` | ✅ Implementación OpenAI vía API |

### 2. Embedder Nuevo (src/embedding/)

- Trait `Embedder` + `EmbedderConfig::from_env()` lee `XAVIER_EMBEDDER=openai`
- Único proveedor concreto: OpenAI (API key + endpoint configurable)
- `NoopEmbedder` por defecto (retorna vector vacío)
- **Solo llama a `build_embedder_from_env()` desde `qmd_memory.rs:2339`**

### 3. Embedder Legacy (src/memory/embedder.rs)

- `EmbeddingClient::from_env()` lee `XAVIER_EMBEDDING_URL` y `XAVIER_EMBEDDING_MODEL`
- Soporta: Ollama, OpenAI-compatible, Legacy (endpoint /embed)
- Detecta proveedor por URL: localhost → Ollama, /v1/ → OpenAI
- **Es el que realmente se usa en producción** (llamado en `generate_embedding`)

### Problemas identificados

1. **HexArch incompleto**: `embedding_adapter.rs` es `todo!()`, nunca se usa
2. **Redundancia**: Dos sistemas de embedding paralelos (el nuevo `src/embedding/` y el legacy `src/memory/embedder.rs`)
3. **Noop por defecto**: Si no hay `XAVIER_EMBEDDING_URL` ni `OPENAI_API_KEY`, las embeddings son `Vec::new()` — la búsqueda semántica no funciona
4. **Dimensiones hardcodeadas**: `OpenAIEmbedder::dimension()` usa match con valores fijos
5. **Cache funcional**: Hay cache de embeddings con TTL 1h en `qmd_memory.rs`, pero solo evita re-embedding de contenido idéntico

---

## Investigación de Modelos de Embedding

### Rankings MTEB (2026)

| Modelo | Score MTEB | Retrieval | Dims | Tokens Contexto | Licencia |
|---|---|---|---|---|---|
| **Gemini Embedding 001** | 68.32 | 67.71 | 768-3072 | 2,048 | Propietaria |
| **Qwen3-Embedding-8B** | 70.58 (multi) | Top | 32-7168 | 32,768 | Apache 2.0 |
| **OpenAI text-embedding-3-large** | 64.6 | 64.59 | 256-3072 | 8,192 | Propietaria |
| **Cohere Embed v4** | — | Fuerte (multi) | — | — | Propietaria |
| **BGE-M3** | — | 72% en benchmarks | 1,024 | 8,192 | Apache 2.0 |
| **mxbai-embed-large** | 64.68 | 59.25% | 1,024 | 512 | Apache 2.0 |
| **Nomic Embed v2** | 62.39 | 57.25% | 768 | 8,192 | Apache 2.0 |

### Análisis para el stack de Xavier

#### Opción 1: Solo Ollama Local (recomendado para ahora)

**Stack CPU-friendly:**

| Modelo | Params | Velocidad CPU | Calidad | Storage |
|---|---|---|---|---|
| **nomic-embed-text** | 137M | ✅ Rápido | ✅ Buena | 768 dims |
| **mxbai-embed-large** | 335M | ✅ Medio | ✅✅ Mejor | 1,024 dims |
| **bge-m3** | 568M | ⚠️ Lento | ✅✅✅ Excelente | 1,024 dims |

**Recomendación inicial: `nomic-embed-text`** (137M params)
- Corre en CPU sin GPU (EditorOne tiene CPU)
- MTEB 62.39, supera a OpenAI ada-002
- 8,192 tokens de contexto (ideal para memorias largas)
- Cache de embeddings de 1h mitiga latencia
- Ya es el default en `embedder.rs`

**Para producción (cuando GPU disponible): `bge-m3`**
- 72% retrieval accuracy (vs 57% nomic)
- Sparse + dense + multi-vector en un solo modelo
- NLP multilingüe real (100+ idiomas)
- ONNX optimization disponible para CPU acelerada

#### Opción 2: API Remota (cuando no hay GPU)

| Proveedor | Modelo | Costo/1M tokens | Calidad | Recomendado para |
|---|---|---|---|---|
| OpenAI | text-embedding-3-small | $0.02 | Buena | Costo mínimo |
| OpenAI | text-embedding-3-large | $0.13 | Excelente | Calidad máxima |
| Voyage AI | voyage-3-large | $0.18 | Excelente (técnico) | Código/documentación |
| Gemini | embedding-001 | $0.15 | Superior | Multilingüe + calidad |

#### Opción 3: Modelo Futuro — Qwen3-Embedding

- Líder MTEB multilingüe (70.58)
- Apache 2.0 — open source
- 32K tokens de contexto + Matryoshka dims flexibles
- **Requiere GPU** (8B params) — no es viable en CPU ahora
- Ideal para cuando el stack migre a GPU

---

## Recomendación Final

### Fase 1 (Inmediata — Ya)
```
Proveedor: Ollama local (localhost:11434)
Modelo:    nomic-embed-text (137M params)
Dims:      768
Costo:     $0 (local)
Latencia:  ~50-150ms en CPU moderna
```

### Fase 2 (Corto plazo)
```
Migrar a bge-m3 via Ollama para mejor calidad de retrieval
Implementar el adapter HexArch real (src/adapters/outbound/embedding/)
Uniformizar: un solo Embedder trait, eliminar el legacy
```

### Fase 3 (Mediano plazo — con GPU)
```
Qwen3-Embedding-8B o Gemini Embedding API
Matryoshka dimensions para balance velocidad/precisión
Embedding adapters entrenables (fine-tuning sobre datos de Xavier)
```

### Fase 4 (Largo plazo)
```
Embedding adapters tipo Chroma (MLP entrenable)
Cross-encoder reranker para retrieval híbrido
Sparse + dense fusion (BM25 + Dense + Multi-vector)
```

---

## Issues Creados

Crear estos issues para trackear el trabajo:

1. **#170** — [embed] Migrar embeddings de legacy a HexArch port
   - Implementar `embedding_adapter.rs` (hoy es `todo!()`)
   - Conectar `EmbeddingPort` desde `qmd_memory.rs` en vez de llamadas directas
   - Deprecar `src/memory/embedder.rs`

2. **#171** — [embed] Configurar Ollama + nomic-embed-text como proveedor local
   - Verificar que Ollama corre en localhost:11434
   - Probar `xavier search` con embeddings reales (hoy devuelve vector vacío si no hay API key)
   - Setear defaults: `XAVIER_EMBEDDING_URL=http://localhost:11434`, `XAVIER_EMBEDDING_MODEL=nomic-embed-text`

3. **#172** — [embed] Evaluación bge-m3 como upgrade de calidad
   - Benchmarks retrieval vs nomic-embed-text
   - Medir latencia en CPU (EditorOne)
   - Verificar compatibilidad con sqlite-vec (1,024 dims)

4. **#173** — [embed] Cache de embeddings multi-capa
   - Cache actual (1h TTL por hash de contenido)
   - Agregar persistencia en disco (sqlite-vec guarda embeddings, evitar re-embedding)
   - Batch embedding para operaciones masivas

---

## Config Recomendada (.env)

```env
# EMBEDDINGS - Local Ollama (recomendado para CPU)
XAVIER_EMBEDDER=openai
XAVIER_EMBEDDING_URL=http://localhost:11434
XAVIER_EMBEDDING_MODEL=nomic-embed-text

# Alternativa API (cuando no hay Ollama)
# XAVIER_EMBEDDER=openai
# XAVIER_EMBEDDING_ENDPOINT=https://api.openai.com/v1/embeddings
# XAVIER_EMBEDDING_MODEL=text-embedding-3-small
# OPENAI_API_KEY=sk-...
```

---

## Referencias

- [MTEB Leaderboard](https://huggingface.co/spaces/mteb/leaderboard)
- [Ollama Embedding Models](https://ollama.com/blog/embedding-models)
- [BGE-M3 Paper (2024)](https://arxiv.org/abs/2402.03216)
- [Nomic Embed v2 (2025)](https://www.nomic.ai/blog/posts/nomic-embed-v2)
- [Qwen3-Embedding (2025)](https://huggingface.co/Qwen/Qwen3-Embedding-0.6B)
- [sqlite-vec Rust docs](https://alexgarcia.xyz/sqlite-vec/rust.html)
