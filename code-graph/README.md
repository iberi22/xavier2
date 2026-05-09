# code-graph

> **Codebase Understanding without RAG** - Tree-sitter + Agentic Search

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2024+-orange.svg)](https://rust-lang.org)
[![Tokio](https://img.shields.io/badge/Tokio-1.42+-blue.svg)](https://tokio.rs)

## ⚡ Why Not RAG?

Based on research from Aider, Claude Code, and Cline:

| Approach | Use Case | code-graph |
|----------|----------|------------|
| **RAG (Vector DB)** | Unstructured docs (knowledge bases) | ❌ Not needed |
| **Tree-sitter** | Code structure (AST, functions, classes) | ✅ Primary |
| **Agentic Search** | Navigation via filesystem | ✅ Fallback |
| **Symbol Index** | Fast lookup (CTags style) | ✅ Fast path |

## 📦 Installation

```bash
cargo install code-graph
```

## 🚀 Quick Start

```bash
# Scan a project
code-graph scan ./my-project

# Query functions/structs
code-graph find "function_name" --lang rust

# Agentic mode: ask questions
code-graph ask "How does auth work?"
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI (main.rs)                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  Scanner    │  │   Indexer   │  │   Query Engine     │ │
│  │  (walkdir)  │─▶│(tree-sitter)│─▶│  (hybrid search)   │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
│         │                │                    │             │
│         ▼                ▼                    ▼             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              SQLite (code_graph.db)                   │   │
│  │  - symbols    - imports    - exports    - refs       │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 📊 Features

- **Multi-language**: Rust, TypeScript, Python, Go, Java, C++
- **Tree-sitter AST**: Parse functions, classes, structs, imports
- **Fast Index**: SQLite-based symbol index (< 1s for 10k files)
- **Agentic Fallback**: Navigate filesystem when needed
- **Zero Config**: Auto-detect language and structure

## 📋 Commands

| Command | Description |
|---------|-------------|
| `scan <path>` | Index entire codebase |
| `find <query>` | Find symbols |
| `ask <question>` | Agentic Q&A |
| `refs <symbol>` | Find references |
| `graph <func>` | Show call graph |

## 🔧 Tech Stack

- **Runtime**: Tokio 1.42+
- **Parser**: tree-sitter 0.24+
- **Database**: SQLite (rusqlite)
- **CLI**: Clap 4
- **Async**: futures, tokio

## 📄 License

MIT License - See [LICENSE](LICENSE) file.

---

*Based on Aider's tree-sitter strategy + Claude Code's agentic search*
