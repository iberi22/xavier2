## Target Files (changes ALREADY written — only verify & compile)

- `src/installer/wizard.rs` — +138 lines: `render_step_ansi()`, `render_all_steps_ansi()`, color-to-ANSI conversion from TUI buffer
- `src/lib.rs` — restored from GitHub main branch (was accidentally overwritten with a minimal version)
- `src/main_installer.rs` — feature-gated with clear error messages for missing features
- `examples/wizard_screenshots.rs` — new example that generates `.ansi` files for each wizard step

## Task
1. `cargo check --lib --features cli-interactive --no-default-features` — must compile clean
2. If any compilation errors, fix them
3. `cargo run --example wizard_screenshots --features cli-interactive --no-default-features` — must run and generate files
4. Commit with message: `feat(installer): phase 4 — TUI screenshot pipeline (ANSI rendering + example)`

## Success Criteria
- `cargo check` exits 0
- Example compiles and runs, generating `.ansi` files in `docs/screenshots/`
- No regressions in existing functionality
