---
title: "feat(evolve): Autonomous Self-Evolving Memory Module (AutoResearch + LoCoMo + Evo-Memory + A-Mem)"
labels:
  - enhancement
  - ai-agent
  - ai-plan
  - high-stakes
assignees: []
---

## 🧬 Overview

Xavier2 needs an **Evolve Module** — a self-contained, autonomous agentic subsystem that continuously improves the entire Xavier2 memory architecture by running automated research experiments, synthesizing SOTA findings, and applying verified improvements. This issue tracks the full design, implementation, and integration plan.

The Evolve Module draws inspiration from three convergent paradigms:
1. **Karpathy's autoresearch** — An autonomous experiment loop where an AI agent modifies code, runs experiments with fixed budgets, measures against a single metric, and keeps/discards changes indefinitely.
2. **LoCoMo Benchmark** (arXiv:2402.17753) — Evaluating very long-term conversational memory through multi-hop QA, temporal reasoning, and event summarization over 300-turn / 9K-token dialogues.
3. **Evo-Memory Benchmark** (arXiv, Nov 2025) — Streaming benchmark evaluating 10+ self-evolving memory modules across multi-turn and single-turn tasks. ReMem consistently outperforms baselines.
4. **A-Mem** (NeurIPS 2025) — Zettelkasten-inspired agentic memory: atomic notes, bidirectional linking, autonomous memory evolution triggered by new knowledge integration.
5. **SEPGA** — Self-Evolving, Policy-Governed Agentic Automation: multi-stage feedback loops (plan → execute → evaluate → reflect) with governance modules.

---

## 🎯 Goals

- [ ] **Top-1 on LoCoMo benchmark** — Beat all existing baselines on multi-hop QA (Single/Multi/Temporal/OpenDomain/Adversarial categories)
- [ ] **Top-1 on Evo-Memory benchmark** — Outperform ReMem and all 10+ memory modules
- [ ] **Autonomous self-improvement** — The Evolve agent runs independently, testing hypotheses against Xavier2's own memory subsystem
- [ ] **Xavier2-as-MCP integration** — Use Xavier2's own memory (running in Docker) as the MCP backend for recording evolution state, experiment logs, and knowledge synthesis

---

## 🏗️ Architecture

### Evolve Agent Flow (Inspired by autoresearch + SEPGA)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                    XAVIER2 EVOLVE MODULE (Autonomous Loop)                    │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  🔬 RESEARCHER  ──▶  🧪 EXPERIMENTER  ──▶  📊 EVALUATOR  ──▶  🧠 REFLECTOR │
│       ▲                                                           │          │
│       │                                                           ▼          │
│       └──────────────────  📦 INTEGRATOR  ◀──────────────────────┘          │
│                            (Apply / Discard)                                 │
│                                                                              │
│  State persisted in: SurrealDB via Xavier2 MCP (localhost:8003)              │
│  Metrics tracked:    LoCoMo scores, Evo-Memory scores, latency, tokens      │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Sub-Agents

| Agent | Role | Inspired By |
|-------|------|-------------|
| **🔬 Researcher** | Scans arXiv, papers, codebases for new memory techniques | autoresearch `program.md` read phase |
| **🧪 Experimenter** | Modifies Xavier2 code (memory module) to implement hypotheses | autoresearch `train.py` modification |
| **📊 Evaluator** | Runs benchmarks (LoCoMo, Evo-Memory) with fixed time/token budgets | autoresearch `val_bpb` measurement |
| **🧠 Reflector** | Analyzes results, generates hypotheses for next cycle | SEPGA reflection loop, A-Mem evolution |
| **📦 Integrator** | Applies winning changes to main branch, discards losers | autoresearch keep/discard pattern |

---

## 📚 Research Synthesis

### From Karpathy's autoresearch (cloned & analyzed)

**Key patterns to adopt:**
1. **Fixed-budget experimentation** — Each experiment runs for a fixed time/token budget, making results comparable
2. **Single metric optimization** — One ground-truth metric (for us: LoCoMo multi-hop QA F1 score)
3. **Branch-per-experiment** — Each hypothesis gets its own branch, keeping main clean
4. **Never stop loop** — The agent runs indefinitely until manually interrupted
5. **Results logging** — TSV/structured logs for every experiment with commit hash, metric, status, description
6. **Simplicity criterion** — Simpler wins. A 0.001 improvement that adds 20 lines of hacky code? Discard. A 0.001 improvement from *deleting* code? Keep.
7. **Crash recovery** — If an experiment crashes, diagnose quickly. If trivial fix → retry. If fundamentally broken → skip and log.

### From LoCoMo Paper (arXiv:2402.17753)

**Benchmark tasks we must target:**
1. **Single-hop QA** — Direct factual recall from conversation history
2. **Multi-hop QA** — Require chaining multiple facts across sessions
3. **Temporal QA** — Questions about when events occurred and temporal ordering
4. **Open-domain QA** — General reasoning about conversation subjects
5. **Adversarial QA** — Questions designed to trick the model with false premises
6. **Event Summarization** — Summarize key life events across 35 sessions
7. **Multi-modal Dialogue Generation** — Generate contextually appropriate responses with images

**Key finding:** RAG with observations (entity assertions extracted during conversation) + session summaries significantly outperforms base LLMs and even long-context models on all tasks.

### From Evo-Memory Benchmark (Nov 2025)

- 10+ memory modules evaluated across sequential task streams
- **ReMem** consistently outperforms baselines across model families
- Key insight: Self-evolving memory (search → adapt → evolve after each interaction) dramatically enhances agent capabilities
- Bridges conversational recall and experience reuse

