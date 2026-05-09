---
name: agentic-memory-ops
description: "High-level protocols for autonomous memory management, context engineering, and adaptive RAG workflows in Xavier."
---

# Agentic Memory Operations (2026)

This skill provides the protocols and strategies for managing Xavier's memory in complex agentic workflows. It shifts the focus from simple retrieval to autonomous context engineering and hierarchical reasoning.

## 🧠 Memory Architecture Protocols

### 1. Hierarchical Context Management
Always distinguish between memory layers to ensure high-signal prompts:
- **Working Memory (Short-Term):** Immediate conversation state (limit to ~2-4k tokens).
- **Episodic Memory:** Past session summaries and key decisions.
- **Semantic Memory:** Fact-based knowledge (retrieved via Xavier vector search).
- **Procedural Memory:** "How-to" guides for tools and workflows.

### 2. Context Engineering (Decay & Forgetting)
Don't dump raw history. Use these techniques:
- **Summarization:** Compress conversational turns older than 5 steps.
- **Signal-to-Noise Filtering:** Remove boilerplate or redundant information from retrieved chunks.
- **Forgetting Mechanism:** Explicitly ignore or archive low-relevance memories to prevent "context poisoning."

## 🔍 Advanced Retrieval Strategies

### 1. Adaptive RAG Router
Implement a routing logic BEFORE retrieval:
- **Simple/Factual:** Direct vector search.
- **Reasoning/Complex:** Multi-hop retrieval (breaking query into sub-queries).
- **Exploratory:** "Self-RAG" (agent critiques retrieved data and re-queries if insufficient).

### 2. Hybrid Search + Reranking
- **Step 1:** Execute keyword (BM25) and vector (Semantic) search in parallel.
- **Step 2:** Apply a Reranker (e.g., Cohere/Cross-Encoder) to the top-50 results.
- **Step 3:** Pass only the top-5 highly relevant chunks to the LLM.

## 🛑 Governance & Budgets

### 1. Iteration Budgets
To prevent infinite agent loops in retrieval:
- Set `MAX_RETRIEVAL_STEPS = 3`.
- If no answer is found after 3 attempts, escalate to the user with "Insufficient Context."

### 2. Confidence Thresholds
Agents must assign a confidence score to their retrieved evidence. If `score < 0.7`, the agent must trigger a "Search Expansion" protocol.

## 🛠 Tools & Scripts
- `scripts/context-compressor.ps1`: Summarizes and compresses active context.
- `scripts/memory-decay.py`: Archives old/unreferenced engrams.
- `scripts/xavier-reranker.js`: Utility for reranking search results.
