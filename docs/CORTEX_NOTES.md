# XAVIER NOTES — Obsidian Replacement

**Open source knowledge base with Xavier memory backend.**

---

## Concept

Xavier Notes is a **local-first, privacy-focused** note-taking app that replaces Obsidian. Instead of local Markdown files, all notes are stored in Xavier — giving you:

- **Semantic search** (find anything by meaning, not just keywords)
- **Cross-note linking** (backlinks, graph view)
- **AI-powered organization** (auto-tags, suggestions)
- **Sync across devices** (via Xavier Cloud)
- **End-to-end encryption** (enterprise tier)

**All for free, open source, with no vendor lock-in.**

---

## Features

### Core (Obsidian-like)

| Feature | Status | Notes |
|---------|--------|-------|
| Markdown editor | ✅ | Full Markdown support |
| Graph view | ✅ | Visualize note connections |
| Daily notes | ✅ | Journal with templates |
| Backlinks | ✅ | See what links to what |
| Tags | ✅ | #tag system |
| Search | ✅ | Full-text + semantic |
| Local-first | ✅ | Data stored in Xavier |
| Dark mode | ✅ | Full dark theme |
| Vault concept | ✅ | Workspace/namespace |

### AI-Powered (Xavier Differentiators)

| Feature | Status | Notes |
|---------|--------|-------|
| Semantic search | ✅ | Meaning-based, not keyword |
| Memory insights | ✅ | "You mentioned X 5 times this week" |
| Auto-linking | ✅ | AI suggests related notes |
| Smart tags | ✅ | AI suggests tags |
| Memory timeline | ✅ | See how your knowledge evolves |
| Cross-agent context | ✅ | Share notes with AI agents |

### Enterprise (Paid Tier)

| Feature | Status | Notes |
|---------|--------|-------|
| E2E encryption | 🔜 | AES-256-GCM |
| Anticipator security | 🔜 | Prompt injection detection |
| Sync across devices | 🔜 | Xavier Cloud |
| Custom domain | 🔜 | Your own endpoint |
| SSO/SAML | 🔜 | Enterprise auth |

---

## UI Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  ◉ Xavier Notes          🔍 Search...          [⚙️] [📱] [👤]   │
├──────────────┬──────────────────────────────────────────────────┤
│              │                                                   │
│  📁 All      │  # Current Note Title                           │
│  📁 Daily    │  ─────────────────────────────────              │
│  📁 Projects  │                                                   │
│  📁 Clients   │  Content of the note in Markdown...             │
│               │                                                   │
│  🏷️ Tags     │  ## Heading                                    │
│  #important   │                                                   │
│  #client      │  More content here...                           │
│  #project     │                                                   │
│               │  [[Backlink to Another Note]]                   │
│  🔗 Backlinks │                                                   │
│  Note A (2)   │  ## Related                                     │
│  Note B (1)   │  [[Auto-linked Suggestion]]                    │
│               │                                                   │
├──────────────┴──────────────────────────────────────────────────┤
│  Graph View  │  Daily: 2026-04-05  │  Recent: note1, note2...  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Graph View

```
        ┌─── Xavier ───┐
       /     │    │     \
      /      │    │      \
   Memory  Search  Notes   Agents
     │       │     │       │
     └───────┴─────┴───────┘
              │
         ┌────┴────┐
         │  Graph  │
         │  View   │
         └─────────┘
```

Interactive 3D graph showing:
- All notes as nodes
- Links between notes as edges
- Clusters by tag/project
- AI-suggested connections

---

## Data Model

### Note
```json
{
  "id": "uuid",
  "title": "Note Title",
  "content": "# Markdown content...",
  "tags": ["tag1", "tag2"],
  "created_at": "2026-04-05T...",
  "updated_at": "2026-04-05T...",
  "links": ["note_id_1", "note_id_2"],
  "backlinks": ["note_id_3"],
  "workspace": "default",
  "encrypted": false
}
```

### Link
```json
{
  "from": "note_id",
  "to": "note_id",
  "type": "wikilink|backlink|autosuggest"
}
```

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Xavier Notes UI                    │
│              (React + TypeScript)                   │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌─────────┐  ┌──────────┐  ┌──────────────┐   │
│  │ Sidebar │  │ Editor   │  │  Graph View  │   │
│  │ - Files │  │ - Markdown│  │  - 3D force  │   │
│  │ - Tags  │  │ - Preview │  │  - Clusters   │   │
│  │ - Search│  │ - Backlink│  │  - Links     │   │
│  └────┬────┘  └────┬─────┘  └──────────────┘   │
│       │            │                              │
│       └────────────┼──────────────────────────────│
│                    │                              │
│              ┌─────▼─────┐                        │
│              │ Xavier    │                        │
│              │ Client    │                        │
│              └─────┬─────┘                        │
│                    │                              │
└────────────────────┼──────────────────────────────┘
                     │
              ┌──────▼──────┐
              │  Xavier API  │
              │  (Rust)      │
              └──────┬──────┘
                     │
              ┌──────▼──────┐
              │  Xavier     │
              │  Memory     │
              └─────────────┘
