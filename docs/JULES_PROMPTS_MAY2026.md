# Jules Prompts — Mayo 2026

_Organized by phase. Each prompt is self-contained with full context._

---

## Phase 1: SEVIER Core Stability

### 🔴 Prompt 1: Fix Docker Missing Env Vars (#78)

**Title:** [sevier][P1] Docker: Missing critical env vars for SEVIER endpoints

**Context:** Sevier endpoints in Docker deployment are missing critical environment variables. The Docker container needs `SEVIER_BASE_URL`, `SEVIER_API_KEY`, `SEVIER_SESSION_ID`, and potentially `XAVIER_BASE_URL` to connect to the SEVIER service.

**Task:**
1. Read `src/adapters/inbound/http/routes.rs` — find where Sevier handlers are registered
2. Read `src/agents/` — find where env vars are currently read for Sevier config
3. Check `docker-compose.yml` or `Dockerfile` for current env var definitions
4. Add missing env vars to Docker configuration:
   - `SEVIER_BASE_URL` (e.g., `http://sevier:8080`)
   - `SEVIER_API_KEY`
   - `SEVIER_SESSION_ID`
5. Update `src/agents/` or `src/app/` to read these env vars and pass to Sevier handlers
6. Verify `sync_check_handler` and `session_event_handler` can read these values

**Acceptance:**
- Docker compose shows SEVIER env vars in service definition
- Sevier handlers can access `SEVIER_BASE_URL` and `SEVIER_API_KEY`
- No hardcoded fallback URLs in the codebase

**Reference:** Related to #77, #80 which established SessionSyncTask configuration.

---

### 🔴 Prompt 2: Add /agents/{id}/unregister Endpoint (#75)

**Title:** [sevier][P1] Missing /xavier/agents/{id}/unregister endpoint

**Context:** There's a register endpoint (`/xavier/agents/register`) but no corresponding unregister endpoint. When agents need to leave, they can't notify the system.

**Task:**
1. Read `src/adapters/inbound/http/routes.rs` — find register endpoint pattern
2. Read `src/agents/unregister_agent_handler.rs` — this handler exists but may not be wired
3. Check `src/cli.rs` — see how router is set up
4. Add route `DELETE /xavier/agents/{id}/unregister` to routes.rs
5. Wire `unregister_agent_handler` to the router in cli.rs
6. The handler should:
   - Accept agent ID from path
   - Call `agent_registry.unregister(id)`
   - Return 200 on success, 404 if agent not found

**Acceptance:**
- `DELETE /xavier/agents/{id}/unregister` returns 200 for registered agents
- Returns 404 for unknown agents
- Agent is removed from registry

---

### 🔴 Prompt 3: Fix /agents/register Wrong Payload (#74)

**Title:** [sevier][P1] Test script sends wrong payload for /xavier/agents/register

**Context:** The test script sends an incorrect JSON payload to `/xavier/agents/register`. The endpoint expects certain fields but the test sends wrong ones.

**Task:**
1. Read `src/adapters/inbound/http/routes.rs` — find the register handler
2. Read `src/ports/inbound/agent_lifecycle_port.rs` — understand expected fields
3. Read `src/domain/agent.rs` — see Agent struct definition
4. Find and read the test script that sends wrong payload
5. Fix the payload to match what the handler expects:
   - Should include: `agent_id`, `name`, `capabilities`, `endpoint`
   - Should match `Agent` domain struct fields
6. Verify the handler parses payload correctly

**Acceptance:**
- Test script sends correct payload matching `Agent` struct
- Handler successfully registers agent with correct data
- No field mismatches or parsing errors

---

## Phase 2: Architecture & Cleanup

### 🟡 Prompt 4: SessionSyncTask Graceful Shutdown (#76)

**Title:** [sevier][P2] SessionSyncTask cron has no graceful shutdown mechanism

**Context:** The SessionSyncTask runs on a cron schedule but has no mechanism to shut down gracefully. When the application stops, the cron task may be interrupted mid-sync, causing inconsistent state.

**Task:**
1. Read `src/app/session_sync_task.rs` — find the cron/interval loop
2. Read `src/app/mod.rs` or where SessionSyncTask is instantiated
3. Add a shutdown signal mechanism:
   - Add `shutdown_tx: oneshot::Sender<()>` to SessionSyncTask
   - Create `shutdown()` method that sends signal
   - Modify cron loop to check `shutdown_rx` each iteration
   - When signal received, complete current sync then exit
4. Add `impl Drop for SessionSyncTask` to trigger shutdown on drop
5. Store `shutdown_tx` in AppState or pass to where SessionSyncTask is created

