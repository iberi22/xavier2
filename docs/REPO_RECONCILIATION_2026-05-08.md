# Xavier Repository Reconciliation — 2026-05-08

Canonical repository: `E:\scripts-python\xavier` (`origin=https://github.com/iberi22/xavier.git`, branch `main`, HEAD `926fb05`).

Purpose: identify duplicate/temporary Xavier checkouts under `E:\scripts-python`, decide what to migrate, archive, or delete, and avoid losing dispersed logic.

## Current canonical working tree

- Modified: `.gitignore` — added `state/auth-profiles.json` to avoid tracking local auth state.
- Untracked local session summaries: `memory/2026-05-06-0440.md`, `0442.md`, `0443.md`, `0458.md`.
- `state/models.json` was restored to tracked state after detecting a local provider key risk.

## Xavier-family checkouts

| Path | Branch | Dirty | Status | Decision |
|---|---:|---:|---|---|
| `E:\scripts-python\xavier` | `main` | 5 | Newest canonical repo. Contains post-`0222ed8` release/security/docs/plugin work through `926fb05`. | **KEEP / CANONICAL** |
| `E:\scripts-python\xavier-new` | `master` | 0 | Clean old clone at `0222ed8`. Its content is ancestor of canonical `xavier`. | **ARCHIVE then DELETE** |
| `E:\scripts-python\xavier-temp` | `master` | 0 | Clean temporary old clone at `0222ed8`, shallow/no parent metadata. | **ARCHIVE then DELETE** |
| `E:\scripts-python\xavier_temp_reclone` | `master` | 0 | Clean reclone at `0222ed8`. Ancestor of canonical. | **ARCHIVE then DELETE** |
| `E:\scripts-python\xavier-clone` | `fix/email-to-github` | 0 | One extra commit `127ba3a fix: replace email refs` affecting `src/tools/kanban.rs`. Need verify if still relevant before deletion. | **MIGRATION CHECK** |
| `E:\scripts-python\xavier-work` | `master` | 4 | Old work tree with docs already migrated, plus uncommitted Cortex outbound adapter/port. Canonical has newer inbound plugin architecture. | **DO NOT DIRECT-MERGE; archive after extracting notes** |
| `E:\scripts-python\temp_xavier_check` | `master` | 0 | Old Git Core structure check at `cae4da1`. No obvious logic missing from canonical beyond old generated/build artifacts. | **ARCHIVE then DELETE** |
| `E:\scripts-python\xavier-benchmark` | non-git | n/a | Benchmark support folder, not same repo. | **KEEP SEPARATE unless obsolete confirmed** |

## Dispersed logic assessment

### 1. `xavier-work` docs

Files below are byte-identical in canonical `xavier`; no migration needed:

- `docs/SWAL-ARCH.md`
- `docs/TODO.md`
- `docs/ADR/001-memory-domain.md`
- `docs/ADR/002-ports-when.md`
- `docs/ADR/003-agent-state.md`
- `docs/ADR/004-cortex-plugin.md`

### 2. `xavier-work` uncommitted Cortex port/adapter

Uncommitted files:

- `src/adapters/outbound/cortex_adapter.rs`
- `src/ports/outbound/cortex_port.rs`
- module exports in `src/adapters/outbound/mod.rs` and `src/ports/outbound/mod.rs`

Recommendation: **do not migrate as code**.

Reasons:

- Canonical `xavier` already has newer plugin implementation: `src/adapters/inbound/http/plugins/cortex.rs` and `pgheart.rs`.
- The old adapter is transport-only and does not persist pulled records locally.
- It stores mutable sync state inside a struct but trait methods use `&self`, which is architecturally suspect and likely compile-breaking unless hidden behind interior mutability.
- It uses a generic `Authorization` header value from token without normalizing `Bearer ...`.

Action: preserve as archived reference only. If a future outbound Cortex backend is needed, reimplement against the canonical plugin/MemoryBackend architecture.

### 3. Old clone-only files absent from canonical

Repeated across old clones:

- build logs: `build-*.txt` — discard.
- generated/site lock: `docs/site/package-lock.json` — canonical intentionally removed.
- old storage abstractions: `src/ports/outbound/storage_port.rs`, `src/adapters/outbound/sqlite/*`, `src/adapters/outbound/vec/storage_adapter.rs` — superseded by current architecture.
- old memory modules: `src/memory/qmd_memory.rs`, `src/memory/surreal_store.rs`, `src/domain/memory/types.rs` — superseded/reorganized in canonical.
- backup/db artifacts: `src/server/http.rs.bak.2026-04-16`, `xavier_memory_vec.db-wal` — discard.

### 4. `xavier-clone` branch `fix/email-to-github`

Commit: `127ba3a fix: replace email refs`, only touches `src/tools/kanban.rs`.

Status after focused diff: **do not migrate**.

Reasons:

- Canonical `xavier/src/tools/kanban.rs` already has safer env-only config via `PLANKA_URL`, `PLANKA_EMAIL`, `PLANKA_PASSWORD`.
- Canonical implements custom `Debug` for `PlankaConfig` and redacts password.
- The clone commit introduces a fallback string that looks like a URL in the email field; this is not a real improvement.
- The change is mostly line-ending/noise (`645 insertions / 645 deletions`) with no valuable behavior to port.

Decision: archive/delete `xavier-clone` after cleanup confirmation.

## Related but separate projects

These are not duplicate Xavier clones; do not delete under this reconciliation pass without separate review:

- `cortex`, `cortex-1`, `cortex-xavier-sync`
- `pgheart`, `pgheart-admin`
- `agent-memory`, `isar_agent_memory`, `memory-core`, `memory-benchmark`

They may contain integration logic, but they are separate products/services or historical experiments.

## Safe cleanup plan

1. Create archive folder: `E:\scripts-python\_archive\xavier-reconcile-20260508`.
2. For each DELETE candidate, write `git status`, `git log -20`, `git remote -v`, and `git diff` to archive metadata.
3. Move candidate folders into archive first, do not hard-delete immediately.
4. Run canonical verification:
   - `cargo fmt --check`
   - `cargo check --lib`
   - `cargo test --lib`
5. Keep archive for at least 7 days before permanent deletion.

## Proposed immediate actions

- [x] Focused diff of `xavier-clone/src/tools/kanban.rs` vs canonical.
- [x] Decide whether to port minimal kanban naming/security cleanup: **do not port**.
- [x] Add local/private memory summary files to `.git/info/exclude` so they do not show as untracked.
- [x] Create dry-run/execute archive script: `scripts/reconcile-xavier-clones.ps1`.
- [x] Run validation: `cargo fmt`, `cargo fmt --check`, `cargo check --lib` passed.
- [ ] Archive/delete clean obsolete clones after BELA confirms.