### From A-Mem (NeurIPS 2025)

**Xavier2 implementation targets:**
1. **Atomic memory notes** — Each memory is a structured note with context, keywords, tags, embeddings
2. **Bidirectional linking** — New memories automatically link to related historical memories
3. **Memory evolution triggers** — When new memory contradicts or enriches existing memory, trigger autonomous update of historical memory's context/attributes
4. **Continuous refinement** — Memory network self-organizes over time without predetermined operations

### From SEPGA Research

- Multi-stage feedback loops: Plan → Execute → Evaluate → Reflect
- Policy governance module prevents runaway self-modification
- Confidence thresholds for autonomous vs. human-escalated decisions

---

## 🛠️ Implementation Plan

### Phase 1: Xavier2 MCP Integration (Skill + Configuration)

- [ ] Create `.agents/skills/xavier2-memory/SKILL.md` — Skill document for using Xavier2 memory as MCP
- [ ] Configure Xavier2 Docker MCP endpoint (`localhost:8003`) for agent state persistence
- [ ] Define MCP tools: `evolve_store_experiment`, `evolve_get_history`, `evolve_store_hypothesis`, `evolve_get_metrics`
- [ ] Verify Xavier2 health endpoint responds at `http://localhost:8003/health`

### Phase 2: Evolve Module Core (`src/agents/evolve/`)

- [ ] `mod.rs` — Module coordinator, manages the 5 sub-agent loop
- [ ] `researcher.rs` — Scans sources (arXiv API, GitHub trending, configured paper lists) for new memory techniques
- [ ] `experimenter.rs` — Generates code modifications to `src/memory/` based on researcher findings
- [ ] `evaluator.rs` — Runs LoCoMo and Evo-Memory benchmarks against current Xavier2 instance
- [ ] `reflector.rs` — Analyzes experiment results, generates improvement hypotheses using A-Mem evolution patterns
- [ ] `integrator.rs` — Applies or discards changes based on metric improvements (autoresearch keep/discard)
- [ ] `config.rs` — Evolve module configuration (intervals, thresholds, benchmark targets)

### Phase 3: Upgrade Existing `self_improve.rs`

The current `self_improve.rs` is a basic metrics tracker (success rate, latency). It needs to be elevated to support:
- [ ] Integration with the Evolve loop as the metrics collection backend
- [ ] A-Mem style structured improvement notes with bidirectional linking
- [ ] Historical experiment tracking in SurrealDB via Xavier2 MCP
- [ ] Confidence-weighted improvement application (SEPGA pattern)

### Phase 4: Benchmark Integration

- [ ] Implement LoCoMo benchmark runner (download dataset, run QA/summarization/dialogue tasks)
- [ ] Implement Evo-Memory benchmark runner (streaming task evaluation)
- [ ] Create `benches/evolve_benchmarks.rs` with standardized metric collection
- [ ] Establish baseline scores on both benchmarks with current Xavier2 implementation

### Phase 5: Memory Architecture Enhancements (Driven by Evolve)

The Evolve module should autonomously discover and implement improvements to:
- [ ] `belief_graph.rs` — Complete temporal GraphRAG with entity-relationship-time triples
- [ ] `virtual_memory.rs` — Full MemGPT-style context paging with page_in/page_out tools
- [ ] `qmd_memory.rs` — A-Mem style atomic notes with bidirectional linking and evolution triggers
- [ ] New: `episodic_memory.rs` — Event sequences across sessions (LoCoMo-inspired)
- [ ] New: `procedural_memory.rs` — Tool mastery and instruction memory

### Phase 6: Autonomous Execution

- [ ] Deploy Evolve agent as background Tokio task within Xavier2 runtime
- [ ] Configure experiment budget (e.g., 10 minutes per hypothesis evaluation)
- [ ] Set up results logging to SurrealDB with structured schema
- [ ] Implement Guardian-style governance: high-stakes changes require human approval
- [ ] Create monitoring dashboard (Grafana) for Evolve module metrics

---

## 📊 Success Metrics

| Metric | Current Baseline | Target |
|--------|-----------------|--------|
| LoCoMo Single-hop QA (F1) | Not measured | > 87% |
| LoCoMo Multi-hop QA (F1) | Not measured | > 70% |
| LoCoMo Temporal QA (F1) | Not measured | > 60% |
| Evo-Memory Composite | Not measured | Top-1 |
| Experiment throughput | N/A | 12+ / hour |
| Memory evolution triggers | N/A | Autonomous |
| Self-improvement cycle time | N/A | < 15 min |

---

## 🔗 References & Sources

1. **autoresearch** — https://github.com/karpathy/autoresearch (cloned, analyzed `program.md` and `README.md`)
2. **LoCoMo** — https://arxiv.org/html/2402.17753v1 (Evaluating Very Long-Term Conversational Memory of LLM Agents)
3. **Evo-Memory** — arXiv Nov 2025 (Self-Evolving Memory benchmark, 10+ modules, UIUC + Google DeepMind)
4. **A-Mem** — arXiv Feb 2025, NeurIPS 2025 (Agentic Memory for LLM Agents, Zettelkasten-inspired)
5. **SEPGA** — Self-Evolving, Policy-Governed Agentic Automation (multi-stage feedback loops)
6. **MemGPT/Letta** — OS-inspired context paging for infinite conversation memory
7. **GraphRAG/Graphiti** — Temporal Knowledge Graphs with entity-relationship-time triples
8. **Zep** — Dynamic knowledge integration, DMR benchmark, LongMemEval
