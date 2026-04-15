# Architecture: Xavier2

## Core Philosophy
Xavier2 is a single Rust binary acting as a multi-agent cognitive memory swarm, inspired by the **"System 3" paradigm** (Rational Thought, Meta-Cognition, and Error Correction). This system transcends simple vector retrieval by implementing strict reasoning and self-reflection layers before serving responses.

## Product Direction
Xavier2 is not a standalone RAG silo. It is the memory and reasoning substrate for **agentic workflows**, and it must also be able to host RAG retrieval inside those workflows. The intended direction is bidirectional:
- **RAG inside agentic flows**: retrieval, episodic memory, and belief validation are invoked as capabilities inside orchestrated agents.
- **Agentic logic inside RAG**: retrieval can escalate into multi-step reasoning, oversight, tool use, and memory consolidation when the query requires it.

This means the project should be evaluated not only on retrieval quality, but on how well memory improves long-horizon reasoning, task execution, and self-correction inside ADK-style agent systems.

## Tech Stack
- **Language**: Rust
- **Runtime**: Tokio (for massive asynchronous parallelism across agent swarms)
- **Framework**: `zavora-ai/adk-rust` (Agent Development Kit for agnostic, modular agent building)
- **Database / Memory**: SQLite + SQLite-vec for durable shared memory and vector search, plus `QmdMemory` for in-process retrieval workflows
- **Code Index**: `code-graph` SQLite sidecar for AST/symbol indexing exposed through `/code/*`
- **Web Packages**: React/Vite panel client in `panel-ui` and Astro docs site in `docs/site`, coordinated as root npm workspaces
- **Advanced Techniques**: BM25 (Hybrid Search), MCP (Model Context Protocol), QMD Memory, Belief Graphs.
- **Hosted Control Plane**: workspace-scoped token auth, quota-aware usage tracking, and sync policy metadata for managed deployments

## Agent Swarm Layers (System 1-2-3)
1. **System 1 (Retrieval)**: Fast instinct-like agents powered by shared memory, lexical retrieval, and code indexing to fetch raw context and immediate facts quickly.
2. **System 2 (Reasoning)**: Deliberate agents implementing Chain of Thought (CoT) to construct logical answers based on System 1's context.
3. **System 3 (Action / Oversight)**: Meta-cognitive agents that overrule and evaluate System 2's reasoning. They check reasoning steps against memory state and belief relationships, and execute actions or error corrections. If anomalies, contradictions or hallucinations are detected, the response is vetoed and sent back for re-evaluation.

## CRITICAL DECISIONS

| Date       | Decision                                   | Context                                   |
|------------|--------------------------------------------|-------------------------------------------|
| 2026-03-05 | Monolithic Rust binary via `adk-rust`      | Maximizes Tokio parallelism & performance while remaining LLM-agnostic. |
| 2026-03-05 | Multi-Layer System 3 RAG Architecture      | Emulates human rational thought checking to eliminate standard LLM hallucinations.  |
| 2026-03-05 | SurrealDB-backed shared memory direction   | Durable memory and graph state remain a core product direction, even where some subsystems still use in-process stores. |
| 2026-03-10 | Rebranded to Xavier2                        | Transitioning to a production-ready cognitive memory system for OpenClaw. |
| 2026-03-11 | Agentic-first memory substrate             | Xavier2 must support agent workflows with embedded RAG and RAG flows with escalated agentic reasoning. |
| 2026-03-17 | `code-graph` retained as SQLite sidecar    | Source indexing is a real runtime dependency today and is configured independently from shared memory storage. |
| 2026-03-17 | Runtime configuration externalized         | `XAVIER2_HOST`, `XAVIER2_PORT`, and `XAVIER2_CODE_GRAPH_DB_PATH` are canonical runtime controls across local runs, tests, and Docker. |
| 2026-03-19 | Mixed Rust + Node monorepo formalized      | Rust runtime stays in the Cargo workspace; `panel-ui` and `docs/site` are first-class Node packages managed from the repo root. |
| 2026-03-19 | HTTP API documented as agent-safe default  | External agents should prefer authenticated HTTP/curl integration; MCP remains optional for IDE-native tool transport. |
| 2026-03-19 | Workspace-aware hosted surface added       | Auth tokens now resolve to workspace-scoped memory/runtime/session state, enabling quotas, usage reporting, and sync policy controls without changing the core binary deployment model. |
| 2026-03-21 | Enforced WorkspaceRegistry Isolation       | Rejected PRs attempting to use global in-memory workspace state to guarantee correct multi-tenant cloud logic. `main` strictly enforces isolated `WorkspaceRegistry`. |
| 2026-03-29 | Local-first LLM provider defaults           | `ModelProviderKind::Local` is checked first; Ollama (localhost:11434) is default. External providers (Gemini, OpenAI) require explicit API keys. Managed embeddings disabled by default. |