**Acceptance:**
- `SessionSyncTask` has `shutdown()` method
- Cron loop exits cleanly when shutdown is called
- No dangling tasks after AppState drop

---

### 🟡 Prompt 5: Duplicate session_event_handler (#84)

**Title:** [sevier][P3] Duplicate session_event_handler definitions in routes.rs and cli.rs

**Context:** `session_event_handler` is defined in both `routes.rs` and `cli.rs` — this is code duplication and can cause confusion about which one is actually used.

**Task:**
1. Read `src/adapters/inbound/http/routes.rs` — find session_event_handler
2. Read `src/cli.rs` — find the duplicate
3. Determine which one is actually wired to the router
4. Remove the unused duplicate
5. If both are used in different contexts (HTTP vs CLI), rename one to clarify purpose:
   - `session_event_handler_http` for routes.rs
   - `session_event_handler_cli` for cli.rs
6. Ensure no other handlers have similar duplication

**Acceptance:**
- Only one `session_event_handler` exists (or two with clear naming)
- Both HTTP and CLI paths compile correctly
- No duplicate handler definitions

---

### 🟡 Prompt 6: TimeMetrics Global OnceLock Cleanup (#93)

**Title:** [arch] TimeMetricsStore OnceLock replaced with proper port

**Context:** `TimeMetricsStore` was previously accessed via module-level `OnceLock<TimeMetricsStore>`. It should now use the proper inbound port `TimeMetricsPort` through AppState.

**Task:**
1. Read `src/ports/inbound/time_metrics_port.rs` — understand the port trait
2. Read `src/ports/inbound/mod.rs` — see if TimeMetricsPort is exported
3. Search for `OnceLock` usage related to TimeMetrics
4. Remove module-level `OnceLock<TimeMetricsStore>`
5. Ensure TimeMetricsStore is instantiated in AppState and accessed via port
6. Update any direct `TimeMetricsStore::get()` calls to use the port instead

**Acceptance:**
- No `OnceLock<TimeMetricsStore>` remaining in codebase
- TimeMetrics accessed through `Arc<dyn TimeMetricsPort>`
- All tests pass

---

## Phase 3: Future Preparation

### 🟢 Prompt 7: qmd_memory.rs Modularization (#164)

**Title:** [PERF] qmd_memory.rs modularization

**New issue:** #164 (replaces failed #137)

**Context:** `src/app/qmd_memory.rs` is 105KB, making it hard to navigate and maintain. It should be split into logical modules.

**Task:**
1. Read `src/app/qmd_memory.rs` — identify logical sections:
   - Search functionality (BM25, semantic)
   - Add/update operations
   - Index management
   - Cache management
   - Serialization
2. Create `src/app/qmd_memory/` directory with modules:
   - `mod.rs` — re-exports everything
   - `search.rs` — search operations
   - `storage.rs` — add/update operations
   - `index.rs` — index management
   - `cache.rs` — cache operations
3. Keep `qmd_memory.rs` as a thin re-export hub
4. Update `src/lib.rs` to include new modules
5. Run `cargo check --lib` to verify compilation

**Acceptance:**
- `src/app/qmd_memory.rs` is < 200 lines (mostly re-exports)
- New modules in `src/app/qmd_memory/` are logically separated
- All cargo tests pass

---

### 🟢 Prompt 8: Magic Constants Extraction (Follow-up to #127)

**Title:** Continue extracting hardcoded constants across codebase

**Context:** Issue #127 extracted constants from `gating.rs`, but other files likely have similar magic constants that should be centralized.

**Task:**
1. Search for magic constants across codebase:
   - `const.*=.*1000` patterns (buffer sizes, thresholds)
   - `const.*=.*3600` patterns (timeouts in seconds)
   - `const.*=.*8080` or similar port numbers
2. Create `src/config/mod.rs` or extend existing config module
3. Group constants by domain:
   - `RETRIEVAL_*` for retrieval (already done)
   - `SYNC_*` for sync-related
   - `HTTP_*` for HTTP server settings
   - `AGENT_*` for agent settings
4. Replace all magic constants with imports from config
5. Add config loading from env vars where appropriate (like #152 did for Sevier)

**Acceptance:**
- All magic constants replaced with named constants
- Constants organized by domain in config module
- No `const _: usize = <number>` without explanation in source files

---

## Notes for Jules

- Always run `cargo check --lib` after making changes
- Use `git diff HEAD --stat` to verify changes before committing
- For Docker changes: check both `Dockerfile` and `docker-compose.yml`
- For architecture changes: verify no `Arc<RwLock<...>>` direct usage remains (should use ports)
