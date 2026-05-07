# Xavier2 Configuration Reference

> Auto-generated from `src/settings.rs` and `config/xavier2.config.json`  
> Last updated: 2026-05-06

## Overview

Xavier2 uses a layered configuration system in this priority order (highest first):

1. **Environment variables** — `XAVIER2_*` env vars
2. **Config file** — `config/xavier2.config.json` (overridable via `XAVIER2_CONFIG_PATH`)
3. **Hardcoded defaults** — `Default` trait impls in `src/settings.rs`

The config file is loaded at startup via `Xavier2Settings::load()`. Values from the config file are then applied as env var defaults (via `set_if_absent`), meaning **explicit env vars always override config file values**.

---

## Config File Schema

Path: `config/xavier2.config.json` (default, overridable via `XAVIER2_CONFIG_PATH`)

### Top-Level Structure

```json
{
  "server": { /* ServerSettings */ },
  "workspace": { /* WorkspaceSettings */ },
  "memory": { /* MemorySettings */ },
  "models": { /* ModelSettings */ },
  "retrieval": { /* RetrievalSettings */ },
  "sync": { /* SyncSettings */ }
}
```

All top-level keys are optional and default to their `Default` impl when absent.

---

## Section: `server` — ServerSettings

Controls the HTTP server binding and logging.

| JSON Key | Type | Default | Config File Override | Env Var | Description |
|----------|------|---------|---------------------|---------|-------------|
| `host` | `string` | `"127.0.0.1"` | `"0.0.0.0"` | `XAVIER2_HOST` | IP address the HTTP server binds to |
| `port` | `integer` | `8006` | `8006` | `XAVIER2_PORT` | HTTP server port |
| `log_level` | `string` | `"info"` | `"info"` | `XAVIER2_LOG_LEVEL` | Log verbosity: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"` |
| `code_graph_db_path` | `string` | `"data/code_graph.db"` | `"data/code_graph.db"` | `XAVIER2_CODE_GRAPH_DB_PATH` | Path to the code graph SQLite database |

---

## Section: `workspace` — WorkspaceSettings

Controls workspace identity, billing plan, and embedding mode.

| JSON Key | Type | Default | Config File Override | Env Var | Description |
|----------|------|---------|---------------------|---------|-------------|
| `default_workspace_id` | `string` | `"default"` | `"default"` | `XAVIER2_DEFAULT_WORKSPACE_ID` | Default workspace ID for new sessions |
| `default_plan` | `string` | `"community"` | `"community"` | `XAVIER2_DEFAULT_PLAN` | Billing plan: `"community"`, `"pro"`, `"cloud"`, `"enterprise"` |
| `storage_limit_bytes` | `uint64?` | `null` | `null` | `XAVIER2_STORAGE_LIMIT_BYTES` | Optional storage cap in bytes |
| `request_limit` | `uint?` | `null` | `null` | `XAVIER2_REQUEST_LIMIT` | Optional request count cap |
| `request_unit_limit` | `uint64?` | `null` | `null` | `XAVIER2_REQUEST_UNIT_LIMIT` | Optional request unit cap |
| `embedding_provider_mode` | `string` | `"bring_your_own"` | `"bring_your_own"` | `XAVIER2_EMBEDDING_PROVIDER_MODE` | Embedding provider: `"local"`, `"cloud"`, `"bring_your_own"`, `"disabled"` |
| `managed_google_embeddings` | `bool` | `false` | `false` | `XAVIER2_MANAGED_GOOGLE_EMBEDDINGS` | Enable managed Google embedding service |
| `sync_policy` | `string` | `"local_only"` | `"local_only"` | `XAVIER2_SYNC_POLICY` | Memory sync policy |

---

## Section: `memory` — MemorySettings

Controls the storage backend and data paths.

| JSON Key | Type | Default | Config File Override | Env Var | Description |
|----------|------|---------|---------------------|---------|-------------|
| `backend` | `string` | `"vec"` | `"vec"` | `XAVIER2_MEMORY_BACKEND` | Storage backend: `"vec"`, `"sqlite"`, `"file"` |
| `data_dir` | `string` | `"data"` | `"data"` | `XAVIER2_DATA_DIR` | Base data directory |
| `embedding_dimensions` | `uint` | `768` | `768` | `XAVIER2_EMBEDDING_DIMENSIONS` | Dimensionality of embedding vectors |
| `workspace_dir` | `string` | `"data/workspaces"` | `"data/workspaces"` | `XAVIER2_WORKSPACE_DIR` | Directory for workspace data |
| `file_path` | `string` | `"data/workspaces/default/memory-store.json"` | _default_ | `XAVIER2_MEMORY_FILE_PATH` | JSON file store path (file backend) |
| `sqlite_path` | `string` | `"data/memory-store.sqlite3"` | _default_ | `XAVIER2_MEMORY_SQLITE_PATH` | SQLite store path (sqlite backend) |
| `vec_path` | `string` | `"data/vec-store.sqlite3"` | _default_ | `XAVIER2_MEMORY_VEC_PATH` | Vector SQLite store path (vec backend) |

