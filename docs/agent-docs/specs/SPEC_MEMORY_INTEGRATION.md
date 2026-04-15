---
title: "Memory Integration Specification (GitHub Issues vs. Xavier2)"
type: SPECIFICATION
id: "spec-memory-integration"
created: 2026-03-24
updated: 2026-03-24
agent: antigravity
model: gemini-2.0-pro
requested_by: user
summary: |
  Formalizes the distinction between task-level memory (GitHub Issues) and
  long-term repository memory (Xavier2 MCP/HTTP).
keywords: [memory, xavier2, github, issues, state, rag]
tags: ["#memory", "#protocol", "#context"]
---

# 🧠 Memory Integration Specification

## 1. Overview

To maintain a clean and performant workspace, agents must distinguish between **Task State** and **Project Knowledge**. This protocol prevents "context bloat" in rules and documentation while ensuring durable agent learning.

## 2. Memory Split

| Layer | Storage | Lifetime | Purpose |
|-------|---------|----------|---------|
| **Task State** | GitHub Issues (`<agent-state>`) | Ephemeral (until PR merge) | Progress, plan, immediate blockers, next actions. |
| **Project Knowledge** | Xavier2 (MCP/HTTP) | Permanent | Architecture, research, complex logic, past solutions. |

## 3. Usage Patterns

### A. Researching a Task (Read)
Before starting a new issue, agents **MUST**:
1. Search Xavier2 for similar past implementations: `search_memories(query: "...")`.
2. Check for relevant architectural decisions: `query_memories(query: "architecture for ...")`.
3. Use found context to refine the `<plan>` in the GitHub Issue.

### B. Completing a Task (Write)
Upon successful implementation or deep research:
1. Store reusable findings in Xavier2: `add_memory(path: "research/...", content: "...")`.
2. Link to the resolving PR or Issue in the memory metadata.
3. **DO NOT** store temporary checklists or small debugging logs in Xavier2.

## 4. Integration with Context Protocol (v2.1)

The `<memory>` tag in the `<agent-state>` block within GitHub Issues should **ONLY** contain:
- Task-specific variables (e.g., `last_modified_file`).
- References to Xavier2 entry IDs (e.g., `xavier2_ref: "doc-1234"`).
- Immediate, non-reusable state.

## 5. Tooling

- **MCP Tool**: `xavier2-memory` (Primary for IDE-native agents).
- **HTTP/Curl**: For CI/CD and external automation scripts.
- **CLI**: `gc` (Git-Core) for local state and atomic operations.
