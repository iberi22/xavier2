# SWAL LoCoMo Benchmark Results

**Date:** 2026-04-07
**Category:** all
**Xavier2 URL:** http://localhost:8003

---

## Summary

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Total Queries | 22 | - | - |
| Successful | 22 | - | âœ… |
| Failed | 0 | 0 | âœ… |
| Avg Latency | 3568.5ms | < 500ms | âŒ |
| Avg Precision | 3.53/5 | â‰¥ 4.0 | âš ï¸ |
| Avg Recall | 93.4% | â‰¥ 80% | âœ… |
| Avg FPR | 35.5% | < 20% | âš ï¸ |

---

## Results by Category

| Category | Count | Latency | Precision | Recall |
|----------|-------|---------|-----------|--------|
| multi_hop | 6 | 3731.8ms | 3.67/5 | 88.3% |
| single_hop | 8 | 3319ms | 3.42/5 | 93.8% |
| temporal | 4 | 3638.5ms | 3.75/5 | 100% |
| open_domain | 4 | 3752.8ms | 3.3/5 | 93.8% |
---

## Detailed Query Results

| ID | Type | Query | Latency | Precision | Recall | FPR | Status |
|----|------|-------|---------|-----------|--------|-----|--------|
| SH-01 | single_hop | What is BELA's timezone? | 3302ms | 3/5 | 100% | 60% | âœ… |
| SH-02 | single_hop | What is ManteniApp's pricing? | 1960ms | 4.2/5 | 83.3% | 0% | âœ… |
| SH-03 | single_hop | Who is Leonardo working with? | 4605ms | 3.2/5 | 100% | 60% | âœ… |
| SH-04 | single_hop | What product are we selling to Rodacenter? | 4725ms | 3.6/5 | 100% | 40% | âœ… |
| SH-05 | single_hop | What is Xavier2's current version? | 3016ms | 3.2/5 | 100% | 60% | âœ… |
| SH-06 | single_hop | What is pplx-embed status? | 1726ms | 3/5 | 100% | 60% | âœ… |
| SH-07 | single_hop | What is the Tripro demo URL? | 6173ms | 3/5 | 100% | 60% | âœ… |
| SH-08 | single_hop | Where should projects be stored? | 1045ms | 4.2/5 | 66.7% | 20% | âœ… |
| MH-01 | multi_hop | Who worked on Xavier2 fixes and what decisions w... | 9390ms | 4/5 | 100% | 0% | âœ… |
| MH-02 | multi_hop | Find a client interested in maintenance monitor... | 1896ms | 4.4/5 | 80% | 0% | âœ… |
| MH-03 | multi_hop | What projects involve Chile and what is the sta... | 1066ms | 3.6/5 | 75% | 20% | âœ… |
| MH-04 | multi_hop | What are the active SWAL cron jobs? | 1026ms | 3.4/5 | 100% | 40% | âœ… |
| MH-05 | multi_hop | What security measures are active for SWAL? | 932ms | 3.8/5 | 100% | 20% | âœ… |
| MH-06 | multi_hop | What is the Xavier2 memory architecture? | 8081ms | 2.8/5 | 75% | 60% | âœ… |
| TR-01 | temporal | When was pplx-embed fixed? | 1683ms | 3.6/5 | 100% | 40% | âœ… |
| TR-02 | temporal | What decisions were made about SurrealDB persis... | 7901ms | 3.4/5 | 100% | 20% | âœ… |
| TR-03 | temporal | What happened in the last session about Xavier2? | 2401ms | 4.4/5 | 100% | 0% | âœ… |
| TR-04 | temporal | What is the timeline of Xavier2 versions? | 2569ms | 3.6/5 | 100% | 40% | âœ… |
| OD-01 | open_domain | Summarize the Xavier2 memory system improvements... | 8212ms | 2.6/5 | 100% | 80% | âœ… |
| OD-02 | open_domain | What is the overall status of SWAL operations? | 1321ms | 3.6/5 | 100% | 20% | âœ… |
| OD-03 | open_domain | What skills are available for sales operations? | 4504ms | 3.6/5 | 100% | 40% | âœ… |
| OD-04 | open_domain | What is the complete SWAL product portfolio? | 974ms | 3.4/5 | 75% | 40% | âœ… |
---

## Areas Needing Improvement

### Low Precision Queries (Precision < 3.5):
- **SH-01**: What is BELA's timezone? (Precision: 3/5)
- **SH-03**: Who is Leonardo working with? (Precision: 3.2/5)
- **SH-05**: What is Xavier2's current version? (Precision: 3.2/5)
- **SH-06**: What is pplx-embed status? (Precision: 3/5)
- **SH-07**: What is the Tripro demo URL? (Precision: 3/5)
- **MH-04**: What are the active SWAL cron jobs? (Precision: 3.4/5)
- **MH-06**: What is the Xavier2 memory architecture? (Precision: 2.8/5)
- **TR-02**: What decisions were made about SurrealDB persistence? (Precision: 3.4/5)
- **OD-01**: Summarize the Xavier2 memory system improvements made (Precision: 2.6/5)
- **OD-04**: What is the complete SWAL product portfolio? (Precision: 3.4/5)

### Low Recall Queries (Recall < 70%):
- **SH-08**: Where should projects be stored? (Recall: 66.7%)

### High Latency Queries (> 500ms):
- **SH-01**: What is BELA's timezone? (Latency: 3302ms)
- **SH-02**: What is ManteniApp's pricing? (Latency: 1960ms)
- **SH-03**: Who is Leonardo working with? (Latency: 4605ms)
- **SH-04**: What product are we selling to Rodacenter? (Latency: 4725ms)
- **SH-05**: What is Xavier2's current version? (Latency: 3016ms)
- **SH-06**: What is pplx-embed status? (Latency: 1726ms)
- **SH-07**: What is the Tripro demo URL? (Latency: 6173ms)
- **SH-08**: Where should projects be stored? (Latency: 1045ms)
- **MH-01**: Who worked on Xavier2 fixes and what decisions were made? (Latency: 9390ms)
- **MH-02**: Find a client interested in maintenance monitoring with AI (Latency: 1896ms)
- **MH-03**: What projects involve Chile and what is the status? (Latency: 1066ms)
- **MH-04**: What are the active SWAL cron jobs? (Latency: 1026ms)
- **MH-05**: What security measures are active for SWAL? (Latency: 932ms)
- **MH-06**: What is the Xavier2 memory architecture? (Latency: 8081ms)
- **TR-01**: When was pplx-embed fixed? (Latency: 1683ms)
- **TR-02**: What decisions were made about SurrealDB persistence? (Latency: 7901ms)
- **TR-03**: What happened in the last session about Xavier2? (Latency: 2401ms)
- **TR-04**: What is the timeline of Xavier2 versions? (Latency: 2569ms)
- **OD-01**: Summarize the Xavier2 memory system improvements made (Latency: 8212ms)
- **OD-02**: What is the overall status of SWAL operations? (Latency: 1321ms)
- **OD-03**: What skills are available for sales operations? (Latency: 4504ms)
- **OD-04**: What is the complete SWAL product portfolio? (Latency: 974ms)

---

## Recommendations

1. **Memory Gaps**: Add more memories for queries with low recall
2. **Embedding Quality**: Consider re-indexing memories with low precision scores
3. **Query Patterns**: Some queries may need semantic search improvements
4. **Cache Warming**: Pre-load frequently accessed memories for faster cold starts

---

*Generated by SWAL LoCoMo Benchmark Script*
