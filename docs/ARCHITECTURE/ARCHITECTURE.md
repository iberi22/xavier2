# Xavier2 Architecture

**Version:** 0.4.1
**Last Updated:** 2026-04-20

---

## System Overview

Xavier2 is a fast vector memory system for AI agents, providing ~7ms average search latency using SQLite-vec with hybrid retrieval (vector + keyword + graph).

```
┌─────────────────────────────────────────────────────────────┐
│                      Xavier2 v0.4.1                        │
├───────────────┬───────────────┬─────────────────────────────┤
│   CLI Tool    │   HTTP API    │      MCP-stdio              │
├───────────────┴───────────────┴─────────────────────────────┤
│                  Security Layer                            │
│            (Prompt Injection Detection)                    │
├─────────────────────────────────────────────────────────────┤
│                  Hybrid Search Engine                      │
│    ┌──────────┬──────────┬──────────┬──────────┐           │
│    │ Vector   │  FTS5    │  Graph   │  RRF     │           │
│    │ (vec)    │ (BM25)   │ (Entity) │ Fusion   │           │
│    └──────────┴──────────┴──────────┴──────────┘           │
├─────────────────────────────────────────────────────────────┤
│              SQLite-vec Storage                             │
│    ┌──────────────┬──────────────┬──────────────┐           │
│    │ memory_vec   │ memory_fts   │  entities    │           │
│    │   (vector)   │   (text)     │   (graph)    │           │
│    └──────────────┴──────────────┴──────────────┘           │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. CLI (`src/cli.rs`)

Human-friendly command interface for:
- `xavier2 http` - Start HTTP server
- `xavier2 mcp` - Start MCP-stdio server
- `xavier2 search <query>` - Search memories
- `xavier2 add <content>` - Add memory
- `xavier2 stats` - Show statistics
- `xavier2 code scan|find|stats` - Code graph operations

All CLI commands pass through `secure_cli_input()` for security scanning.

### 2. HTTP Server (`src/server/`)

Axum-based HTTP server with the following endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/ready` | GET | Readiness check |
| `/memory/search` | POST | Vector search |
| `/memory/add` | POST | Add memory |
| `/memory/stats` | GET | Memory statistics |
| `/memory/query` | POST | Hybrid query |
| `/security/scan` | POST | Security scan |
| `/code/scan` | POST | Scan code directory |
| `/code/find` | POST | Find code symbols |
| `/code/context` | POST | Get code context |
| `/code/stats` | GET | Code graph stats |

### 3. Security Layer (`src/security/`)

Prompt injection detection with multiple layers:

```
Input → phrase detection → homoglyph → encoding → heuristic → threat_category
         ↓                    ↓           ↓           ↓            ↓
      Block/Pass         Block/Pass   Block/Pass  Block/Pass   Block/Pass
```

**Layers:**
- `phrase.rs` - Common injection phrases
- `homoglyph.rs` - Unicode homoglyph attack detection
- `encoding.rs` - Mixed encoding evasion detection
- `heuristic.rs` - Pattern-based heuristics
- `threat_categories.rs` - Threat categorization
- `canary.rs` - Canary token detection
- `entropy.rs` - High entropy secret detection

**Thresholds:**
- Confidence >= 0.5 → Block
- Auto-sanitize if `auto_sanitize` enabled

### 4. Memory System (`src/memory/`)

**Storage Backend:** SQLite-vec (persistent, embedded)

**Tables:**
- `memory_vec` - Vector embeddings (768-dim nomic-embed-text)
- `memory_fts` - FTS5 full-text search
- `entities` - Knowledge graph nodes
- `relations` - Knowledge graph edges

**Search Pipeline:**
```
1. Vector search (sqlite-vec) → top-k by cosine similarity
2. FTS5 search (BM25) → top-k by relevance
3. Graph traversal (entity relations) → top-k by connectivity
4. RRF fusion → combined ranking with k=60
```

### 5. Embedding (`src/memory/embedder.rs`)

**Provider:** Ollama with `nomic-embed-text` model
- **Dimensions:** 768
- **Latency:** ~24ms per embedding
- **Quantization:** F16
- **Model size:** 274 MB

### 6. Code Graph (`src/code_graph/`)

Index and search code symbols across repositories:

- **Symbol types:** Function, Struct, Class, Enum, Trait, Module
- **Languages:** Rust, Python, JavaScript, TypeScript, Go, etc.
- **Operations:** scan, find, context, stats

## Data Flow

### Add Memory
```
User → CLI "add" → secure_cli_input() → SecurityService.process_input()
     → HTTP POST /memory/add → Validate → Embed (Ollama)
     → Store (sqlite-vec + fts5 + entities)
```

### Search Memory
```
User → CLI "search" → secure_cli_input() → SecurityService.process_input()
     → HTTP POST /memory/search → Validate → Embed (Ollama)
     → Vector search + FTS5 + Graph → RRF fusion
     → Return ranked results
```

### Security Scan
```
Request → SecurityService.process_input()
       → Layer 1: phrase detection
       → Layer 2: homoglyph check
       → Layer 3: encoding analysis
       → Layer 4: heuristic analysis
       → Layer 5: threat categorization
       → Combined confidence score
       → Allow/Block with sanitization
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `XAVIER2_PORT` | `8006` | HTTP server port |
| `XAVIER2_HOST` | `0.0.0.0` | Bind address |
| `XAVIER2_TOKEN` | `dev-token` | Authentication |
| `XAVIER2_DEV_MODE` | `false` | Skip auth (dev) |
| `XAVIER2_LOG_LEVEL` | `info` | Log verbosity |

## Performance

- **Vector search:** ~7ms average
- **Add memory:** ~50ms (including embedding)
- **Search:** ~50-100ms end-to-end
- **Code scan:** O(n) where n = lines of code

## Storage

- **Memory DB:** `xavier2_memory_vec.db` (~400-500 memories)
- **Code DB:** `data/code_graph.db` (141 files, ~2669 symbols)

## Security Model

All inputs are scanned for:
1. **Prompt injection** - Direct and indirect
2. **Data exfiltration** - Context leakage, credential leak
3. **Path traversal** - Directory escape in code operations
4. **Unicode attacks** - Homoglyph spoofing
5. **Encoding evasion** - Mixed script obfuscation

Security is applied at:
- HTTP handler entry points (all 11 endpoints)
- CLI command entry points (search, add)
- Code operations (scan, find, context)

## Extensions

- [Security](./SECURITY.md) - Detailed security documentation
- [Benchmark Comparison](../BENCHMARK_COMPARISON.md) - Performance vs competitors
- [ROADMAP](../ROADMAP.md) - Development roadmap