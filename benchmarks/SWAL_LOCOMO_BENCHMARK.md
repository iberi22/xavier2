# SWAL Operations Memory Benchmark (LoCoMo-SWAL)

**Framework Version:** 1.0
**Created:** 2026-04-05
**Purpose:** Evaluate Xavier memory system against real SWAL operational use cases
**Based on:** LoCoMo (Long Context Memory) benchmark principles

---

## Overview

This benchmark evaluates the Xavier memory system using real SWAL operational queries derived from actual agent sessions, memory files, and project data. Unlike synthetic benchmarks, SWAL-LoCoMo uses genuine operational questions that BELA and the ventas agent encounter daily.

---

## Memory Categories (from real data)

### 1. Client Information
- **Leonardo Duque**: Vendedor/socio externo, works with Rodacenter (Chile)
- **Rodacenter**: Cliente potencial in Antofagasta, empresa: tripro.cl
- **Tripro**: Company with manteniapp demo at tripro.cl/manteniapp

### 2. Technical Decisions
- **Xavier v0.4.1**: Running with FileMemoryStore (SurrealDB disabled due to protocol mismatch)
- **pplx-embed**: Healthy, 1024d embeddings
- **HyDE**: Disabled (XAVIER_DISABLE_HYDE=1)
- **SurrealDB**: v2.6.4 running but NOT used (http vs ws protocol issue)

### 3. Project Status
- **gestalt-rust**: Active project
- **manteniapp**: SaaS product - Starter $499/mo, Pro $999/mo, Enterprise $2,499/mo
- **xavier**: Memory system, v0.4.1
- **tripro_landing_page_astro**: Live at tripro.cl

### 4. Sales Interactions
- **ManteniApp RFI**: Interested in maintenance monitoring with AI
- **Pricing**: Enterprise software sales
- **Demo**: tripro.cl/manteniapp

### 5. Agent Operations
- **Cron jobs**: Project Synthesizer (6h), Security Audit (daily 8AM), GitHub Monitor (1h)
- **Skills**: sales-pro, src-generator installed locally
- **BELA timezone**: America/Bogota

---

## Task Types

### A. Single-Hop Retrieval (Factual Recall)

Direct lookup of a specific fact stored in memory.

| ID | Query | Expected Answer Source |
|----|-------|------------------------|
| SH-01 | What is BELA's timezone? | USER.md ? America/Bogota |
| SH-02 | What is ManteniApp's pricing? | MEMORY.md ? Starter $499, Pro $999, Enterprise $2,499 |
| SH-03 | Who is Leonardo working with? | MEMORY.md ? Rodacenter (Chile) |
| SH-04 | What product are we selling to Rodacenter? | MEMORY.md ? ManteniApp |
| SH-05 | What is Xavier's current version? | BENCHMARKS.md ? v0.4.1 |
| SH-06 | What is pplx-embed status? | 2026-04-05.md ? Healthy |
| SH-07 | What is the Tripro demo URL? | MEMORY.md ? tripro.cl/manteniapp |
| SH-08 | Where should projects be stored? | MEMORY.md ? E:\scripts-python\ |

### B. Multi-Hop Reasoning

Requires connecting multiple pieces of information.

| ID | Query | Required Connections |
|----|-------|---------------------|
| MH-01 | Who worked on Xavier fixes and what decisions were made? | 2026-04-05.md + MEMORY.md connections |
| MH-02 | Find a client interested in maintenance monitoring with AI | MEMORY.md ? Rodacenter + ManteniApp |
| MH-03 | What projects involve Chile and what is the status? | tripro_landing_page + Rodacenter + manteniapp |
| MH-04 | What are the active SWAL cron jobs? | MEMORY.md ? Project Synthesizer, Security Audit, etc. |
| MH-05 | What security measures are active for SWAL? | 2026-03-31.md ? Security Audit, GitHub monitoring |
| MH-06 | What is the Xavier architecture for memory? | memory-store.json ? schema, bridge, stores |

