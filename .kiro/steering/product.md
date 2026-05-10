# Xavier — Product Overview

Xavier is an open-source **cognitive memory runtime for AI agents**. It acts as the central memory and reasoning substrate for agentic workflows, providing durable memory, hybrid retrieval, and multi-step reasoning capabilities.

## Core Purpose

- Serve as the shared memory layer for AI agent swarms (SWAL ecosystem)
- Enable **RAG inside agentic flows** (retrieval, episodic memory, belief validation)
- Enable **agentic logic inside RAG** (retrieval can escalate into multi-step reasoning, tool use, and memory consolidation)

## Key Capabilities

- **Hybrid Search**: BM25 + vector retrieval with RRF fusion (`XAVIER_RRF_K` configurable)
- **Belief Graph**: Graph-based relationship tracking for memory consistency
- **Code Graph Index**: AST/symbol indexing via SQLite sidecar (`/code/*` endpoints)
- **MCP Server**: Model Context Protocol over streamable HTTP transport
- **Multi-tenant Isolation**: Workspace-scoped memory via `WorkspaceRegistry`
- **Semantic Caching**: Tier-1 caching layer to reduce redundant retrievals
- **Overdrive Pipeline**: HyDE, Self-Correction, and RRF reranking
- **Chronicle Module**: Harvest, redact, generate, and publish memory snapshots
- **Agent Spawn**: Multi-provider agent spawning (Ollama, Gemini, OpenAI, Groq)

## System Architecture (System 1-2-3)

1. **System 1 (Retrieval)** — Fast agents using shared memory, lexical retrieval, and code indexing
2. **System 2 (Reasoning)** — Chain-of-Thought agents building logical answers from System 1 context
3. **System 3 (Oversight)** — Meta-cognitive agents that veto hallucinations and trigger re-evaluation

## Runtime URL

Default local instance: `http://localhost:8003`

## Related Plugins

- **Cortex** (`E:\scripts-python\cortex`) — Sync plugin for Xavier
- **PGheart** (`E:\scripts-python\pgheart`) — Companion plugin
