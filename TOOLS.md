# TOOLS.md - Xavier2 Tools

## Xavier2 (Mi Cerebro - PRIMARY MEMORY)

### API Direct
- **URL:** http://localhost:8006
- **Token:** dev-token (header: `X-Xavier2-Token`)
- **Endpoints:**
  - `GET /health` — Status
  - `POST /memory/search` — Vector search
  - `POST /memory/add` — Add memory
  - `GET /memory/stats` — Stats

### Wrapper Script (PowerShell)
```powershell
# Ubicación
.\scripts\xavier2-helper.ps1

# Comandos
.\scripts\xavier2-helper.ps1 -Action search -Query "roadmap"
.\scripts\xavier2-helper.ps1 -Action add -Content "..." -Category decisions
.\scripts\xavier2-helper.ps1 -Action health
.\scripts\xavier2-helper.ps1 -Action stats
```

---

## Web Research (3 Providers)

| Provider | Uso | API Key |
|----------|-----|---------|
| MiniMax MCP | Primary | Built-in |
| Tavily | AI search | tvly-dev-eqveB... |
| Brave | Fast | BSA45gpKkamwtD... |

---

## SWAL Node (Termux)

```bash
# SSH a Termux
ssh termux-cf

# Script principal
swal-node.sh [docker|xavier2|status|tunnel|restart]
```

### Docker Containers (PC)
| Container | Port | Status |
|-----------|------|--------|
| xavier2 | 8006 | ✅ |
| cortex | 8003 | ✅ |
| synapse-dashboard | 8080 | ✅ |
| pgheart-postgres | 5432 | ✅ |
| pplx-embed | 8002 | ⚠️ NO conectado |

---

## GitHub

### Repos Principales
- `iberi22/xavier2` — Mi cerebro
- `iberi22/gestalt-rust` — Agente Rust
- `iberi22/tripro_landing_page_astro` — ManteniApp

### CLI
```bash
gh issue list --repo iberi22/xavier2
gh issue create --repo iberi22/xavier2 --title "..." --body "..."
```

---

## Codex (Coding Agent)

```powershell
cd E:\scripts-python\cortex ; codex --dangerously-bypass-approvals-and-sandbox exec "tu prompt"
```

---

## Audio Transcription

```bash
python E:\scripts-python\scripts\audio-to-text.py audio.ogg --language es
```

---

_Xavier2 CEO — Herramientas actualizadas 2026-04-30_