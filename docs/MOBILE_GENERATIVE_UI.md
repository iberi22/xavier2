# XAVIER MOBILE + GENERATIVE UI — SPEC

## Overview

Two major additions:
1. **Generative Chat UI** — Natural language → Dynamic UI components
2. **Mobile App** — Flutter app for iOS/Android

---

## Part 1: Generative Chat UI

### What Is It?

```
User: "Show me all my client projects with their status"
        │
        ▼
┌─────────────────────────────────────┐
│  Xavier Chat (Agent)                │
│                                     │
│  [Understands intent]               │
│  [Retrieves memories]              │
│  [Generates UI component]          │
│                                     │
│  ┌─────────────────────────────┐   │
│  │ 📊 Projects Dashboard       │   │
│  │ ─────────────────────────── │   │
│  │ 🟢 ManteniApp    | Active  │   │
│  │ 🟡 Xavier        | Testing │   │
│  │ 🔴 Tripro Landing| Done    │   │
│  └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

### Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Client   │────▶│  Xavier     │────▶│  LLM API    │
│  (Browser) │     │  Chat API   │     │  (pplx)     │
└─────────────┘     └──────┬──────┘     └─────────────┘
                           │
                    ┌──────▼──────┐
                    │  UI Gen     │
                    │  Engine     │
                    └─────────────┘
```

### UI Components to Generate

| Component | Trigger Phrases |
|-----------|-----------------|
| **Stats Card** | "show stats", "how many", "summary" |
| **Table** | "list all", "show me the", "compare" |
| **Timeline** | "history of", "when did", "timeline" |
| **Chart** | "graph", "visualize", "over time" |
| **Dashboard** | "overview", "full status", "everything" |
| **Alert** | "warn", "issue", "problem", "error" |
| **Empty State** | "nothing found", "no data" |

### API Endpoint

```rust
POST /chat/generate
Request:
{
  "message": "Show me all my client projects",
  "context": {
    "user_id": "bela",
    "workspace": "default"
  }
}

Response:
{
  "component": "dashboard",
  "data": {
    "title": "Client Projects",
    "items": [...]
  },
  "markdown": "Here's your **3 active projects**...",
  "actions": [...]
}
```

### Implementation Steps

1. **Chat Endpoint** — `POST /chat/generate`
2. **Intent Classifier** — Detect what UI component to show
3. **Memory Retrieval** — Get relevant memories
4. **LLM Call** — Generate response + UI spec
5. **UI Renderer** — Render component on client

### Tech Stack

| Layer | Technology |
|-------|------------|
| API | Rust (existing) |
| Intent | Rule-based + LLM |
| Memory | Xavier existing |
| LLM | pplx-embed (already running) |
| Frontend | React/Vue/Svelte |
| Mobile | Flutter (see Part 2) |

---

## Part 2: Mobile App (Flutter)

### Scope

Cross-platform Flutter app for iOS/Android.

### Features

| Feature | Priority | Status |
|---------|----------|--------|
| Chat interface | P0 | TODO |
| Memory search | P0 | TODO |
| Add memory | P0 | TODO |
| Memory stats | P1 | TODO |
| Push notifications | P2 | TODO |
| Biometric auth | P2 | TODO |
| Offline mode | P2 | TODO |

### Architecture

```
┌─────────────────┐
│   Flutter App   │
├─────────────────┤
│  chat_screen    │──▶ Xavier Cloud API
│  search_screen │──▶ (encrypted)
│  profile_screen│
└─────────────────┘
```

### Flutter Dependencies

```yaml
dependencies:
  flutter:
    sdk: flutter
  http: ^1.2.0        # API calls
  flutter_secure_storage: ^9.0.0  # Token storage
  local_auth: ^2.1.0   # Biometrics
  hive: ^2.2.3         # Offline cache
```

### Project Location

