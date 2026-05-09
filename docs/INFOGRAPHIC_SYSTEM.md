# XAVIER INFOGRAPHIC DESIGN SYSTEM

**AI generates UI instantly using pre-built components — no LLM generation cost.**

---

## Concept

```
USER: "Show me Q1 sales summary"
        ↓
AI selects: ReportTemplate + StatsRow + ChartBar
        ↓
AI fills with data from Xavier
        ↓
INSTANT beautiful infographic
```

**No expensive LLM generation.** Just component selection + data binding.

---

## 20 Pre-built Templates

### 📊 METRICS
| Template | Use When | Data Shape |
|----------|----------|------------|
| `MetricBig` | Hero numbers | `{value, label, change}` |
| `StatsRow` | 4 KPIs | `{items: [{v, l}]}` |
| `IconStat` | Icon + stat | `{icon, value, label, color}` |
| `DividerStats` | Side comparison | `{left: {v,l}, right: {v,l}}` |
| `Sparkline` | Mini trend | `{value, data: []}` |

### 📈 DATA
| Template | Use When | Data Shape |
|----------|----------|------------|
| `ChartBar` | Bar chart | `{title, data: [{l,v}]}` |
| `Progress` | Goals | `{items: [{l, v, c}]}` |
| `Leaderboard` | Rankings | `{items: [{n, v}]}` |
| `Pipeline` | Funnel | `{stages: [{n, c, co}]}` |

### 📋 LISTS
| Template | Use When | Data Shape |
|----------|----------|------------|
| `ListCards` | Icon cards | `{items: [{i, t, s}]}` |
| `Timeline` | Events | `{items: [{d, t, dd}]}` |
| `Checklist` | Tasks | `{items: [{t, d}]}` |
| `BadgeRow` | Tags | `{badges: []}` |

### 📄 DOCS
| Template | Use When | Data Shape |
|----------|----------|------------|
| `Report` | Documents | `{title, date, sections: [{t, p}]}` |
| `Comparison` | A vs B | `{left: {l,v}, right: {l,v}, vs}` |

### ⚠️ ALERTS
| Template | Use When | Data Shape |
|----------|----------|------------|
| `Alert` | Notifications | `{type, title, desc}` |
| `Minilist` | Quick data | `{items: [{l, v}]}` |

### 🌟 SPECIAL
| Template | Use When | Data Shape |
|----------|----------|------------|
| `HeroBanner` | Announcements | `{value, label, sub}` |

---

## Intent → Template Mapping

| User Says | Template | Example |
|-----------|----------|---------|
| "show me the metrics" | StatsRow + MetricBig | Dashboard overview |
| "compare X and Y" | Comparison | Xavier vs Mem0 |
| "timeline of events" | Timeline | Activity log |
| "progress on goals" | Progress | Sprint status |
| "who's winning" | Leaderboard | Rankings |
| "full report" | Report | Document output |
| "show as bars" | ChartBar | Data viz |
| "list all items" | ListCards | Item catalog |
| "warning about X" | Alert | Notifications |
| "checklist for Y" | Checklist | Task list |
| "big announcement" | HeroBanner | Hero section |
| "tag these" | BadgeRow | Categories |
| "with icons" | IconStat | Icon + number |
| "funnel for sales" | Pipeline | Pipeline view |
| "quick stats" | Minilist | Simple list |
| "split view" | DividerStats | 50/50 |
| "spark trend" | Sparkline | Mini chart |

---

## How AI Uses This

### Step 1: Understand Intent
```
User: "Show me a summary of our Q1 performance"
→ Intent: SUMMARY + METRICS
```

### Step 2: Select Components
```
→ MetricBig (hero metric)
→ StatsRow (4 KPIs)
→ Timeline (key events)
→ ChartBar (monthly data)
```

### Step 3: Fetch Data from Xavier
```
→ Query: "Q1 2026 performance metrics"
→ Get: {revenue, users, growth, events}
```

### Step 4: Bind Data to Components
```
MetricBig: {value: "$1.2M", label: "Q1 Revenue", change: 24}
StatsRow: [{v:"$1.2M",l:"Revenue"}, {v:"45K",l:"Users"}, ...]
Timeline: [{d:"Jan",t:"Launched"}, {d:"Feb",t:"1K users"}, ...]
ChartBar: [{l:"Jan",v:300}, {l:"Feb",v:500}, {l:"Mar",v:400}]
```

