# Architecture - code-graph

## CRITICAL DECISIONS

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Tree-sitter (NOT RAG) | Based on Aider/Claude Code research - RAG is dead for code |
| 2 | SQLite (NOT Vector DB) | Fast, local, no embedding overhead |
| 3 | Async with Tokio | Fast parallel parsing |
| 4 | Multi-language | Rust, TS, Python, Go, Java, C/C++ |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI (main.rs)                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │  Scanner    │  │   Indexer   │  │   Query Engine     │ │
│  │  (walkdir)  │─▶│(tree-sitter)│─▶│  (SQL queries)     │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
│         │                │                    │             │
│         ▼                ▼                    ▼             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              SQLite (code_graph.db)                   │   │
│  │  - symbols    - imports    - exports    - refs       │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Key Components

### 1. Parser (tree-sitter)
- Rust, TypeScript, Python, Go, Java, C/C++
- Extract functions, structs, classes, enums, traits
- Get signatures, line numbers, column positions

### 2. Indexer
- Walk directory tree (respects .gitignore)
- Parse files in parallel (Tokio Semaphore)
- Batch insert to SQLite

### 3. Query Engine
- Find by name (fuzzy)
- Filter by kind (function, struct, class)
- Filter by language

## Why NOT RAG?

Based on research from Aider, Claude Code, Cline:
- Code is structured, not unstructured text
- Exact matches work better than semantic search
- Tree-sitter gives AST structure
- Fast index is more important than semantic understanding

---
*Last updated: 2026-03-05*