```
E:\scripts-python\xavier-mobile\  (NEW)
├── lib/
│   ├── main.dart
│   ├── screens/
│   │   ├── chat_screen.dart
│   │   ├── search_screen.dart
│   │   └── profile_screen.dart
│   ├── widgets/
│   │   ├── memory_card.dart
│   │   └── chat_bubble.dart
│   └── services/
│       └── xavier_api.dart
├── pubspec.yaml
└── android/ios/
```

---

## Part 3: Google Cloud Deployment

### Current State
- Xavier running locally on port 8003
- Need cloud deployment for mobile app + API access

### Cloud Architecture

```
                    ┌─────────────────┐
                    │   Cloud Run     │
┌─────────┐         │   Xavier API    │
│ Mobile  │────────▶│   (Docker)      │
│   App   │  HTTPS  │                 │
└─────────┘         └────────┬────────┘
                            │
                    ┌────────▼────────┐
                    │  Cloud SQL      │
                    │  (PostgreSQL)   │
                    └─────────────────┘
```

### Deployment Steps

1. **Build Docker image**
2. **Push to Artifact Registry**
3. **Deploy to Cloud Run**
4. **Set up custom domain** (optional)
5. **Configure TLS** (automatic with Cloud Run)

### Cost (Free Tier)

| Resource | Usage | Cost |
|----------|-------|------|
| Cloud Run | 2M reqs/mo | **$0** |
| 1 CPU, 512MB | Always on | **$0** (free tier) |
| Egress | ~5GB/mo | **~$0.12** |
| **Total** | | **~$0.12/mo** |

---

## Part 4: Internal License ($8/mo)

### Why Pay When We Can Self-Host?

1. **Support development** — Money funds Xavier R&D
2. **Production usage** — Real traffic, real bugs
3. **Feature testing** — Cloud tier features before release
4. ** SLA** — If something breaks, we get priority

### Plan

1. Sign up at xavier.swal.ai (when live)
2. Choose Cloud tier: $8/mo
3. Use internally for all agents
4. Provide feedback for improvements

---

## Implementation Roadmap

### Week 1: Chat API

- [ ] `POST /chat/generate` endpoint
- [ ] Intent classifier (rule-based)
- [ ] pplx-embed integration for response
- [ ] Basic React UI

### Week 2: UI Components

- [ ] Stats card component
- [ ] Table component
- [ ] Timeline component
- [ ] Dashboard component

### Week 3: Flutter Setup

- [ ] Create Flutter project
- [ ] Chat screen UI
- [ ] API client service
- [ ] Secure token storage

### Week 4: Cloud Deployment

- [ ] Docker build + push
- [ ] Cloud Run deployment
- [ ] Custom domain setup
- [ ] Mobile app pointing to cloud

---

## Files to Create

```
E:\scripts-python\xavier\
├── src\chat\              (NEW)
│   ├── mod.rs
│   ├── generate.rs
│   ├── intent.rs
│   └── ui_components.rs
├── web\chat\             (NEW)
│   ├── index.html
│   ├── App.tsx
│   └── components\
├── docs\
│   ├── GENERATIVE_UI.md
│   └── MOBILE_APP.md
└── docker-compose.prod.yml (UPDATE)

E:\scripts-python\xavier-mobile\  (NEW)
├── lib\
├── pubspec.yaml
└── ...
```

---

## Status Checklist

| Item | Status | Notes |
|------|--------|-------|
| Chat API | TODO | Needs development |
| Intent Classifier | TODO | Rule-based + LLM |
| UI Components | TODO | 6 component types |
| React Frontend | TODO | Chat interface |
| Flutter App | TODO | Mobile |
| Cloud Deploy | TODO | Cloud Run |
| Internal License | TODO | Buy $8/mo plan |

---

## Next Steps

1. **Today**: Start chat API development
2. **This week**: Get React chat UI working
3. **Next week**: Flutter app skeleton
4. **After**: Cloud deployment + internal license
