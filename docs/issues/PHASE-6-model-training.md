# PHASE 6: Fine-Tune Local Model with Xavier Memory Data

**Status:** 🔴 PROPOSED
**Created:** 2026-04-16
**Labels:** training, fine-tuning, ML

---

## Vision

Train a small local LLM (3-8B parameters) using all curated memory data from Xavier to create a **domain-specific AI assistant** that:
- Understands SWAL context perfectly
- Has better retrieval accuracy than generic models
- Runs 100% offline on local GPU
- Improves over time as more data is curated

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    PHASE 6 PIPELINE                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐     │
│  │  Xavier     │───▶│  Data        │───▶│  Fine-tune    │     │
│  │  Memory      │    │  Curation    │    │  (QLoRA)     │     │
│  │  (Raw)       │    │  Pipeline    │    │  Small LLM   │     │
│  └──────────────┘    └──────────────┘    └──────────────┘     │
│         │                                        │             │
│         │           ┌──────────────┐             │             │
│         └─────────▶│  Training     │◀────────────┘             │
│                     │  Dataset      │                           │
│                     │  (Curated)    │                           │
│                     └──────────────┘                           │
│                            │                                    │
│                            ▼                                    │
│                     ┌──────────────┐                           │
│                     │  Fine-tuned  │                           │
│                     │  Model       │                           │
│                     │  (Phi-3.5)   │                           │
│                     └──────────────┘                           │
│                            │                                    │
│                            ▼                                    │
│                     ┌──────────────┐                           │
│                     │  Local AI    │                           │
│                     │  Assistant   │                           │
│                     │  (Sinapsis)  │                           │
│                     └──────────────┘                           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Data Sources from Xavier

### 1. Structured Memory Categories

| Category | Path Pattern | Data Type |
|----------|--------------|-----------|
| Business Context | `swal/*` | Company info, products, strategy |
| Clients | `client/*` | Contact info, interests, projects |
| Projects | `project/*`, `product/*` | Specs, pricing, status |
| Sales | `sales/*` | RFI templates, proposals, process |
| Technical | `system/*`, `skills/*` | Architecture, configs |
| Agents | `agent/*` | Agent configs, capabilities |
| Benchmark | `benchmark/*` | Performance metrics |

### 2. Memory Kinds (Typed)

| Kind | Count (Est.) | Use Case |
|------|-------------|----------|
| `document` | ~100 | General knowledge |
| `conversation_summary` | ~50 | Decisions, conclusions |
| `fact_atom` | ~200 | Atomic facts (pricing, names) |
| `entity_state` | ~50 | Entity snapshots |
| `reflection` | ~20 | Synthesized insights |

---

## Data Curation Pipeline

### Step 1: Export from Xavier

```python
# Export all memories to JSONL
def export_memories():
    memories = xavier.query_all(limit=10000)
    for m in memories:
        yield {
            "content": m.content,
            "path": m.path,
            "kind": m.kind,
            "metadata": m.metadata,
            "created_at": m.created_at
        }
```

### Step 2: Format as Training Data

#### Format: Instruction-Response Pairs

```json
{
  "instruction": "Quién es BELA y qué proyectos tiene SWAL?",
  "response": "BELA es el fundador y desarrollador principal de SouthWest AI Labs (SWAL). Tiene varios proyectos incluyendo: Xavier (sistema de memoria para agentes AI), ManteniApp (SaaS de monitoreo de maquinaria), Gestalt-Rust (sistema de ejecución de agentes), y Cortex (memoria enterprise).",
  "category": "business",
  "source": "swal/business/overview"
}
```

#### Format: Q&A from Facts

```json
{
  "question": "Cuál es el precio de ManteniApp Starter?",
  "answer": "ManteniApp Starter cuesta $499 USD por mes.",
  "fact_path": "product/manteniapp/pricing",
  "confidence": 0.95
}
```

#### Format: Reasoning Chains

```json
{
  "context": ["Leonardo Duque es vendedor en Rodacenter Chile", "Rodacenter interesa en ManteniApp"],
  "question": "Por qué Leonardo Duque podría recomendar ManteniApp?",
  "reasoning": "1) Leonardo trabaja en Rodacenter Chile. 2) Rodacenter está interesado en ManteniApp. 3) ManteniApp monitorea maquinaria industrial. 4) Chile tiene industria minera en Antofagasta.",
  "answer": "Porque Leonardo ve una oportunidad de negocio en monitoreo de maquinaria para la industria minera chilena."
}
```

### Step 3: Quality Filtering

| Filter | Criteria | Action |
|--------|----------|--------|
| Relevance | Score < 0.5 | Exclude |
| Hallucination | Fact not verified | Flag for review |
| Duplicates | Similarity > 0.9 | Deduplicate |
| Age | > 6 months | Re-verify |
| Source | Low confidence | Exclude |

---

