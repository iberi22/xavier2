# Xavier2 Environment Variables Reference

> Complete inventory of every environment variable used in the xavier2 codebase  
> Last updated: 2026-05-06 | Sources: 33+ `.rs` files under `src/`

---

## How to Read This

- **Config JSON**: Whether this var maps to a key in `config/xavier2.config.json`
- **Source Files**: Primary files that define/read this variable
- **Required**: `✅` = must be set, `⚠️` = dev-mode bypass available, `❌` = optional

---

## Table of Contents

1. [Server & Network](#1-server--network)
2. [Workspace & Billing](#2-workspace--billing)
3. [Memory Backend](#3-memory-backend)
4. [Embedding](#4-embedding)
5. [LLM Provider](#5-llm-provider)
6. [Provider-Specific Credentials](#6-provider-specific-credentials)
7. [Routing](#7-routing)
8. [Memory Layers](#8-memory-layers)
9. [Context Budget](#9-context-budget)
10. [Synchronization](#10-synchronization)
11. [Vector Store Features](#11-vector-store-features)
12. [Retrieval](#12-retrieval)
13. [Telegram Bot](#13-telegram-bot)
14. [Billing — Stripe](#14-billing--stripe)
15. [Project Management — Planka](#15-project-management--planka)
16. [CLI / Legacy](#16-cli--legacy)
17. [Rust Logging](#17-rust-logging)

---

## 1. Server & Network

| Variable | Type | Config JSON | Default | Required | Description |
|----------|------|-------------|---------|----------|-------------|
| `XAVIER2_HOST` | `string` | `server.host` | `"127.0.0.1"` | ❌ | IP address to bind the HTTP server |
| `XAVIER2_PORT` | `u16` | `server.port` | `8006` | ❌ | HTTP server port |
| `XAVIER2_URL` | `string` | — | `"http://localhost:8006"` | ❌ | Full URL for client, verification, and sync connections |
| `XAVIER2_TOKEN` | `string` | — | (random if dev mode) | ✅ | Authentication token for HTTP API and CLI clients |
| `XAVIER2_TOKEN_SECRET` | `string` | — | — | ✅ | Secret key for HMAC signing in sync/verification |
| `XAVIER2_DEV_MODE` | `bool` | — | `false` | ❌ | Bypass auth requirements; generates random token if unset |
| `XAVIER2_LOG_LEVEL` | `string` | `server.log_level` | `"info"` | ❌ | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `XAVIER2_CODE_GRAPH_DB_PATH` | `string` | `server.code_graph_db_path` | `"data/code_graph.db"` | ❌ | Path to code graph SQLite DB |
| `XAVIER2_CONFIG_PATH` | `string` | — | `"config/xavier2.config.json"` | ❌ | Override config file location |
| `XAVIER2_ALLOWED_DOMAINS` | `string` | — | (none) | ❌ | Comma-separated domain allowlist for URL validator (test-only) |

**Source**: `src/settings.rs`, `src/main.rs`, `src/cli/auth.rs`, `src/cli/utils.rs`, `src/server/http.rs`, `src/security/url_validator.rs`

---

## 2. Workspace & Billing

| Variable | Type | Config JSON | Default | Required | Description |
|----------|------|-------------|---------|----------|-------------|
| `XAVIER2_DEFAULT_WORKSPACE_ID` | `string` | `workspace.default_workspace_id` | `"default"` | ❌ | Default workspace ID |
| `XAVIER2_WORKSPACE_ID` | `string` | — | `"default"` | ❌ | Legacy fallback workspace ID |
| `XAVIER2_DEFAULT_PLAN` | `string` | `workspace.default_plan` | `"community"` | ❌ | Billing plan: `community`, `pro`, `cloud`, `enterprise` |
| `XAVIER2_STORAGE_LIMIT_BYTES` | `u64?` | `workspace.storage_limit_bytes` | `null` | ❌ | Optional storage cap |
| `XAVIER2_REQUEST_LIMIT` | `usize?` | `workspace.request_limit` | `null` | ❌ | Optional request count cap |
| `XAVIER2_REQUEST_UNIT_LIMIT` | `u64?` | `workspace.request_unit_limit` | `null` | ❌ | Optional request unit cap |
| `XAVIER2_SYNC_POLICY` | `string` | `workspace.sync_policy` | `"local_only"` | ❌ | Memory sync policy |
| `XAVIER2_EMBEDDING_PROVIDER_MODE` | `string` | `workspace.embedding_provider_mode` | `"bring_your_own"` | ❌ | Embedding provider: `local`, `cloud`, `bring_your_own`, `disabled` |
| `XAVIER2_MANAGED_GOOGLE_EMBEDDINGS` | `bool` | `workspace.managed_google_embeddings` | `false` | ❌ | Enable managed Google embeddings |

**Source**: `src/settings.rs`, `src/workspace.rs`, `src/memory/mod.rs`

---

## 3. Memory Backend

| Variable | Type | Config JSON | Default | Required | Description |
|----------|------|-------------|---------|----------|-------------|
| `XAVIER2_MEMORY_BACKEND` | `string` | `memory.backend` | `"vec"` | ❌ | Storage backend: `vec`, `sqlite`, `file` |
| `XAVIER2_DATA_DIR` | `string` | `memory.data_dir` | `"data"` | ❌ | Base data directory |
| `XAVIER2_EMBEDDING_DIMENSIONS` | `usize` | `memory.embedding_dimensions` | `768` | ❌ | Embedding vector dimensions |
| `XAVIER2_WORKSPACE_DIR` | `string` | `memory.workspace_dir` | `"data/workspaces"` | ❌ | Workspace data directory |
| `XAVIER2_MEMORY_FILE_PATH` | `string` | `memory.file_path` | `"data/workspaces/default/memory-store.json"` | ❌ | JSON file store path |
| `XAVIER2_MEMORY_SQLITE_PATH` | `string` | `memory.sqlite_path` | `"data/memory-store.sqlite3"` | ❌ | SQLite store path |
| `XAVIER2_MEMORY_VEC_PATH` | `string` | `memory.vec_path` | `"data/vec-store.sqlite3"` | ❌ | Vector SQLite store path |

**Source**: `src/settings.rs`, `src/memory/sqlite_store.rs`, `src/memory/sqlite_vec_store.rs`

---

## 4. Embedding

| Variable | Type | Config JSON | Default | Required | Description |
|----------|------|-------------|---------|----------|-------------|
| `XAVIER2_EMBEDDING_URL` | `string` | `models.embedding_url` | `"http://localhost:11434/v1"` | ❌ | Primary embedding endpoint URL |
| `XAVIER2_EMBEDDING_ENDPOINT` | `string` | — | (same as EMBEDDING_URL) | ❌ | Alternative embedding endpoint (takes precedence if set) |
| `XAVIER2_EMBEDDING_MODEL` | `string` | `models.embedding_model` | `"embeddinggemma"` | ❌ | Embedding model name |
| `XAVIER2_EMBEDDING_API_KEY` | `string` | — | (none) | ❌ | API key for embedding provider |
| `XAVIER2_EMBEDDING_API_FLAVOR` | `string` | — | `"openai-compatible"` | ❌ | API flavor: `openai-compatible`, `anthropic-compatible` |
| `XAVIER2_EMBEDDER` | `string` | — | — | ❌ | Legacy embedder selection |

**Embedding defaults (constants in `src/embedding/mod.rs`)**:

| Mode | Endpoint Default | Model Default |
|------|-----------------|---------------|
| Local | `http://localhost:11434/v1/embeddings` | `embeddinggemma` |
| Cloud | `https://api.openai.com/v1/embeddings` | `text-embedding-3-small` |

**Source**: `src/embedding/mod.rs`, `src/embedding/openai.rs`

---

## 5. LLM Provider

| Variable | Type | Config JSON | Default | Required | Description |
|----------|------|-------------|---------|----------|-------------|
| `XAVIER2_MODEL_PROVIDER` | `string` | `models.provider` | `"local"` | ❌ | LLM provider: `local`, `cloud`, `disabled`, `openai`, `anthropic`, `deepseek`, `minimax`, `gemini` |
| `XAVIER2_API_FLAVOR` | `string` | `models.api_flavor` | `"openai-compatible"` | ❌ | API protocol: `openai-compatible`, `anthropic-compatible` |
| `XAVIER2_LOCAL_LLM_URL` | `string` | `models.local_llm_url` | `"http://localhost:11434/v1"` | ❌ | Local LLM endpoint (Ollama, etc.) |
| `XAVIER2_LOCAL_LLM_MODEL` | `string` | `models.local_llm_model` | `"qwen3-coder"` | ❌ | Local LLM model name |
| `XAVIER2_LOCAL_LLM_API_KEY` | `string` | — | `"ollama"` | ❌ | Local LLM API key (defaults to "ollama") |
| `XAVIER2_LOCAL_ANTHROPIC_URL` | `string` | — | `"http://localhost:11434"` | ❌ | Local Anthropic-compatible endpoint |
| `XAVIER2_CLOUD_LLM_MODEL` | `string` | — | `"gpt-4o-mini"` | ❌ | Cloud LLM model (generic mode) |
| `XAVIER2_CLOUD_LLM_URL` | `string` | — | `"https://api.openai.com/v1"` | ❌ | Cloud LLM URL (generic mode) |
| `XAVIER2_LLM_MODEL` | `string` | — | (provider-dependent) | ❌ | Fallback model name for all providers |
| `XAVIER2_LLM_API_KEY` | `string` | — | (none) | ❌ | Generic LLM API key (fallback for all providers) |

**Source**: `src/agents/provider.rs`, `src/settings.rs`

---

## 6. Provider-Specific Credentials

These are used when `XAVIER2_MODEL_PROVIDER` is set to a specific cloud provider.

| Variable | Provider | Default Model | Default Base URL | Required |
|----------|----------|---------------|-----------------|----------|
| `OPENAI_API_KEY` | OpenAI | `gpt-4o-mini` | `https://api.openai.com/v1` | ✅ |
| `OPENAI_MODEL` | OpenAI | — | — | ❌ |
| `OPENAI_BASE_URL` | OpenAI | `https://api.openai.com/v1` | — | ❌ |
| `ANTHROPIC_API_KEY` | Anthropic | `claude-3-5-sonnet-latest` | `https://api.anthropic.com/v1` | ✅ |
| `ANTHROPIC_MODEL` | Anthropic | — | — | ❌ |
| `ANTHROPIC_BASE_URL` | Anthropic | `https://api.anthropic.com/v1` | — | ❌ |
| `DEEPSEEK_API_KEY` | DeepSeek | `deepseek-chat` | `https://api.deepseek.com` | ✅ |
| `DEEPSEEK_MODEL` | DeepSeek | — | — | ❌ |
| `DEEPSEEK_BASE_URL` | DeepSeek | `https://api.deepseek.com` | — | ❌ |
| `MINIMAX_API_KEY` | MiniMax | `MiniMax-Text-01` | `https://api.minimax.chat/v1` | ✅ |
| `MINIMAX_MODEL` | MiniMax | — | — | ❌ |
| `MINIMAX_BASE_URL` | MiniMax | `https://api.minimax.chat/v1` | — | ❌ |
| `GEMINI_API_KEY` | Gemini | `gemini-2.0-flash` | (no base URL override) | ✅ |
| `GEMINI_MODEL` | Gemini | — | — | ❌ |

**Fallback chain**: Provider-specific model → `XAVIER2_LLM_MODEL` → hardcoded default  
**API key fallback**: `ANTHROPIC_API_KEY` → `XAVIER2_LLM_API_KEY` (Anthropic only)  
**URL fallback**: `ANTHROPIC_BASE_URL` → `XAVIER2_CLOUD_LLM_URL` → default (Anthropic only)

**Source**: `src/agents/provider.rs`

---

## 7. Routing

| Variable | Type | Config JSON | Default | Required | Description |
|----------|------|-------------|---------|----------|-------------|
| `XAVIER2_ROUTER_POLICY_PATH` | `string` | — | (none) | ❌ | Path to JSON routing policy file |
| `XAVIER2_ROUTER_POLICY_REFRESH_SECS` | `u64` | — | `30` | ❌ | Policy file refresh interval |
| `XAVIER2_ROUTER_RETRIEVED_MODEL` | `string` | `models.router_retrieved_model` | `""` | ❌ | Model for retrieved-augmented queries |
| `XAVIER2_ROUTER_COMPLEX_MODEL` | `string` | `models.router_complex_model` | `""` | ❌ | Model for complex reasoning queries |
| `XAVIER2_ROUTER_FAST_MODEL` | `string` | — | (none) | ❌ | Model for fast/direct queries (router override) |
| `XAVIER2_ROUTER_QUALITY_MODEL` | `string` | — | (none) | ❌ | Model for quality queries (router override) |

**Routing policy defaults** (in `src/agents/router.rs`):
- `policy.version` = 1
- `thresholds.strong_retrieval_confidence` = 0.72
- `thresholds.weak_reasoning_confidence` = 0.68
- All models default to `enabled: true`

**Source**: `src/agents/router.rs`, `src/settings.rs`

---

## 8. Memory Layers

### Working Memory

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `XAVIER2_WORKING_MEMORY_CAPACITY` | `usize` | `100` | Maximum items in working memory |
| `XAVIER2_WORKING_LRU_THRESHOLD` | `usize` | `2` | Access count threshold for LRU exemption |
| `XAVIER2_WORKING_BM25_K1` | `f32` | `1.5` | BM25 scoring parameter k1 |
| `XAVIER2_WORKING_BM25_B` | `f32` | `0.75` | BM25 scoring parameter b |

### Episodic Memory

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `XAVIER2_EPISODIC_SUMMARY_WINDOW` | `usize` | `10` | Turns before triggering episodic summary |
| `XAVIER2_MAX_EPISODIC_SESSIONS` | `usize` | `50` | Maximum sessions retained in episodic memory |
| `XAVIER2_EPISODIC_MIN_EVENT_IMPORTANCE` | `f32` | `0.5` | Minimum importance score for key events |

**Source**: `src/memory/layers_config.rs`, `src/memory/working.rs`

---

## 9. Context Budget

Controls how many documents/tokens are included at each context regeneration hook.

### SessionStart Hook

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `XAVIER2_CTX_SS_MIN_DOCS` | `usize` | `3` | Minimal docs at session start |
| `XAVIER2_CTX_SS_MIN_TOKENS` | `usize` | `600` | Minimal tokens at session start |
| `XAVIER2_CTX_SS_MED_DOCS` | `usize` | `5` | Medium docs at session start |
| `XAVIER2_CTX_SS_MED_TOKENS` | `usize` | `1200` | Medium tokens at session start |
| `XAVIER2_CTX_SS_MAX_DOCS` | `usize` | `8` | Maximum docs at session start |
| `XAVIER2_CTX_SS_MAX_TOKENS` | `usize` | `2400` | Maximum tokens at session start |

### Precompact Hook

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `XAVIER2_CTX_PC_MIN_DOCS` | `usize` | `4` | Minimal docs for precompact |
| `XAVIER2_CTX_PC_MIN_TOKENS` | `usize` | `800` | Minimal tokens for precompact |
| `XAVIER2_CTX_PC_MED_DOCS` | `usize` | `7` | Medium docs for precompact |
| `XAVIER2_CTX_PC_MED_TOKENS` | `usize` | `1600` | Medium tokens for precompact |
| `XAVIER2_CTX_PC_MAX_DOCS` | `usize` | `10` | Maximum docs for precompact |
| `XAVIER2_CTX_PC_MAX_TOKENS` | `usize` | `3200` | Maximum tokens for precompact |

**Source**: `src/context/orchestrator.rs`

---

## 10. Synchronization

| Variable | Type | Config JSON | Default | Legacy Alias | Description |
|----------|------|-------------|---------|-------------|-------------|
| `XAVIER2_SYNC_INTERVAL_MS` | `u64` | `sync.interval_ms` | `300000` | `SEVIER2_SYNC_INTERVAL_MS` | Interval between sync cycles (ms) |
| `XAVIER2_SYNC_LAG_THRESHOLD_MS` | `u64` | `sync.lag_threshold_ms` | `30000` | `SEVIER2_LAG_THRESHOLD_MS` | Maximum allowed write lag (ms) |
| `XAVIER2_SYNC_SAVE_OK_RATE_THRESHOLD` | `f32` | `sync.save_ok_rate_threshold` | `0.95` | `SEVIER2_SAVE_OK_RATE_THRESHOLD` | Minimum save success rate |
| `XAVIER2_SYNC_MAX_RETRIES` | `u32` | `sync.max_retries` | `3` | `SEVIER2_SYNC_MAX_RETRIES` | Maximum health check retries |
| `XAVIER2_SYNC_RETRY_DELAY_MS` | `u64` | `sync.retry_delay_ms` | `1000` | (none) | Delay between retries (ms) |
| `XAVIER2_SYNC_MIN_HEALTH_INTERVAL_MS` | `u64` | — | `1000` | `SEVIER2_SYNC_MIN_HEALTH_INTERVAL_MS` | Min interval between health checks |
| `XAVIER2_SYNC_TIMEOUT_MS` | `u64` | — | `5000` | `SEVIER2_SYNC_TIMEOUT_MS` | Timeout per health check attempt |

**Legacy note**: The `SEVIER2_*` names are read as fallbacks when the `XAVIER2_*` version is not set. These are for backward compatibility.

**Source**: `src/tasks/session_sync_task.rs`, `src/settings.rs`

---

## 11. Vector Store Features

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `XAVIER2_QJL_THRESHOLD` | `usize` | `30000` | Vector count threshold for QJL quantization (must be > 0) |
| `XAVIER2_RRF_K` | `usize` | `60` | Reciprocal Rank Fusion k value (must be > 0) |
| `XAVIER2_ENTITY_EXTRACTION_ENABLED` | `bool` | `true` | Enable entity extraction in vector store |
| `XAVIER2_AUDIT_CHAIN_ENABLED` | `bool` | `true` | Enable audit chain validation |

**Note**: RRF k is dynamic — when dataset > 1000, the value scales: `RRF_K = base + (dataset_size / 1000)`

**Source**: `src/memory/sqlite_vec_store.rs`

---

## 12. Retrieval

| Variable | Type | Config JSON | Default | Description |
|----------|------|-------------|---------|-------------|
| `XAVIER2_DISABLE_HYDE` | `bool` | `retrieval.disable_hyde` | `true` | Disable HyDE (Hypothetical Document Embedding) query expansion |

**Source**: `src/settings.rs`, `src/retrieval/`

---

## 13. Telegram Bot

| Variable | Type | Default | Required | Description |
|----------|------|---------|----------|-------------|
| `XAVIER2_TELEGRAM_ENABLED` | `bool` | `false` | ❌ | Enable Telegram bot integration |
| `XAVIER2_TELEGRAM_TOKEN` | `string` | `""` | ✅ | Telegram Bot API token |

**Behavior**: If `enabled` is `true` but `token` is empty/unset, the bot logs an error and does not start.

**Source**: `src/telegram/mod.rs`

---

## 14. Billing — Stripe

| Variable | Type | Default | Required | Description |
|----------|------|---------|----------|-------------|
| `STRIPE_SECRET_KEY` | `string` | — | ✅ | Stripe secret API key |
| `STRIPE_WEBHOOK_SECRET` | `string` | — | ✅ | Stripe webhook signing secret |
| `STRIPE_PRICE_PRO` | `string` | — | ❌ | Stripe price ID for Pro plan |
| `STRIPE_PRICE_CLOUD` | `string` | — | ❌ | Stripe price ID for Cloud plan |
| `STRIPE_PRICE_ENTERPRISE` | `string` | — | ❌ | Stripe price ID for Enterprise plan |
| `STRIPE_CANCEL_URL` | `string` | — | ❌ | Cancel URL for Stripe Checkout |
| `STRIPE_SUCCESS_URL` | `string` | — | ❌ | Success URL for Stripe Checkout |

**Source**: `src/billing/mod.rs`, `src/billing/stripe_client.rs`, `src/billing/plans.rs`

---

## 15. Project Management — Planka

| Variable | Type | Default | Required | Description |
|----------|------|---------|----------|-------------|
| `PLANKA_URL` | `string` | — | ✅ | Planka server URL |
| `PLANKA_EMAIL` | `string` | — | ✅ | Planka login email |
| `PLANKA_PASSWORD` | `string` | — | ✅ | Planka login password |

All three must be set for the Planka integration to activate. Used for automatic task creation/move operations.

**Source**: `src/tools/kanban.rs`

---

## 16. CLI / Legacy

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `XAVIER2_API_URL` | `string` | (falls back to `XAVIER2_URL`) | CLI client URL (legacy) |
| `XAVIER2_AUTH_TOKEN` | `string` | (falls back to `XAVIER2_TOKEN`) | CLI client auth token (legacy) |

**Source**: `src/cli/auth.rs`, `src/cli/utils.rs`

---

## 17. Rust Logging

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `RUST_LOG` | `string` | (none) | Standard Rust logging filter; captured in CLI verbosity struct |

**Source**: `src/cli.rs`

---

## Undocumented Env Vars

All env vars found in the codebase are documented above. There are **no undocumented `XAVIER2_*` env vars** at this time.

---

## Env Var Count Summary

| Category | Count |
|----------|-------|
| Server & Network | 10 |
| Workspace & Billing | 9 |
| Memory Backend | 7 |
| Embedding | 6 |
| LLM Provider | 10 |
| Provider Credentials | 15 |
| Routing | 6 |
| Memory Layers | 7 |
| Context Budget | 12 |
| Synchronization | 7 |
| Vector Store Features | 4 |
| Retrieval | 1 |
| Telegram Bot | 2 |
| Stripe Billing | 7 |
| Planka | 3 |
| CLI/Legacy | 2 |
| Rust Logging | 1 |
| **Total** | **109** |

---

## Quick Start: Minimal Configuration

For a development setup, you only need:

```bash
# Dev mode — generates a random token, bypasses auth
export XAVIER2_DEV_MODE=true

# Or explicit token (only if not using dev mode)
export XAVIER2_TOKEN="your-secure-token"

# Optional: customize host/port
export XAVIER2_HOST="0.0.0.0"
export XAVIER2_PORT="8006"
```

For production with a cloud LLM:

```bash
export XAVIER2_TOKEN="your-secure-token"
export XAVIER2_TOKEN_SECRET="your-hmac-secret"
export XAVIER2_MODEL_PROVIDER="openai"
export OPENAI_API_KEY="sk-..."
```

For production with local LLM (Ollama):

```bash
export XAVIER2_TOKEN="your-secure-token"
export XAVIER2_MODEL_PROVIDER="local"
export XAVIER2_LOCAL_LLM_URL="http://localhost:11434/v1"
export XAVIER2_LOCAL_LLM_MODEL="llama3"
```
