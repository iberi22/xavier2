---
title: "FEAT: Implement System Tray Icon with LED Triangle Animation"
labels: ["enhancement", "ui", "rust"]
status: "OPEN"
priority: "MEDIUM"
summary: |
  Create a cross-platform system tray icon for Xavier. The icon should consist of 3 dots forming a triangle that glow mint-green when the system is active.
---

# Feature: System Tray Icon

## Context
Xavier needs a non-intrusive way to show its background status and provide quick access to its controls.

## Requirements
- Use `tray-icon` and `tao` in Rust.
- **Visuals**:
    - 3 dots in a triangle configuration.
    - Active state: Mint green LED effect.
    - Inactive: Dim gray.
- **Interaction**:
    - Right-click menu: [Status, Start/Stop, Open Config, Dashboard, Exit].
    - Left-click: Show/Hide the Floating Config UI (Issue #2).

## Acceptance Criteria
- [ ] Tray icon appears in the system notification area (Windows/macOS/Linux).
- [ ] Icon color switches based on Xavier server status.
- [ ] Context menu items trigger appropriate actions.

## References
- See `scripts/xavier-service.ps1` for existing status check logic.
