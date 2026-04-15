---
title: "Agent CLI Guide"
description: "Instructions for agents on using the Git-Core ecosystem CLIs"
---

# 🛠️ Agent CLI Guide

This guide defines the standard operating procedures for using Command Line Interfaces (CLIs) within the Git-Core ecosystem. All agents must follow these usage patterns to ensure stability and state consistency.

## 🌟 The "Git-Core" Ecosystem

We operate with a stack of specialized CLIs:

1.  **`gc`** (Git-Core): The primary protocol orchestrator.
2.  **`gh`** (GitHub CLI): Interface for GitHub issues, PRs, and API.
3.  **`gemini`** (Gemini CLI): Access to Google Gemini 1.5/2.0 models for context analysis.
4.  **`copilot`** (GitHub Copilot CLI): Code suggestions and explanation.
5.  **`jules`** (Jules CLI): Autonomous task execution engine.

---

## 1. `gc` (Git-Core Protocol)
**Role:** Master Orchestrator & State Manager.
**When to use:** ALWAYS prefer `gc` over native git commands for protocol actions.

### Core Commands
| Command | Description | Agent Usage |
|---------|-------------|-------------|
| `gc init` | Initialize repo | Use only when setting up new workspaces. |
| `gc check` | Validate environment | Run before starting complex tasks. |
| `gc issue list` | View assigned tasks | Use to find what to work on. |
| `gc issue create` | Create new tasks | Use for splitting work or creating sub-tasks. |
| `gc commit` | Atomic commits | **MANDATORY**. Replaces `git commit`. Enforces conventions. |

> **⚠️ IMPORTANT**: Do not use `git commit` directly. Use `gc commit` to ensure commit message validity and hook execution.

---

## 2. `gh` (GitHub CLI)
**Role:** Remote State Synchronization.
**When to use:** Interacting with Issues, PRs, Releases, and Repo settings.

### Standard Patterns
- **List Issues:** `gh issue list --assignee "@me"`
- **View Issue:** `gh issue view <id>`
- **Create PR:** `gh pr create --fill`
- **Comment:** `gh issue comment <id> --body "..."`

---

## 3. `gemini` (Gemini CLI)
**Role:** Large Context Intelligence.
**Usage:** High-volume context analysis, summarization, and complex reasoning.
**Command:** `gemini <prompt>` or `gemini analyze <file>` (depending on implementation).

### Agent Protocol
- Use for **analyzing large files** that exceed your context window.
- Use for **generating reports** or summaries of the codebase.

---

## 4. `copilot` (GitHub Copilot CLI)
**Role:** Code Assistant & Explainer.
**Usage:** `gh copilot explain` or `gh copilot suggest`.

### Agent Protocol
- Use `gh copilot suggest "<query>"` to generated standard boilerplate.
- Use `gh copilot explain` to understand legacy code before modifying.

---

## 5. `jules` (Jules CLI)
**Role:** Autonomous Worker.
**Usage:** Delegating tasks to sub-agents or running background jobs.
**Command:** `jules run <task>`

---

## 🔄 Interaction Flow

1.  **Start:** `gc issue list` (Check assigned work)
2.  **Plan:** `gemini analyze ...` (Understand context)
3.  **Implement:** Edit code.
4.  **Verify:** `gc check` / `cargo test`
5.  **Commit:** `gc commit "feat: implemented X"` (Standardized commit)
6.  **Push:** `git push` (Native git allowed for push)
7.  **PR:** `gh pr create`

## 🛡️ Safety Rules
- **NEVER** use `--force` unless explicitly authorized by user.
- **ALWAYS** check `gc status` (if available) or `git status` before committing.
- **NEVER** hardcode credentials in CLI arguments.
