# 🧠 Global Xavier Integration Rule (Core Memory)

Xavier es el **Memory Core** (núcleo de memoria) de toda la operación de bots. **Cortex** es ahora un **plugin** de Xavier encargado de la sincronización empresarial.

## 🛡️ Identity & Backend
- **Core System**: Xavier (Cerebro central).
- **Enterprise Plugin**: Cortex (Sync & Cloud).
- **Core Function**: Contextual engine, semantic memory, and task verification.
- **Service URL**: `http://localhost:8003` (Standard).
- **Token**: `dev-token`.

## 🌊 Cascade Flow Integration
Xavier debe integrarse completamente en los flujos de **Cascade** (Antigravity Agentic Mode):

1. **Turn-Based Context**: En cada turno de Cascade, el agente debe verificar si hay contexto nuevo en Xavier que afecte la decisión actual.
2. **Atomic Verification**: Cada paso verificado en Cascade debe ser persistido en Xavier inmediatamente.
3. **Cross-Agent Sync**: Cascade utiliza Xavier para asegurar que el estado sea consistente entre diferentes agentes (Xavier, Bela, Leonardo, etc.).

## 📡 Essential Protocols

### 1. Pre-Task Context Retrieval (Recall)
Before starting any complex task, refactor, or architectural change, agents **MUST** query Xavier for relevant past decisions or established patterns.
> **Action**: `POST /memory/query` or `POST /memory/search`.

### 2. Post-Task Verification (Retention)
After completing a task, agents **MUST** store a summary of the changes and the verified status in Xavier. This ensures that future sessions (even on different projects) have access to this "experience."
> **Action**: `POST /memory/add` with `type: task_verification` in metadata.

### 3. Cross-Project Knowledge
Xavier acts as a shared brain. If a pattern (e.g., "how to stabilize a Rust CI") is solved in one project, it should be queryable from another.
- **Path tagging**: Use the `path` field in memory items to categorize (e.g., `tasks/cleanup`, `architecture/auth`).

## 🤖 Interaction Interface

| Operation | Endpoint | Purpose |
| :--- | :--- | :--- |
| **Add Memory** | `/memory/add` | Store new knowledge/task status. |
| **Semantic Query** | `/memory/query` | Find conceptually related information. |
| **Symbol Search** | `/code/find` | Locate code across the indexed workspace. |

## ⚠️ Mandatory Alignment
- **No Placeholders**: Never store "TODO" items in Xavier; only verified state or deep research.
- **Privacy**: Never store secrets (keys, tokens) in Xavier memory.
- **Token Header**: Always include `X-Cortex-Token: dev-token`.

---
*Generated for Google Antigravity IDE integration.*
