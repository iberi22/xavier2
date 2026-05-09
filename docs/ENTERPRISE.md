# Xavier Enterprise Edition

Xavier is available as an MIT-licensed open-source runtime and as a commercial enterprise offering from SouthWest AI Labs.

## Open Source vs Enterprise

The MIT edition provides:

- the open-source Rust runtime;
- local and self-hosted deployment;
- HTTP and MCP interfaces;
- standard memory retrieval and code indexing;
- community-driven usage without paid support commitments.

The enterprise edition adds:

- proprietary retrieval enhancements;
- commercial support and escalation paths;
- private integrations and audit features;
- roadmap prioritization tied to customer agreements.

## Reserved Enterprise Features

### Advanced RRF algorithms

The public runtime ships with the standard RRF baseline. Enterprise deployments may include weighted fusion, private reranking logic, and customer-tuned scoring profiles beyond the standard `k=60` behavior.

### Proprietary embedding models

When `XAVIER_EMBEDDING_MODEL=enterprise`, enterprise customers may use non-public embedding providers, private routing logic, and tuned model variants delivered by SWAL under commercial agreement.

### Advanced audit and logging

Enterprise deployments may include compliance-grade audit exports, longer retention, richer access telemetry, and support-assisted diagnostics not included in the MIT repository.

### Priority support queue integration

Enterprise support plans may include response-time targets, private escalation channels, and direct priority queue handling for incidents and feature requests.

## Pricing Tiers

| Tier | Monthly Price | Intended Use | Support |
|------|---------------|--------------|---------|
| Starter | $49 per workspace | Small teams validating Xavier in production-adjacent workflows | Email support, best effort |
| Pro | $499 per workspace | Teams needing faster response times and commercial guidance | Priority support queue |
| Enterprise | Custom | Organizations needing private enhancements, audit controls, and contractual SLAs | Dedicated support plan |

## Feature Comparison

| Capability | Open Source (MIT) | Enterprise |
|------------|-------------------|------------|
| Core runtime | Yes | Yes |
| Self-hosting | Yes | Yes |
| Standard HTTP and MCP APIs | Yes | Yes |
| Standard RRF retrieval | Yes | Yes |
| Advanced RRF scoring | No | Yes |
| Proprietary embedding models | No | Yes |
| Advanced audit and logging | No | Yes |
| Priority support queue | No | Yes |
| Private enhancements | No | Yes |
| Roadmap priority access | No | Yes |

## Commercial Contact

For enterprise licensing, support, and private deployment discussions:

`enterprise@southwest-ai-labs.com`
