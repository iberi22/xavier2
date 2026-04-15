# Xavier2

> Cognitive memory runtime for AI agents — open source, built for production, designed to scale.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.4.1-green.svg)](https://github.com/iberi22/xavier2-1)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](https://www.rust-lang.org/)

---

## 🎁 If Xavier2 Saves You Time — Support Its Future

Xavier2 is **free and open source under MIT** — anyone can use it, fork it, and build on it.

If you're **using Xavier2 to power a product, a startup, or any commercial activity**, the MIT license gives you that freedom — but it also means the development is sustained by people who invest their time and resources into making it better.

**Consider becoming a supported user:**

- 💼 **Commercial users** — if you're building a product on Xavier2, a support agreement ensures you get priority fixes, feature requests, and a direct line to the people who know the codebase best.
- 🏢 **Companies** — enterprise subscriptions fund infrastructure, GPU resources, and dedicated development time.
- 👤 **Individuals** — annual plans are affordable and go a long way.

Every subscription, sponsor, and enterprise contract keeps the MIT license intact for everyone — while funding the people who maintain this project.

**👉 [View Pricing & Plans](docs/PRICING.md)**
**📧 enterprise@southwest-ai-labs.com**

---

## What It Is

Xavier2 is a Rust-based shared memory system for AI agent workflows. It gives agents persistent, searchable memory with vector search, knowledge graphs, and hybrid retrieval — all in a single self-hosted runtime.

```
curl -X POST http://localhost:8003/memory/add \
  -H "X-Xavier2-Token: $TOKEN" \
  -d '{"content":"Design decision: use RRF k=60","path":"decisions/001"}'

curl -X POST http://localhost:8003/memory/search \
  -H "X-Xavier2-Token: $TOKEN" \
  -d '{"query":"design decisions"}'
```

### Key Features

- **Hybrid vector search** — RRF fusion of dense vectors + FTS5 + knowledge graph signals
- **Multiple backends** — file, memory, sqlite, vec (sqlite-vec) — switch with `XAVIER2_MEMORY_BACKEND`
- **Code indexing** — understand your codebase structure automatically
- **MCP transport** — connect to any MCP-compatible AI client
- **Panel UI** — visual memory browser at `/panel`
- **Audit chain** — tamper-evident hash chain on all memory operations

### Backends

| Backend | Best For | Persistence | Speed |
|---------|----------|-------------|-------|
| `file` | Local dev | ✅ | 429ms/doc |
| `memory` | Ephemeral sessions | ❌ | 1.3ms/doc |
| `sqlite` | Lightweight prod | ✅ | 1.5ms/doc |
| `vec` ⭐ | Full vector search | ✅ | 8.3ms/doc |

---

## Quick Start

```bash
git clone https://github.com/iberi22/xavier2-1.git
cd xavier2-1
cp .env.example .env
# Edit .env and set XAVIER2_TOKEN to a long random string

docker compose up -d

# Verify
curl http://localhost:8003/health

# Add memory
curl -X POST http://localhost:8003/memory/add \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content":"First memory","path":"test","metadata":{}}'

# Search
curl -X POST http://localhost:8003/memory/search \
  -H "X-Xavier2-Token: $XAVIER2_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query":"first","limit":5}'
```

---

## Licensing

Xavier2 uses a **dual-license model**:

| License | Who It's For | Cost |
|---------|-------------|------|
| **MIT** | Open source users, hobbyists, researchers | Free |
| **Enterprise** | Commercial products, companies, startups | [See pricing](docs/PRICING.md) |

**The MIT license is permanent and irrevocable.** Buying an enterprise license never affects your rights under MIT — it only adds support, priority features, and reserved enterprise capabilities.

See: [LICENSE](LICENSE) · [LICENSE.enterprise](LICENSE.enterprise) · [ENTERPRISE_SECRETS.md](ENTERPRISE_SECRETS.md)

---

## Enterprise Edition

The open source version (MIT) provides the Xavier2 core runtime, self-hosting, HTTP and MCP interfaces, code indexing, and the standard memory retrieval pipeline.

Enterprise customers get proprietary enhancements, advanced audit and logging controls, private embedding options, and priority support with direct SWAL escalation.

Contact: `enterprise@southwest-ai-labs.com`

---

## Enterprise Features (Reserved)

These capabilities are available only to enterprise customers and are documented in [ENTERPRISE_SECRETS.md](ENTERPRISE_SECRETS.md):

- 🔐 **Proprietary embedding models** — hosted backends with organization-specific tuning
- 📊 **Custom RRF scoring** — weighted multi-signal reranking for retrieval quality
- 🛡️ **Advanced audit logging** — expanded retention, compliance exports, tamper-evident pipelines
- 🎯 **Priority support queue** — direct escalation to engineering with contracted SLAs
- 🤖 **Fine-tuning service** — train custom embedding models on your private data
- ⚡ **Dedicated GPU resources** — isolated GPU cards for embedding inference and fine-tuning
- ☁️ **Colab integration** — connect your Colab account to run fine-tuning jobs directly from Xavier2's interface

---

## Docs

- [📄 Full Documentation](docs/site/index.html)
- [💰 Pricing & Plans](docs/PRICING.md)
- [🔐 Enterprise Features](ENTERPRISE_SECRETS.md)
- [🏗️ Architecture](.gitcore/ARCHITECTURE.md)
- [🔧 Storage & Backends](docs/STORAGE_SWITCH.md)

---

## Security Notes

- Auth via `X-Xavier2-Token` header — never use `dev-token` in production
- JWT/RBAC code exists in `src/security/` (not yet active)
- Health/readiness endpoints are intentionally public
- See [SECURITY.md](SECURITY.md) for full security disclosure policy
