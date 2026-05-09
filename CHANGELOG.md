# Xavier Changelog

## [0.4.0] - 2026-03-24

### Added
- **TUI Dashboard**: Interactive terminal-based monitor using `ratatui` for real-time memory and metrics visibility.
- **Git-Chunk Synchronization**: Decentralized sync protocol using compressed JSONL chunks for friction-less memory sharing via Git.
- **Local LLM Provider**: Native support for local OpenAI-compatible endpoints (Ollama, LocalAI) via `ModelProviderKind::Local`.
- **Hierarchical Curation**: Memory Manager categorizes facts using CurationAgent (Domain > Topic).
- **Temporal Graph**: Belief Graph now ingests `valid_from` timestamps connected to session context.

### Changed
- **Metadata Flexibility**: Memory documents now support arbitrary JSON metadata, fully queryable and displayed in the TUI.

## [0.3.0] - 2026-03-17

### Added
- **Security Audit**: Performed a comprehensive security review and documented findings in `security_audit_report.md`.
- **REST API Exposure**: Integrated Axum-based HTTP endpoints for memory search, addition, and agent runtime interaction.
- **Enhanced Retrieval**: Implemented hybrid search combining semantic embeddings with keyword-based retrieval.
- **Self-Improving Agents**: Added experimental `self_improve.rs` module for autonomous performance analysis and optimization.
- **Belief Graphs**: Operationalized `belief_graph.rs` to track relationships between memory nodes.

### Changed
- **Architecture Realignment**: Migrated repository structure to comply with Git-Core v3.2 Protocol.
- **Documentation Consolidation**: Centralized system specifications, research, and agent prompts under the `docs/` hierarchy.
- **Auth Middleware**: Standardized `X-Xavier-Token` enforcement across all public endpoints.

### Fixed
- **Docker Integration**: Resolved health check failures and port binding conflicts in the development stack.
- **Dependency Management**: Aligned crate versions for `axum`, `tokio`, and `surrealdb` across workspace members.

---

## [0.2.0] - 2026-03-08

### Added
- **Hybrid Retrieval Engine**: Initial implementation of the multi-stage memory search.
- **MCP Surface**: Added Model Context Protocol support for seamless IDE integration.
- **Persistence Layer**: Integrated SurrealDB as the primary storage engine for belief graphs.

---

## [0.1.0] - 2026-03-01

- **Initial Baseline**: Core Rust-native runtime for agent memory orchestration.
