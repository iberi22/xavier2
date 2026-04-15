# code-graph Specification

## Overview

**code-graph** is a fast, lightweight codebase indexer that uses tree-sitter for AST parsing instead of RAG vector embeddings. Based on research from Aider, Claude Code, and Cline showing that RAG is "dead" for code understanding.

## Goals

1. **Fast indexing** (< 1s for 10k files)
2. **Zero config** (auto-detect language and structure)
3. **Multi-language** (Rust, TypeScript, Python, Go, Java, C/C++)
4. **Simple queries** (find, list, stats)
5. **No external services** (SQLite local storage)

## Non-Goals

- Semantic search (use RAG for docs, not code)
- Vector embeddings
- Cloud services
- Complex CI/CD integration

## Architecture

### Data Flow

```
Source Files → WalkDir → Tree-sitter Parser → SQLite → Query Engine → CLI
```

### Storage Schema

```sql
symbols:
  - id (PK)
  - name (indexed)
  - kind (function, struct, class, enum, trait, impl)
  - lang (Rust, TypeScript, Python, etc.)
  - file_path (indexed)
  - start_line, end_line
  - start_col, end_col
  - signature (optional)
  - parent (for methods)
```

## Commands

### scan <path>
Index all code files in a directory.

```bash
code-graph scan ./my-project
```

Features:
- Respects .gitignore
- Skips node_modules, target, __pycache__, etc.
- Parses in parallel (Tokio)
- Shows progress and stats

### find <query>
Find symbols by name (fuzzy search).

```bash
code-graph find "handle_request"
code-graph find "AuthMiddleware"
```

### functions / structs / classes / enums
List all symbols of a specific kind.

```bash
code-graph functions
code-graph structs
code-graph classes
```

### stats
Show indexing statistics.

```bash
code-graph stats
```

## Supported Languages

| Language | Extensions | Status |
|----------|------------|--------|
| Rust | .rs | ✅ Complete |
| TypeScript | .ts, .tsx | ✅ Complete |
| JavaScript | .js, .jsx | ✅ Complete |
| Python | .py | ✅ Complete |
| Go | .go | ✅ Complete |
| Java | .java | ✅ Complete |
| C/C++ | .c, .cpp, .h, .hpp | ⚠️ Partial |

## Performance

| Metric | Target |
|--------|--------|
| Files/second | 1000+ |
| Symbols/second | 10000+ |
| Memory usage | < 100MB |
| DB size | ~1KB per 100 symbols |

## Future Features

- [ ] Call graph analysis
- [ ] Reference finding
- [ ] Import dependency graph
- [ ] LSP integration (find references, goto definition)
- [ ] MCP server for OpenClaw integration

## Research Sources

- Aider tree-sitter strategy
- Claude Code agentic search
- Cline's approach to code understanding
- "Is RAG Dead?" video analysis (the video Bel mentioned)

---
*Spec version: 0.1.0*
*Created: 2026-03-05*
