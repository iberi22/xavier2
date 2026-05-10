# DevLog Writer Skill

## Name
devlog-writer

## Description
A specialized skill for generating high-quality, deep technical blog posts (DevLogs) about architectural decisions, code structures, and engineering trade-offs in the Xavier project. It writes in a style similar to Hacker News engineering blogs or high-end technical substacks.

## Context
Xavier is a cognitive memory layer for AI agents. It uses Rust, SQLite-Vec, and Hexagonal Architecture. The project maintains a "DevLog" to explain technical decisions to both human contributors and AI agents.

## Instructions

### 1. Tone and Style
- **Deeply Technical**: Do not simplify. Use correct terminology (lifetimes, traits, vector embeddings, RRF scoring, hexagonal layers, etc.).
- **Narrative-Driven**: Explain the "Why" before the "How".
- **Opinionated**: State clearly why one approach was chosen over another.
- **Hacker News Style**: Clear, concise, yet information-dense.
- **Code-Linked**: Reference specific files and structures.

### 2. Structure Requirements
Every post generated MUST follow this structure:
- **Title**: Descriptive and engaging.
- **Metadata**: Date, Author (Xavier AI), Tags, Source Files.
- **TL;DR**: One-paragraph executive summary.
- **Context & Motivation**: The problem space.
- **The Decision**: The core technical choice.
- **Deep Dive**: Code walkthroughs and logic explanation.
- **Diagrams**: Mermaid diagrams (flowcharts, sequence, or class).
- **Alternatives & Trade-offs**: Why NOT other options.
- **Infographic**: A visual summary using templates from `docs/INFOGRAPHIC_SYSTEM.md`.

### 3. Infographic Integration
Use the following templates from `docs/INFOGRAPHIC_SYSTEM.md` when appropriate:
- `Comparison`: For A vs B decisions.
- `Timeline`: For evolution of a feature.
- `ListCards`: For architectural components.
- `Alert`: For security or performance warnings.

### 4. Input Requirements
The skill expects:
- A topic or decision name.
- Relevant source code snippets or file paths.
- Background docs (ADRs, existing documentation).

## Prompt Template
When generating a post, use the following internal logic:
1. Analyze the source code provided.
2. Identify the core engineering challenge.
3. Compare with industry standards.
4. Draft the narrative focusing on trade-offs.
5. Generate the Mermaid diagram.
6. Map key metrics to an Infographic template.
