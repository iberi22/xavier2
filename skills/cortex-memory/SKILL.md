# SKILL.md - Xavier Memory Agent Skill

## Name
**xavier-memory** — Shared memory system for AI agents

## Description
Use Xavier as the shared brain for all agent operations. This skill provides:
- Memory read/write operations
- Automatic sync from agent sessions
- Token-efficient patterns
- Cross-agent context sharing

## Category
Memory / Knowledge Management

## Platforms
- OpenClaw
- Claude Code
- Codex
- Any REST API client

## Commands

### read_memory(query, limit=5)
Search Xavier for relevant memories.

```powershell
POST /memory/search
Body: { "query": "your question", "limit": 5 }
```

### write_memory(content, metadata)
Add a memory to Xavier.

```powershell
POST /memory/add
Body: {
  "content": "Factual summary (max 2KB)",
  "metadata": {
    "source": "agent-name",
    "priority": "high|medium|low",
    "category": "technical|client|operations|sales"
  }
}
```

### manage_memories(operation)
Auto-curation operations.

```powershell
# Apply decay
POST /memory/decay

# Merge duplicates
POST /memory/consolidate

# Remove low-quality
DELETE /memory/evict?threshold=0.2

# Check quality
GET /memory/quality?threshold=0.3
```

### get_stats()
Get memory statistics.

```powershell
GET /memory/stats
```

## Usage Examples

### Before starting a task
```
READ: "context about current project"
→ Search memories related to the project
```

### After making a decision
```
WRITE: "Decision: chose X because Y. Next step Z."
→ Store decision with source=agent, category=technical
```

### When asked about a client
```
READ: "client profile and history"
→ Search memories about the client
```

## Environment

| Variable | Default | Description |
|----------|---------|-------------|
| XAVIER_URL | http://localhost:8003 | Xavier API URL |
| XAVIER_TOKEN | dev-token | API authentication token |

## Pricing

| Tier | Price | Features |
|------|-------|----------|
| **Local** | Free | Single machine, unlimited agents |
| **Cloud** | $8/mo | Cross-device sync, web endpoint, 10GB storage |

## Installation

```bash
# Clone repository
git clone https://github.com/iberi22/xavier.git
cd xavier

# Run with Docker
docker compose up -d

# Verify
curl http://localhost:8003/health
```

## Files

- `SKILL.md` — This file
- `xavier-client.js` — Node.js client library
- `examples/` — Usage examples

## Author
SouthWest AI Labs — https://swal.ai
