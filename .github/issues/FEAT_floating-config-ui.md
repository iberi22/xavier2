---
title: "FEAT: Create Floating egui Configuration Window"
labels: ["enhancement", "ui", "egui"]
status: "OPEN"
priority: "MEDIUM"
summary: |
  Implement a lightweight, frameless floating window using egui/eframe to allow manual editing of 'config/xavier.config.json'.
---

# Feature: Floating Config UI

## Context
Provide a quick, visual way to modify Xavier settings without opening a full IDE or terminal.

## Requirements
- Use `egui` and `eframe`.
- Frameless/Floating window style.
- **Features**:
    - Load settings from `config/xavier.config.json`.
    - Form fields for: `port`, `workspace_id`, `token`, `log_level`.
    - "Save" button to persist changes.
- **Aesthetics**: Consistent with the mint green LED theme.

## Acceptance Criteria
- [ ] Window opens quickly from the tray icon or CLI.
- [ ] Edits to the form are successfully saved to the JSON file.
- [ ] Changes in the JSON are reflected in the UI upon opening.

## References
- Config path: `config/xavier.config.json`
