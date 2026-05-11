# Phase 4 Deep Review — Wizard ANSI + Playwright + Docs

## Findings Categorized by Severity

### 🔴 Blocker
- **ANSI Style Bleeding**: The ANSI rendering pipeline in `render_step_ansi` did not explicitly reset styles when changing attributes like bold or background color. This caused styles to "bleed" into subsequent cells, leading to incorrect visual representation in `.ansi` files and resulting screenshots.
  - *Fix*: Updated `render_step_ansi` to emit a reset SGR code (`\u{1b}[0m`) before applying any new styles.

### 🟡 Warning
- **Masked Field Multi-byte Inconsistency**: The `render_input_field` used `value.len()` (byte count) instead of character count to determine the number of bullets to display. This resulted in too many bullets for multi-byte characters like emojis.
  - *Fix*: Changed `value.len()` to `value.chars().count()` in `render_input_field`.

### 🟢 Suggestion
- **ANSI Background Coverage**: The original logic skipped background emission for certain specific colors (`BG`, `CARD_BG`). While intended for optimization, it's safer to always emit the background color if it's not `Color::Reset` to ensure the screenshot environment (Playwright) matches the TUI's intended dark theme.
  - *Action*: Simplified background emission logic to include all non-reset background colors.

## Verification Summary
- **ANSI Generation**: Verified that generated `.ansi` files now contain proper reset and background codes.
- **Unit Tests**: Added comprehensive unit tests in `src/installer/wizard_test.rs` covering multi-byte characters, masked input, and cursor positioning. All tests passed.
- **Documentation**: Verified that `README.md` and `docs/FEATURE_STATUS.md` correctly reflect the TUI Installer's status and usage.