### Step 5: Render
```
→ Instant beautiful infographic
→ No LLM generation cost
→ Only API call + data binding
```

---

## Dashboard Builder Integration

### Canvas with Drag & Drop

```
┌─────────────────────────────────────────────────────┐
│  XAVIER DASHBOARD BUILDER                          │
├─────────────────────────────────────────────────────┤
│                                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────┐│
│  │ 📊 MetricBig │  │ 📊 StatsRow  │  │ 📈 Chart ││
│  │   [Drag]     │  │    [Drag]    │  │  [Drag] ││
│  └──────────────┘  └──────────────┘  └──────────┘│
│                                                      │
│  ┌──────────────────────────────────────────────┐  │
│  │ 🌐 Generated Canvas (drag widgets here)      │  │
│  │  [MetricBig] [StatsRow] [Sparkline] [Pin]  │  │
│  └──────────────────────────────────────────────┘  │
│                                                      │
│  [+ Add] [💾 Save] [📌 Pin Current] [🔄 Reset]    │
└─────────────────────────────────────────────────────┘
```

### Widget Actions
- **Drag** — Move widget on canvas
- **Resize** — Change widget size
- **Pin** — Lock widget in place
- **Unpin** — Unlock for editing
- **Delete** — Remove from canvas
- **Duplicate** — Copy widget

### Save to Xavier
```json
{
  "type": "dashboard",
  "name": "Q1 Performance",
  "layout": [
    { "component": "MetricBig", "x": 0, "y": 0, "w": 2, "h": 1, "data": {...} },
    { "component": "StatsRow", "x": 0, "y": 1, "w": 2, "h": 1, "data": {...} }
  ],
  "created": "2026-04-05",
  "pinned": true
}
```

---

## Query → UI Examples

| User Query | Components Generated |
|-----------|-------------------|
| "Q1 performance dashboard" | HeroBanner + StatsRow + ChartBar + Timeline |
| "compare Xavier vs Mem0" | Comparison + BadgeRow |
| "sales pipeline status" | Pipeline + Leaderboard |
| "project checklist" | Checklist + Progress |
| "alerts and warnings" | Alert (multiple) |
| "team rankings" | Leaderboard + BadgeRow |
| "weekly summary" | Report + StatsRow + Minilist |
| "memory metrics" | IconStat (multiple) + Sparkline |

---

## Technical Implementation

### Component Registry
```typescript
const components = {
  MetricBig: { render: (data) => <MetricBig {...data} /> },
  StatsRow: { render: (data) => <StatsRow {...data} /> },
  Timeline: { render: (data) => <Timeline {...data} /> },
  // ... all 20
};
```

### Intent Classifier
```typescript
function classifyIntent(query: string): Component[] {
  // "dashboard" → [HeroBanner, StatsRow, ChartBar]
  // "compare" → [Comparison, BadgeRow]
  // "timeline" → [Timeline]
  // ...
}
```

### Data Binder
```typescript
function bindData(component: string, data: any): Props {
  // Fetch from Xavier
  // Transform to component shape
  // Return bound component
}
```

---

## File Structure

```
xavier/
├── web/
│   ├── design-system/
│   │   └── index.html     ← 20 pre-built templates
│   ├── agent-generative/
│   │   └── index.html     ← AI canvas builder
│   └── dashboard-builder/
│       └── index.html     ← Drag-drop canvas
└── docs/
    └── INFOGRAPHIC_SYSTEM.md
```

---

## Performance

| Metric | Value |
|--------|-------|
| Render time | <50ms |
| LLM calls | 0 (just data fetch) |
| Token cost | ~0 |
| UX | Instant |

---

## Status

| Item | Status |
|------|--------|
| Design System | ✅ 20 templates done |
| Dashboard Builder | 🔜 TODO |
| Canvas drag/drop | 🔜 TODO |
| Save to Xavier | 🔜 TODO |
| Puck integration | 🔜 TODO |

---

*Next: Build the dashboard builder with drag-drop canvas*