### C. Temporal Reasoning

Questions about timing and sequence of events.

| ID | Query | Expected Temporal Data |
|----|-------|------------------------|
| TR-01 | When was pplx-embed fixed? | 2026-04-05.md ? "Fixed pplx-embed (docker restart)" |
| TR-02 | What decisions were made about SurrealDB persistence? | 2026-04-05.md ? FileMemoryStore chosen |
| TR-03 | What happened in the last Xavier session? | 2026-04-05.md ? Full night of fixes |
| TR-04 | When was the last security audit? | 2026-03-31.md ? 8 AM daily |
| TR-05 | What is the timeline of Xavier versions? | BENCHMARKS.md ? v0.4.0 (failed), v0.4.1 (current) |

### D. Open-Domain Reasoning

Broad questions requiring synthesis and analysis.

| ID | Query | Requires |
|----|-------|----------|
| OD-01 | Summarize the Xavier memory system improvements made | BENCHMARKS.md + 2026-04-05.md synthesis |
| OD-02 | What is the overall status of SWAL operations? | Cross-reference all memory files |
| OD-03 | Analyze the memory hygiene situation | Identify gaps, duplicates, stale data |
| OD-04 | What skills are available for sales operations? | MEMORY.md + TOOLS.md synthesis |
| OD-05 | What is the complete SWAL product portfolio? | MEMORY.md ? Xavier, ManteniApp, Software Factory, etc. |

---

## Evaluation Metrics

### Primary Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| **Precision** | Did we retrieve the RIGHT memory? (1-5 scale) | = 4.0 |
| **Recall** | Did we retrieve ALL relevant memories? | = 80% |
| **Latency (Cold)** | First search for a query | < 500ms |
| **Latency (Warm)** | Subsequent search for same query | < 100ms |
| **False Positive Rate** | Irrelevant results returned | < 20% |

### Secondary Metrics

| Metric | Description |
|--------|-------------|
| **Relevance Ranking** | Most relevant result at top? (1-5) |
| **Context Completeness** | Enough context to answer? (1-5) |
| **Answer Accuracy** | Correct answer delivered? (Y/N) |
| **Hallucination Rate** | Fabricated information? (Y/N) |

---

## Scoring Rubric

### Precision Score (per query)
- **5**: Exact match, complete context
- **4**: Relevant result, minor missing details
- **3**: Partially relevant, key info missing
- **2**: Tangentially related
- **1**: Completely irrelevant

### Recall Score (per query)
- **100%**: All relevant memories retrieved
- **75%**: Most relevant retrieved, 1-2 missing
- **50%**: Some relevant, significant missing
- **25%**: Few relevant found
- **0%**: No relevant memories found

---

## Baseline Targets

| Task Type | Precision Target | Recall Target | Latency Target |
|-----------|-----------------|---------------|----------------|
| Single-Hop | 4.5 | 90% | < 200ms |
| Multi-Hop | 4.0 | 80% | < 500ms |
| Temporal | 4.0 | 85% | < 300ms |
| Open-Domain | 3.5 | 70% | < 1000ms |

---

## Implementation

### Script Location
`E:\scripts-python\scripts\swal-locomo-benchmark.ps1`

### Execution
```powershell
# Run full benchmark
E:\scripts-python\scripts\swal-locomo-benchmark.ps1

# Run specific category only
E:\scripts-python\scripts\swal-locomo-benchmark.ps1 -Category single_hop
```

### Output
- Console: Real-time progress and summary
- JSON: `E:\scripts-python\xavier\benchmark-results\swal-locomo-{date}.json`
- Markdown: `E:\scripts-python\xavier\BENCHMARKS\SWAL_LOCOMO_BASELINE_{date}.md`

---

## Benchmark Evolution

This benchmark grows with SWAL operations:
- New queries added as new use cases emerge
- Queries refined based on retrieval failures
- Categories expanded as new memory types added

**Last Updated:** 2026-04-05
