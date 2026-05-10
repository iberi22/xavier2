# Xavier — Project Structure

## Repository Layout

```
xavier/
├── src/                    # Main Rust source (library + binaries)
│   ├── main.rs             # Server + CLI entry point
│   ├── lib.rs              # Library root
│   ├── workspace.rs        # Multi-tenant WorkspaceRegistry
│   ├── settings.rs         # Runtime configuration
│   ├── agents/             # System 1-2-3 cognitive agent layers
│   ├── memory/             # QMD Memory + Belief Graph
│   ├── server/             # HTTP API + MCP server (Axum)
│   ├── api/                # Route handlers
│   ├── retrieval/          # Hybrid search (BM25 + vector + RRF)
│   ├── search/             # Search utilities
│   ├── embedding/          # Embedding providers
│   ├── chronicle/          # Harvest / redact / publish pipeline
│   ├── security/           # E2E encryption, prompt injection detection
│   ├── sync/               # Chunk-based sync protocol
│   ├── session/            # Session management
│   ├── scheduler/          # Task scheduling (cron)
│   ├── coordination/       # Agent coordination
│   ├── checkpoint/         # Checkpoint / resume
│   ├── consolidation/      # Memory consolidation
│   ├── consistency/        # Belief consistency checks
│   ├── context/            # Context routing
│   ├── crypto/             # Cryptographic utilities
│   ├── domain/             # Core domain types
│   ├── ports/              # Hexagonal architecture ports
│   ├── adapters/           # Hexagonal architecture adapters
│   ├── billing/            # Usage tracking / quota
│   ├── secrets/            # Secret management
│   ├── tasks/              # Task runtime
│   ├── tools/              # Agent tools
│   ├── telegram/           # Telegram bot integration
│   ├── time/               # Time utilities
│   ├── ui/                 # UI helpers
│   ├── utils/              # Shared utilities
│   ├── verification/       # Atomic verification
│   ├── a2a/                # Agent-to-agent protocol
│   └── bin/                # Additional binary helpers
├── code-graph/             # SQLite AST/symbol index sidecar (separate Cargo crate)
├── panel-ui/               # React + Vite + Tauri desktop panel (npm workspace)
├── docs/site/              # Astro Starlight documentation site (npm workspace)
├── tests/                  # Integration and E2E tests
│   ├── e2e.rs
│   ├── integration.rs
│   └── integration/        # Modular integration test suites
├── benches/                # Criterion benchmarks
├── benchmarks/             # Benchmark result archives
├── scripts/                # PowerShell + Bash utility scripts
├── docker/                 # Dockerfiles and compose variants
├── config/                 # Runtime config (xavier.config.json)
├── data/                   # SQLite DB files (gitignored in prod)
├── docs/                   # Architecture, API, ADR, and guide docs
├── skills/                 # Agent skill definitions
├── memory/                 # Daily memory notes (YYYY-MM-DD.md)
├── web/                    # Web UI (Vite + Tailwind, separate package)
├── plugins/                # Plugin source files
├── state/                  # Runtime state (auth profiles, models)
├── logs/                   # Runtime logs
├── bin/                    # Pre-built binaries
├── .gitcore/               # GitCore agent documentation protocol
│   ├── ARCHITECTURE.md     # Architectural decisions and philosophy
│   ├── STATE.md            # Current project status and module health
│   ├── SDLC_WORKFLOW.md    # Development lifecycle and commit conventions
│   ├── SRC_CONFIG.md       # Requirements ID conventions
│   ├── planning/           # PLANNING.md + TASK.md
│   └── rules/              # Agent integration rules
├── .github/
│   ├── workflows/          # CI/CD (cargo test, clippy, benchmarks, docs)
│   ├── issues/             # Tracked issues in markdown
│   └── instructions/       # Per-AI-tool instruction files
├── Cargo.toml              # Rust workspace root
├── package.json            # Node monorepo root
├── docker-compose.yml      # Primary compose file
└── SOUL.md / USER.md       # Project identity documents
```

## Architecture Pattern

Xavier follows a **hexagonal architecture** (ports & adapters):
- `src/domain/` — core domain types, no external dependencies
- `src/ports/` — abstract interfaces (traits)
- `src/adapters/` — concrete implementations (SQLite, HTTP, etc.)

## Key Conventions

### Commit Format
```
type(scope): description #issue

# Types: feat, fix, docs, refactor, test, chore
# Scopes: memory, server, agents, workspace, security
```

### Branch Naming
```
feat/description-#issue
fix/description-#issue
```

### Requirement IDs
```
XAVIER-FUN-NNN   # Functional requirement
XAVIER-NF-NNN    # Non-functional requirement
XAVIER-INT-NNN   # Interface requirement
XAVIER-DB-NNN    # Database entity
```

### Workspace Isolation
All memory and runtime state is scoped to a `WorkspaceRegistry`. Never use global in-memory workspace state — this was explicitly rejected to ensure correct multi-tenant behavior.

### LLM Provider Priority
`Local` (Ollama at `localhost:11434`) is checked first. External providers require explicit API keys and must not be assumed available.

## Agent Documentation (GitCore Protocol)

All agent-facing documentation lives in `.gitcore/`. Read these at the start of complex tasks:
- `STATE.md` — current build/test/module status
- `ARCHITECTURE.md` — critical decisions and philosophy
- `SDLC_WORKFLOW.md` — development workflow and commands
- `planning/TASK.md` — active task tracking
