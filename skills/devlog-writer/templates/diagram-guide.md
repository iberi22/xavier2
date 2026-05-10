# DevLog Diagram Guide

Use this guide to ensure consistency in technical diagrams within DevLog posts.

## 1. Flowcharts (Decision Logic)
Use for explaining "how it works" or "decision trees".
**Type**: `graph TD` or `graph LR`

## 2. Sequence Diagrams (Communication)
Use for explaining interactions between components (e.g., Agent -> Memory -> Storage).
**Type**: `sequenceDiagram`

## 3. Class Diagrams (Structures)
Use for explaining Rust structs, traits, and their relationships.
**Type**: `classDiagram`

## 4. State Diagrams (Lifecycle)
Use for explaining the state transitions of a memory or a task.
**Type**: `stateDiagram-v2`

## Rules
- Use descriptive node names.
- Keep diagrams simple; focus on the specific concept being explained.
- Always use high-contrast styling compatible with both light and dark modes.
