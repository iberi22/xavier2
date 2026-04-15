# Xavier2 Pricing & Commercial Model

> Built and maintained by [SouthWest AI Labs](mailto:enterprise@southwest-ai-labs.com)

---

## Our Philosophy

Xavier2 is **MIT licensed and always will be.** The open source core is free forever. Our commercial products exist to fund the open source development — not to restrict it.

**If you build a product on Xavier2, we want to be part of your success.** Commercial licensing is how we align that interest.

---

## 🏷️ License Tiers

### MIT License (Free)
Everyone can use Xavier2 under MIT — no strings attached.

✅ Use in any open source or commercial project
✅ Fork, modify, distribute
✅ Self-host without limits
❌ No guaranteed support
❌ No reserved enterprise features

---

### Annual Subscription — From $499/year

| Plan | Price | Includes |
|------|-------|----------|
| **Starter** | $499/yr | 1 production deployment, email support, access to reserved enterprise features |
| **Pro** | $1,999/yr | 5 production deployments, priority support (48h response), reserved features + early access |
| **Enterprise** | Custom | Unlimited deployments, dedicated Slack channel, custom RRF tuning, on-site training |

**All plans include:**
- Access to reserved enterprise features documented in [ENTERPRISE_SECRETS.md](../ENTERPRISE_SECRETS.md)
- Priority bug fixes (patches applied to your fork)
- Access to private repository with enterprise patches
- Compliance documentation (SOC2-ready audit logs, GDPR data handling)

**Annual vs One-Time:**
- Annual subscription is **recommended** — it includes updates, priority support, and access to new reserved features as they ship.
- One-time perpetual licenses are available for Pro ($4,999) and Enterprise (contact us) — these include 1 year of updates and support, then revert to community support only.

---

## ⚡ Paid Services (Credits / Top-Up Model)

Some capabilities are offered as **metered services** — pay only for what you use, via a credit balance.

### 💰 Credit Packs

| Pack | Credits | Price | Validity |
|------|---------|-------|----------|
| Pay-as-you-go | 1 credit | $0.01 | 30 days |
| Starter | 500 credits | $4.99 | 90 days |
| Growth | 5,000 credits | $39.99 | 180 days |
| Scale | 50,000 credits | $299.99 | 365 days |

**Credits are consumed by metered services (see below).** Unused credits expire — purchase only what you'll use.

---

## 🔬 Metered Services

### 1. Enterprise Embedding API
**Consumption:** 1 credit per 1,000 tokens processed

- Routes your queries through SWAL-hosted embedding models
- Organization-specific embedding tuning (your model, your data)
- Isolation: your data is NEVER used to train shared models
- Fallback to open embedding providers (OpenAI, Cohere, Voyage)

```bash
export XAVIER2_EMBEDDING_PROVIDER=swal-enterprise
export XAVIER2_ENTERPRISE_API_KEY=your-enterprise-key
```

### 2. Private Fine-Tuning
**Consumption:** 500 credits per fine-tuning job

Train a custom embedding model on your private dataset. This runs on **dedicated GPU infrastructure** — your data never leaves our secure environment.

**What you get:**
- Custom embedding model trained on your data
- Private endpoint for inference
- Model hosted by SWAL, accessed via API
- Retraining available as new data accumulates

**Use cases:**
- Specialized domain memory (legal, medical, financial)
- Codebase-specific embeddings
- Organization-specific terminology and context

```bash
# Submit a fine-tuning job
curl -X POST https://api.xavier2.ai/v1/fine-tune \
  -H "Authorization: Bearer $ENTERPRISE_API_KEY" \
  -d '{"dataset":"legal_memos_2026","model":"xavier2-embed-base","epochs":10}'
```

### 3. Dedicated GPU Resources
**Consumption:** 100 credits per GPU-hour

For enterprise customers who need **guaranteed isolated GPU capacity** — no shared resources, no noisy neighbors.

- Dedicated NVIDIA GPU cards (H100 / A100 / RTX 4090)
- Your own virtual environment
- Full data residency guarantees
- SSH/VPN access to your dedicated instance

**Use cases:**
- High-volume embedding inference
- Large-scale fine-tuning jobs
- Real-time memory systems with strict latency SLAs

### 4. Colab Integration
**Consumption:** 50 credits per fine-tuning job

Connect your Google Colab account to Xavier2's interface and run fine-tuning jobs directly — using Colab's GPU credits for compute, while SWAL handles orchestration and model management.

**Setup:**
1. Link your Colab account in Xavier2's panel UI (Settings → Integrations → Google Colab)
2. Authorize via OAuth — SWAL never stores your Colab credentials
3. Submit fine-tuning jobs that execute on your Colab runtime
4. Model is imported back into Xavier2 when complete

This gives you Colab's free/paid GPU access combined with Xavier2's model management — without duplicating GPU costs.

---

## 💼 Enterprise Custom Contracts

For large teams, governments, or regulated industries:

- **On-premise deployment** — Xavier2 installed in your private cloud
- **SOC2 / ISO 27001 compliance** — full security documentation
- **Custom SLAs** — 99.99% uptime guarantees available
- **Reserved GPU capacity** — guaranteed H100/A100 slots for your use
- **Direct engineering access** — speak to the people who built it
- **Custom model training** — we'll train and maintain models specifically for your domain

Contact: **enterprise@southwest-ai-labs.com**

---

## 📊 Feature Comparison

| Feature | MIT (Free) | Starter | Pro | Enterprise |
|---------|-----------|---------|-----|------------|
| Xavier2 runtime | ✅ | ✅ | ✅ | ✅ |
| Self-hosting | ✅ | ✅ | ✅ | ✅ |
| Vector + FTS + KG search | ✅ | ✅ | ✅ | ✅ |
| MCP transport | ✅ | ✅ | ✅ | ✅ |
| Basic support | ❌ | ✅ Email | ✅ Priority | ✅ Dedicated |
| Reserved enterprise features | ❌ | ✅ | ✅ | ✅ |
| Fine-tuning credits | ❌ | ❌ | ✅ 500/yr | ✅ Custom |
| Dedicated GPU | ❌ | ❌ | ❌ | ✅ |
| On-premise deployment | ❌ | ❌ | ❌ | ✅ |
| Custom SLA | ❌ | ❌ | ❌ | ✅ |

---

## ❓ FAQ

**Q: Does buying an enterprise license affect my MIT rights?**
A: No. The MIT license is permanent. An enterprise license adds separate commercial terms — it never modifies or restricts your MIT rights.

**Q: Can I use Xavier2 in a commercial product without paying?**
A: Yes — MIT allows commercial use. However, if you're building a product that depends heavily on Xavier2, a commercial subscription is how you support the project and gain priority access to new features.

**Q: How does the credit system work?**
A: Credits are deducted when you use metered services (embedding API, fine-tuning, dedicated GPU). You purchase credit packs upfront and draw down as you use them. Credits expire based on the pack validity period.

**Q: Is my training data used to improve the shared models?**
A: Never. Your data is used exclusively for your private model. SWAL maintains strict data isolation — enterprise customer data is never used in shared training runs.

**Q: What happens if I don't renew my subscription?**
A: Your MIT rights continue indefinitely. Reserved enterprise features deactivate, but the MIT-licensed core remains fully functional. Existing fine-tuned models continue to work on your self-hosted deployment.

---

## 📬 Contact

**Enterprise sales & licensing:** enterprise@southwest-ai-labs.com
**General inquiries:** contact@southwest-ai-labs.com
**Security disclosures:** security@southwest-ai-labs.com

SouthWest AI Labs — Building open source infrastructure for AI agents.