## Training Configuration

### Model Selection

| Model | Parameters | VRAM | Training Time | Quality |
|-------|-----------|------|--------------|---------|
| **Phi-3.5 Mini** | 3.8B | 6-8GB | ~2-4 hours | ⭐⭐⭐⭐ |
| **Llama 3.2 3B** | 3B | 6GB | ~2 hours | ⭐⭐⭐⭐ |
| **Qwen2.5 3B** | 3B | 6GB | ~2 hours | ⭐⭐⭐⭐ |
| **Mistral 7B Q4** | 7B | 12GB | ~6 hours | ⭐⭐⭐⭐⭐ |

**Recommendation:** Start with **Phi-3.5 Mini** or **Llama 3.2 3B** for fastest iteration.

### Training Method: QLoRA

```python
# QLoRA Configuration
config = {
    "model": "microsoft/Phi-3.5-mini-instruct",
    "lora_r": 64,
    "lora_alpha": 16,
    "lora_dropout": 0.1,
    "quantization": "4-bit",
    "target_modules": ["q_proj", "v_proj", "k_proj", "o_proj"],
    "training_steps": 1000,
    "batch_size": 4,
    "learning_rate": 2e-4,
    "warmup_steps": 100
}
```

### Dataset Size Target

| Phase | Records | Description |
|-------|---------|-------------|
| v0.1 | 500 | Core SWAL knowledge |
| v0.5 | 2,000 | Expanded with client data |
| v1.0 | 5,000 | Full curated corpus |

---

## Training Dataset Format

### JSONL Structure

```jsonl
{"instruction": "...", "response": "...", "category": "..."}
{"instruction": "...", "response": "...", "category": "..."}
...
```

### Categories for Training

| Category | Weight | Description |
|----------|--------|-------------|
| `business` | 20% | Company, products, strategy |
| `client` | 15% | Client info, interests |
| `technical` | 25% | Architecture, code, configs |
| `sales` | 15% | RFI, proposals, process |
| `reasoning` | 15% | Chain-of-thought, analysis |
| `general` | 10% | General knowledge |

---

## Implementation Phases

### Phase 6.1: Data Export & Curation
- [ ] Create export script from Xavier API
- [ ] Build deduplication pipeline
- [ ] Create JSONL formatter
- [ ] Manual review of sample data

### Phase 6.2: Training Setup
- [ ] Install Axolotl or LLaMA-Factory
- [ ] Configure QLoRA for small model
- [ ] Create training config YAML
- [ ] Test training on 100 samples

### Phase 6.3: Fine-tuning
- [ ] Train v0.1 on core data (~500 records)
- [ ] Evaluate on LOCOMO benchmark
- [ ] Iterate on quality
- [ ] Train v1.0 on full curated data

### Phase 6.4: Integration
- [ ] Export model to Ollama format
- [ ] Create inference API
- [ ] Integrate with Xavier retrieval
- [ ] A/B test vs base model

---

## Expected Improvements

| Metric | Base Model | Fine-tuned |
|--------|------------|------------|
| Factuality | 60% | **85%+** |
| SWAL Context | 40% | **90%+** |
| Retrieval Accuracy | 75% | **90%+** |
| Hallucination Rate | 15% | **<5%** |
| Latency | ~500ms | **~200ms** |

---

## Tools

| Tool | Purpose |
|------|---------|
| **Axolotl** | Training (QLoRA, LoRA) |
| **LLaMA-Factory** | Alternative training UI |
| **Ollama** | Model serving |
| **MLX** | Apple Silicon training (future) |
| **vLLM** | Inference optimization |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Overfitting | Use validation set, early stopping |
| Data leakage | Hold out 20% for testing |
| Catastrophic forgetting | Mix with base model data |
| Training time | Use gradient checkpointing |
| Model size | Start small, scale up |

---

## Success Criteria

- [ ] 500+ curated training examples
- [ ] Fine-tuned model passes LOCOMO at >70%
- [ ] Model runs locally on RX 6600
- [ ] Benchmark shows improvement over base model
- [ ] Integrated into Xavier inference pipeline

---

## Commands

```bash
# Export memories
python scripts/export_xavier_memories.py --output data/training.jsonl

# Deduplicate
python scripts/curate_training_data.py --input data/training.jsonl --output data/curated.jsonl

# Train with Axolotl
axolotl train configs/phi35-swal.yaml

# Export to Ollama
ollama create swal-assistant -f Modelfile

# Test
ollama run swal-assistant "Quién es BELA?"
```

---

## References

- Axolotl: https://github.com/OpenAccess-AI-Collective/axolotl
- LLaMA-Factory: https://github.com/hiyouga/LLaMA-Factory
- QLoRA Paper: https://arxiv.org/abs/2305.14314
- OpenOrca Dataset: https://huggingface.co/datasets/Open-Orca/OpenOrca