```

---

## File Structure

```
xavier-notes/
├── src/
│   ├── components/
│   │   ├── Sidebar/
│   │   │   ├── FileTree.tsx
│   │   │   ├── TagList.tsx
│   │   │   └── SearchInput.tsx
│   │   ├── Editor/
│   │   │   ├── MarkdownEditor.tsx
│   │   │   ├── Preview.tsx
│   │   │   └── BacklinkPanel.tsx
│   │   ├── Graph/
│   │   │   ├── GraphView.tsx
│   │   │   └── GraphNode.tsx
│   │   └── Layout/
│   │       ├── AppShell.tsx
│   │       └── Toolbar.tsx
│   ├── hooks/
│   │   ├── useNotes.ts
│   │   ├── useSearch.ts
│   │   ├── useGraph.ts
│   │   └── useXavier.ts
│   ├── services/
│   │   └── xavierClient.ts
│   ├── stores/
│   │   ├── noteStore.ts
│   │   └── uiStore.ts
│   ├── App.tsx
│   └── main.tsx
├── public/
├── package.json
├── vite.config.ts
└── index.html
```

---

## API Integration

### Xavier Endpoints Used

| Endpoint | Usage |
|----------|-------|
| `POST /memory/add` | Create note |
| `POST /memory/search` | Search notes + semantic search |
| `GET /memory/{id}` | Get note by ID |
| `PUT /memory/{id}` | Update note |
| `DELETE /memory/{id}` | Delete note |
| `GET /memory/stats` | Note count, stats |

### Note-specific Operations

```typescript
// Create note
POST /notes
{ title, content, tags, workspace }

// Get backlinks for note
GET /notes/{id}/backlinks

// Get graph data
GET /graph
Response: { nodes: [...], edges: [...] }

// Semantic search
POST /search
{ query: "find notes about project status", type: "semantic" }
```

---

## Comparison vs Obsidian

| Feature | Obsidian | Xavier Notes |
|---------|----------|--------------|
| **Storage** | Local Markdown | Xavier (local or cloud) |
| **Search** | Keyword only | Semantic + keyword |
| **Graph** | Local compute | Cloud compute |
| **Sync** | Paid ($8/mo) | Free (local) / $8 (cloud) |
| **Encryption** | At rest only | E2E (enterprise) |
| **AI features** | Plugins | Built-in |
| **API** | Limited | Full REST API |
| **Mobile** | Paid ($10) | Free (Flutter coming) |
| **License** | Paid (perpetual) | MIT (free forever) |

---

## Pricing

| Feature | Free | Cloud | Enterprise |
|---------|------|-------|------------|
| Notes | Unlimited | Unlimited | Unlimited |
| Semantic search | ✅ | ✅ | ✅ |
| Graph view | ✅ | ✅ | ✅ |
| Local storage | ✅ | ❌ | ❌ |
| Cloud sync | ❌ | ✅ | ✅ |
| E2E encryption | ❌ | ❌ | ✅ |
| Anticipator | ❌ | ❌ | ✅ |
| Price | **$0** | **$8/mo** | **$29/mo** |

---

## Development Roadmap

### Phase 1: Core UI (Week 1)
- [ ] App shell with sidebar
- [ ] Markdown editor with preview
- [ ] File tree / note list
- [ ] Basic search

### Phase 2: Linking (Week 2)
- [ ] Wikilink syntax `[[note]]`
- [ ] Backlinks panel
- [ ] Graph view (basic)

### Phase 3: AI Features (Week 3)
- [ ] Semantic search integration
- [ ] Auto-tagging
- [ ] Related notes suggestions
- [ ] Memory insights

### Phase 4: Polish (Week 4)
- [ ] Dark mode
- [ ] Keyboard shortcuts
- [ ] Mobile responsive
- [ ] Performance optimization

---

## Status

| Component | Status |
|-----------|--------|
| Spec | ✅ Complete |
| React UI (basic) | ✅ Done |
| Chat UI | ✅ Done |
| Full Notes UI | 🔜 TODO |
| Graph View | 🔜 TODO |
| Markdown Editor | 🔜 TODO |
| Semantic Search | ✅ Working via Xavier |
| Mobile App | 🔜 TODO (Flutter) |

---

## Try It

```bash
# Open the chat UI (basic)
file://E:\scripts-python\xavier\web\chat\index.html

# Or serve the full app (when ready)
cd xavier-notes
npm install
npm run dev
```
