# 📚 Xavier2 - Documentation System

> **Cognitive Memory for AI Swarms**

This documentation follows the **[Diátaxis](https://diataxis.fr/)** framework - a systematic approach to technical documentation, covering Tutorials, How-To Guides, Reference, and Explanation.

---

## 🧭 Documentation Map

All documentation now lives under `docs/`. The split is:

- `docs/site/` for the published docs site source
- `docs/system/` for the interactive docs system prototype
- `docs/agent-docs/` for persistent agent-facing docs
- the Diátaxis folders in this directory for product/reference content

### 📖 [Tutorials](./tutorials/) - Learn by doing
- Setting up Xavier2 with SurrealDB.
- Integrating Xavier2 with OpenClaw.

### 🎯 [How-To Guides](./how-to/) - Task-oriented
- Configuring Hybrid Search (BM25 + Vector).
- Managing the Belief Graph via CLI.
- Implementing System 3 verification logic.

### 📚 [Reference](./reference/) - Technical facts
- API Endpoint Reference.
- `ARCHITECTURE.md` - Core System 3 design.
- Configuration schemas (`features.json`, `Cargo.toml`).

### 💡 [Explanation](./explanation/) - Understanding concepts
- The System 1-2-3 Paradigm.
- Why SurrealDB? (Unified Graph/Vector storage).
- Multi-agent swarm coordination.

---

## 🤖 AI Agent Documentation

Technical specifications and context for agents interacting with Xavier2.

| Directory | Purpose |
|-----------|---------|
| **[agent-docs/specs/](./agent-docs/specs/)** | Logic and protocol specifications |
| **[agent-docs/research/](./agent-docs/research/)** | Research on System 3 and Belief Graphs |
| **[agent-docs/prompts/](./agent-docs/prompts/)** | Core system prompts |

## 🧩 Documentation Tooling

| Directory | Purpose |
|-----------|---------|
| **[site/](./site/)** | Astro/Starlight site published to GitHub Pages |
| **[system/](./system/)** | Experimental Astro + Svelte documentation system |

---

## 🚀 Quick Navigation

- **[Architecture](../.gitcore/ARCHITECTURE.md)**: Deep dive into the core logic.
- **[Features](../.gitcore/features.json)**: Current implementation status.
- **[Getting Started](../README.md#quick-start)**: Installation and setup.

---

*Xavier2 documentation is a living system. Last updated: March 2026*