---

## Section: `models` — ModelSettings

Controls LLM and embedding model configuration.

| JSON Key | Type | Default | Config File Override | Env Var | Description |
|----------|------|---------|---------------------|---------|-------------|
| `provider` | `string` | `"local"` | `"local"` | `XAVIER2_MODEL_PROVIDER` | LLM provider: `"local"`, `"cloud"`, `"disabled"`, or explicit `"openai"`/`"anthropic"`/`"deepseek"`/`"minimax"`/`"gemini"` |
| `api_flavor` | `string` | `"openai-compatible"` | `"openai-compatible"` | `XAVIER2_API_FLAVOR` | API protocol: `"openai-compatible"` or `"anthropic-compatible"` |
| `local_llm_url` | `string` | `"http://localhost:11434/v1"` | _default_ | `XAVIER2_LOCAL_LLM_URL` | Local LLM endpoint URL |
| `local_llm_model` | `string` | `"qwen3-coder"` | _default_ | `XAVIER2_LOCAL_LLM_MODEL` | Local LLM model name |
| `embedding_url` | `string` | `"http://localhost:11434/v1"` | _default_ | `XAVIER2_EMBEDDING_URL` | Embedding endpoint URL |
| `embedding_model` | `string` | `"embeddinggemma"` | _default_ | `XAVIER2_EMBEDDING_MODEL` | Embedding model name |
| `router_retrieved_model` | `string` | `""` | _default_ | `XAVIER2_ROUTER_RETRIEVED_MODEL` | Model override for retrieved-augmented queries |
| `router_complex_model` | `string` | `""` | _default_ | `XAVIER2_ROUTER_COMPLEX_MODEL` | Model override for complex reasoning queries |

---

## Section: `retrieval` — RetrievalSettings

Controls retrieval behavior.

| JSON Key | Type | Default | Config File Override | Env Var | Description |
|----------|------|---------|---------------------|---------|-------------|
| `disable_hyde` | `bool` | `true` | `true` | `XAVIER2_DISABLE_HYDE` | Disable HyDE (Hypothetical Document Embedding) expansion |

---

## Section: `sync` — SyncSettings

Controls session synchronization and health checking.

| JSON Key | Type | Default | Config File Override | Env Var | Description |
|----------|------|---------|---------------------|---------|-------------|
| `interval_ms` | `uint64` | `300000` | `300000` | `XAVIER2_SYNC_INTERVAL_MS` | Interval between sync cycles (ms) |
| `lag_threshold_ms` | `uint64` | `30000` | _default_ | `XAVIER2_SYNC_LAG_THRESHOLD_MS` | Maximum allowed write lag (ms) |
| `save_ok_rate_threshold` | `float32` | `0.95` | _default_ | `XAVIER2_SYNC_SAVE_OK_RATE_THRESHOLD` | Minimum save success rate required |
| `max_retries` | `uint32` | `3` | _default_ | `XAVIER2_SYNC_MAX_RETRIES` | Maximum health check retries |
| `retry_delay_ms` | `uint64` | `1000` | _default_ | `XAVIER2_SYNC_RETRY_DELAY_MS` | Delay between retries (ms) |

---

## Config File Current Values

From `config/xavier2.config.json` (as of 2026-05-06):

```json
{
  "server": {
    "host": "0.0.0.0"
  },
  "memory": {
    "backend": "vec"
  },
  "models": {
    "provider": "local"
  },
  "retrieval": {
    "disable_hyde": true
  },
  "sync": {
    "interval_ms": 300000
  }
}
```

Notable overrides from defaults:
- `server.host`: `"127.0.0.1"` → `"0.0.0.0"` (bind to all interfaces)
- All other values currently match defaults

---

## How Config Flows to Env Vars

At startup, `Xavier2Settings::apply_to_env()` sets env vars for ALL config keys — but only if the env var is not already set (`set_if_absent`). This means:

1. A **config file** value sets the base configuration
2. An **env var** set before startup will override the config file
3. If neither exists, the **hardcoded default** applies

