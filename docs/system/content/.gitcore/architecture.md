# Architecture: Xavier

## Core Philosophy
Xavier is a single Rust binary acting as a multi-agent cognitive memory swarm, inspired by the **"System 3" paradigm** (Rational Thought, Meta-Cognition, and Error Correction). This system transcends simple vector retrieval by implementing strict reasoning and self-reflection layers before serving responses.

## Product Direction
Xavier is not a standalone RAG silo. It is the memory and reasoning substrate for **agentic workflows**, and it must also be able to host RAG retrieval inside those workflows. The intended direction is bidirectional:
- **RAG inside agentic flows**: retrieval, episodic memory, and belief validation are invoked as capabilities inside orchestrated agents.
- **Agentic logic inside RAG**: retrieval can escalate into multi-step reasoning, oversight, tool use, and memory consolidation when the query requires it.

This means the project should be evaluated not only on retrieval quality, but on how well memory improves long-horizon reasoning, task execution, and self-correction inside ADK-style agent systems.

## Tech Stack
- **Language**: Rust
- **Runtime**: Tokio (for massive asynchronous parallelism across agent swarms)
- **Framework**: `zavora-ai/adk-rust` (Agent Development Kit for agnostic, modular agent building)
- **Database / Memory**: SurrealDB (Unifies Vector Search and Graph relations for Shared Memory and Belief Graphs)
- **Advanced Techniques**: BM25 (Hybrid Search), MCP (Model Context Protocol), QMD Memory, Belief Graphs.

## Agent Swarm Layers (System 1-2-3)
1. **System 1 (Retrieval)**: Fast instinct-like agents powered by SurrealDB Vector Search and BM25 to fetch raw context and immediate facts quickly.
2. **System 2 (Reasoning)**: Deliberate agents implementing Chain of Thought (CoT) to construct logical answers based on System 1's context.
3. **System 3 (Action / Oversight)**: Meta-cognitive agents that overrule and evaluate System 2's reasoning. They check reasoning steps against a Formal Belief Graph (Graph Nodes in SurrealDB) and execute actions or error corrections. If anomalies, contradictions or hallucinations are detected, the response is vetoed and sent back for re-evaluation.

## CRITICAL DECISIONS

| Date       | Decision                                   | Context                                   |
|------------|--------------------------------------------|-------------------------------------------|
| 2026-03-05 | Monolithic Rust binary via `adk-rust`      | Maximizes Tokio parallelism & performance while remaining LLM-agnostic. |
| 2026-03-05 | Multi-Layer System 3 RAG Architecture      | Emulates human rational thought checking to eliminate standard LLM hallucinations.  |
| 2026-03-05 | SurrealDB as unified Graph/Vector memory   | Simplifies deployment & powers Belief Graph construction dynamically. |
| 2026-03-10 | Rebranded to Xavier                        | Transitioning to a production-ready cognitive memory system for OpenClaw. |
| 2026-03-11 | Agentic-first memory substrate             | Xavier must support agent workflows with embedded RAG and RAG flows with escalated agentic reasoning. |
