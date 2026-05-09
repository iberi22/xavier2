---
title: "Skill Index"
type: INDEX
id: "index-skills"
created: 2026-05-09
updated: 2026-05-09
agent: antigravity
model: gemini-3-flash
requested_by: system
summary: |
  Index of available skills for the Xavier ecosystem. Legacy agent personas have been deprecated in favor of these modular capabilities.
keywords: [skills, capabilities, index]
tags: ["#index", "#skills"]
project: Xavier
---

# 🧠 Skill Index

Legacy agent roles in `.github/agents` have been deprecated. Capabilities are now encapsulated in **Skills**, which provide deep domain logic and protocols.

---

## 📂 Active Skills

| Skill Name | Description | Path |
| :--- | :--- | :--- |
| **Agentic Memory Ops** | Protocols for autonomous memory management, context engineering, and adaptive RAG. | `[.agents/skills/agentic-memory-ops](file:///e:/scripts-python/xavier/.agents/skills/agentic-memory-ops/SKILL.md)` |
| **Xavier Memory (MCP)** | Durable knowledge layer via MCP transport. Includes tools for memory search, creation, and project context. | `[.agents/skills/cortex-memory](file:///e:/scripts-python/xavier/.agents/skills/cortex-memory/SKILL.md)` |

---

## 🛠️ How to Use Skills

Skills are automatically available to the agent via the system instructions. When a task requires a specific capability (e.g., "Deep memory search" or "Context pruning"), the agent should:

1.  **Recall**: Check the Skill Index for the relevant capability.
2.  **Read**: View the `SKILL.md` file for that skill.
3.  **Execute**: Follow the documented protocols and use the provided tools/scripts.

---

## 🚀 Future Skills (Roadmap)

- [ ] **Distributed Sync Skill**: Protocols for multi-node P2P memory synchronization.
- [ ] **Observability Skill**: Integration with Prometheus/Grafana for monitoring Xavier health.
- [ ] **Code Graph Specialist**: Deep AST-based code understanding and symbol navigation.

---

*Xavier Ecosystem - Moving beyond agents to specialized skills.*