### Config JSON → Env Var Map (complete)

This is the full mapping applied by `apply_to_env()`:

| Config JSON Path | Env Var |
|------------------|---------|
| `server.host` | `XAVIER2_HOST` |
| `server.port` | `XAVIER2_PORT` |
| `server.log_level` | `XAVIER2_LOG_LEVEL` |
| `server.code_graph_db_path` | `XAVIER2_CODE_GRAPH_DB_PATH` |
| `workspace.default_workspace_id` | `XAVIER2_DEFAULT_WORKSPACE_ID` |
| `workspace.default_plan` | `XAVIER2_DEFAULT_PLAN` |
| `workspace.storage_limit_bytes` | `XAVIER2_STORAGE_LIMIT_BYTES` |
| `workspace.request_limit` | `XAVIER2_REQUEST_LIMIT` |
| `workspace.request_unit_limit` | `XAVIER2_REQUEST_UNIT_LIMIT` |
| `workspace.embedding_provider_mode` | `XAVIER2_EMBEDDING_PROVIDER_MODE` |
| `workspace.managed_google_embeddings` | `XAVIER2_MANAGED_GOOGLE_EMBEDDINGS` |
| `workspace.sync_policy` | `XAVIER2_SYNC_POLICY` |
| `memory.backend` | `XAVIER2_MEMORY_BACKEND` |
| `memory.data_dir` | `XAVIER2_DATA_DIR` |
| `memory.embedding_dimensions` | `XAVIER2_EMBEDDING_DIMENSIONS` |
| `memory.workspace_dir` | `XAVIER2_WORKSPACE_DIR` |
| `memory.file_path` | `XAVIER2_MEMORY_FILE_PATH` |
| `memory.sqlite_path` | `XAVIER2_MEMORY_SQLITE_PATH` |
| `memory.vec_path` | `XAVIER2_MEMORY_VEC_PATH` |
| `models.provider` | `XAVIER2_MODEL_PROVIDER` |
| `models.api_flavor` | `XAVIER2_API_FLAVOR` |
| `models.local_llm_url` | `XAVIER2_LOCAL_LLM_URL` |
| `models.local_llm_model` | `XAVIER2_LOCAL_LLM_MODEL` |
| `models.embedding_url` | `XAVIER2_EMBEDDING_URL` |
| `models.embedding_model` | `XAVIER2_EMBEDDING_MODEL` |
| `models.router_retrieved_model` | `XAVIER2_ROUTER_RETRIEVED_MODEL` |
| `models.router_complex_model` | `XAVIER2_ROUTER_COMPLEX_MODEL` |
| `retrieval.disable_hyde` | `XAVIER2_DISABLE_HYDE` |
| `sync.interval_ms` | `XAVIER2_SYNC_INTERVAL_MS` |
| `sync.lag_threshold_ms` | `XAVIER2_SYNC_LAG_THRESHOLD_MS` |
| `sync.save_ok_rate_threshold` | `XAVIER2_SYNC_SAVE_OK_RATE_THRESHOLD` |
| `sync.max_retries` | `XAVIER2_SYNC_MAX_RETRIES` |
| `sync.retry_delay_ms` | `XAVIER2_SYNC_RETRY_DELAY_MS` |

---

## Inconsistencies Noted

These are documented but NOT fixed (by task constraints):

1. **`XAVIER2_MANAGED_GOOGLE_EMBEDDINGS` type**: Config file stores `bool`, but `apply_to_env()` writes it as `"1"`/`"0"` string. Reading code checks for `"true"`-like values.

2. **`XAVIER2_SYNC_MIN_HEALTH_INTERVAL_MS` / `XAVIER2_SYNC_TIMEOUT_MS`**: These exist in `src/tasks/session_sync_task.rs` and have legacy `SEVIER2_*` fallback names, but are NOT in `Settings` struct — they're read directly from env without a config file mapping.

3. **`XAVIER2_EMBEDDING_ENDPOINT`**: Used as a distinct alternative to `XAVIER2_EMBEDDING_URL` in `src/embedding/mod.rs`. Both check the same thing but are separate env vars.

4. **Legacy `SEVIER2_*` vars**: Multiple sync config values have legacy `SEVIER2_*` fallback names hardcoded in `session_sync_task.rs` via `read_env_or_legacy()`. These are not configurable via the JSON config file.

5. **`XAVIER2_AUTH_TOKEN` / `XAVIER2_API_URL`**: These are used as CLI fallback vars but have no corresponding config file entry.
