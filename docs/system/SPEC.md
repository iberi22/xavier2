# DocNode - Minimal Astro + Svelte Documentation System

## Vision

A simple, fast, and vault-compatible documentation system that reads markdown files and generates an interactive static site with graph visualization.

---

## Architecture

```
docs/system/
├── src/
│   ├── components/        # Svelte components
│   │   ├── Graph.svelte  # Node graph visualization
│   │   ├── Sidebar.svelte
│   │   ├── Search.svelte
│   │   └── Doc.svelte
│   ├── pages/            # Astro pages
│   │   ├── index.astro
│   │   └── doc/[slug].astro
│   ├── lib/
│   │   ├── vault.ts     # Vault file reader
│   │   ├── graph.ts     # Graph relationships
│   │   └── nav.ts       # Navigation builder
│   └── styles/
│       └── global.css
├── content/              # Vault folder (markdown files)
│   └── .gitcore/
├── public/
├── astro.config.mjs
├── package.json
└── tsconfig.json
```

---

## Key Features

### 1. Vault Reader
- Read markdown files from any folder
- Parse frontmatter (YAML)
- Extract internal links `[[doc-name]]`
- Build relationship graph

### 2. Graph Visualization
- D3.js or custom canvas for node graph
- Show document connections
- Interactive: click to navigate
- Filter by tags

### 3. Search
- Client-side fuzzy search
- Index titles, content, tags
- Instant results

### 4. Node View
- Each document = one node
- Edges = internal links
- Tags = node colors
- Click to view document

---

## Frontmatter Schema

```yaml
---
title: "Document Title"
type: module | api | guide | reference
tags: ["#tag1", "#tag2"]
related: ["other-doc", "another-doc"]
sources: ["SRC.md", "ARCHITECTURE.md"]
version: "1.0"
last_updated: "2026-03-15"
---
```

---

## Internal Link Syntax

Support Obsidian-style links:
- `[[document-name]]` - Link to doc
- `[[document-name|Display Text]]` - Link with alias

---

## API Endpoints (Static)

| Path | Description |
|------|-------------|
| `/api/graph.json` | All nodes and edges |
| `/api/search.json` | Search index |
| `/api/nav.json` | Navigation tree |

---

## Build Commands

```bash
# Development
npm run dev

# Build
npm run build

# Preview
npm run preview
```

---

## Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| astro | ^5.0 | Static site generator |
| @astrojs/svelte | ^4.0 | Svelte integration |
| svelte | ^5.0 | UI components |
| marked | ^15.0 | Markdown parser |
| gray-matter | ^4.0 | Frontmatter parser |
| flexsearch | ^0.7 | Full-text search |

---

## Implementation Phases

### Phase 1: Base (This week)
- [x] Specification (this doc)
- [ ] Project setup (Astro + Svelte)
- [ ] Vault reader
- [ ] Basic markdown rendering

### Phase 2: Graph (Next week)
- [ ] Graph component
- [ ] Internal link extraction
- [ ] Relationship building

### Phase 3: Search & UI
- [ ] Search component
- [ ] Sidebar navigation
- [ ] Responsive design

---

## Vault Structure

The system is designed to work with any folder structure:

```
my-vault/
├── .gitcore/           # GitCore docs (optional)
│   ├── SRC.md
│   └── ARCHITECTURE.md
├── docs/              # General docs
│   └── guide.md
└── api/               # API docs
    └── endpoints.md
```

Configure via `docs/system/astro.config.mjs`:

```javascript
export default defineConfig({
  docsSystem: {
    vault: './content',
    baseUrl: '/docs'
  }
})
```

---

*Document version: 1.0*
*Created: 2026-03-15*
