# ADR-005: Multi-Crate Workspace Migration

*Status: PROPOSED | Date: 2026-04-26*

---

## Context

Currently, Xavier is structured as a single monolithic crate (plus one workspace member `code-graph`). While this simplified initial development, it now presents several challenges:
- **Coupling**: Functional modules like `security` and `memory` are tightly coupled within the same crate.
- **Reusability**: Components like the `PromptInjectionDetector` cannot be easily used by other projects without pulling in the entire `xavier` dependency tree (including `surrealdb-core`, which is heavy).
- **Compile Times**: Any change in a low-level utility triggers a rebuild of the entire project.
- **Publishing**: We cannot publish individual components to crates.io independently.

---

## Decision

We will transition Xavier to a multi-crate workspace structure starting in v0.5.0. This involves extracting core modules into their own crates under a `crates/` directory.

### Target Architecture:
- `xavier` (root): CLI and HTTP server (binary crate).
- `crates/xavier-common`: Shared utilities, crypto, errors, and base types.
- `crates/xavier-security`: Security scanner, prompt guard, and detection layers.
- `crates/xavier-memory`: Core memory domain, storage traits, and SQLite-vec implementation.
- `crates/xavier-a2a`: Agent-to-Agent protocol and registry.

---

## Rationale

1. **Granular Dependencies**: `xavier-security` can remain lightweight, while `xavier-memory` can house the heavier database dependencies.
2. **Parallel Compilation**: Cargo can compile independent crates in parallel, improving development velocity.
3. **Ecosystem Growth**: By providing standalone crates like `xavier-security`, we allow the community to adopt parts of Xavier in their own agents.
4. **Clear Boundaries**: Enforced by crate visibility rules, leading to better internal architecture.

---

## Proposed Extraction Sequence

1. **Step 1: `xavier-common`**
   - Extract `src/utils/`, `src/crypto/`, and shared constants.
2. **Step 2: `xavier-security`**
   - Extract `src/security/`. Depends on `xavier-common`.
3. **Step 3: `xavier-memory-core`**
   - Extract `src/memory/` core logic and traits.
4. **Step 4: `xavier-memory-sqlite`** (optional split)
   - Specialized crate for SQLite-vec backend.

---

## Consequences

**Positive:**
- Better code organization and maintainability.
- Faster CI/CD pipelines (via crate caching).
- Enable multi-crate publishing to crates.io.

**Negative:**
- Initial overhead of managing workspace-wide dependencies.
- Need to update internal paths and `use` statements.
- Increased complexity in the root `Cargo.toml`.
