# SWAL Memory — Technical Backlog (from Engram analysis)

> Extracted from `docs/engram_extraction_analysis_prompt.md`. Engram is design reference only — these are ideas to consider for Xavier2/Cortex, not direct imports.

---

## 1. Canonical Project Identity
- **What:** Projects have a stable canonical ID across sessions/contexts
- **Why matters:** Prevents fragmentation when same project referenced via different paths
- **Status:** Not implemented in Xavier2/Cortex
- **Priority:** 🟡 Medium

## 2. Session Context Alignment
- **What:** Memory layer understands active session context vs global context
- **Why matters:** Reduces hallucination in retrieval when session scope is explicit
- **Status:** Xavier2 has workspace_id concept, but session-level context not modeled
- **Priority:** 🟡 Medium

## 3. Topic-Key Upsert
- **What:** Merge semantics (update if exists, insert if not) at the topic/thread level
- **Why matters:** Avoids duplicate entries for the same logical memory
- **Status:** Xavier2 has revision tracking, but topic-level dedup not implemented
- **Priority:** 🟡 Medium

## 4. Duplicate Counter / Occurrence Tracking
- **What:** Track how many times the same fact has been added across sessions
- **Why matters:** Enables \"belief strength\" — facts seen once are fragile, seen 50x are solid
- **Status:** Not implemented
- **Priority:** 🔴 Low (nice to have, adds complexity)

## 5. Progressive Retrieval
- **What:** Start with fast/cheap results, escalate to deep/expensive search only if needed
- **Why matters:** For real-time use cases, avoid expensive embedding search when keyword match suffices
- **Status:** Xavier2 has hybrid search but no explicit escalation path
- **Priority:** 🟡 Medium

## 6. MCP Profiles (Multi-Context Protocol)
- **What:** Attach different retrieval/config profiles to different contexts
- **Why matters:** Developer context vs chat context vs automation context need different memory behaviors
- **Status:** Not modeled in Xavier2
- **Priority:** 🟡 Medium

## 7. Token-Savings Benchmark
- **What:** Measure tokens saved vs naive full-context approach
- **Why matters:** Quantify the actual value of the memory layer
- **Status:** Not implemented
- **Priority:** 🟡 Low

## 8. Memory Importance Scoring + Decay
- **What:** Facts that keep being retrieved should be boosted; rarely accessed facts should decay
- **Why matters:** Keeps most relevant memories at top of results
- **Status:** Xavier2 hybrid search scores by relevance, but importance/decay not implemented
- **Priority:** 🟡 Medium

---

## SWAL Priority Order (for next sprint cycle)

1. **Session context alignment** — worth doing now since we have the session concept already
2. **Topic-key upsert** — prevents memory pollution from duplicate adds
3. **Progressive retrieval escalation** — natural extension of existing hybrid search
4. **Memory importance scoring** — core to making RAG results better over time
5. **MCP profiles** — multi-tenant story enhancement
6. Canonical project identity, token benchmark, duplicate counters — later