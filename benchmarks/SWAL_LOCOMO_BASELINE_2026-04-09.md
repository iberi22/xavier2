# SWAL LoCoMo Benchmark Results

**Date:** 2026-04-09
**Category:** all
**Xavier2 URL:** http://localhost:8003

---

## Summary

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Total Queries | 22 | - | - |
| Successful | 22 | - | âœ… |
| Failed | 0 | 0 | âœ… |
| Avg Latency | 977.1ms | < 500ms | âŒ |
| Avg Precision | 4.15/5 | â‰¥ 4.0 | âœ… |
| Avg Recall | 96.8% | â‰¥ 80% | âœ… |
| Avg FPR | 15.5% | < 20% | âœ… |

---

## Results by Category

| Category | Count | Latency | Precision | Recall |
|----------|-------|---------|-----------|--------|
| multi_hop | 6 | 966.5ms | 4/5 | 92.5% |
| single_hop | 8 | 998.5ms | 3.92/5 | 100% |
| temporal | 4 | 964.8ms | 4.65/5 | 100% |
| open_domain | 4 | 962.8ms | 4.35/5 | 93.8% |
---

## Detailed Query Results

| ID | Type | Query | Latency | Precision | Recall | FPR | Status |
|----|------|-------|---------|-----------|--------|-----|--------|
| SH-01 | single_hop | What is BELA's timezone? | 813ms | 4.4/5 | 100% | 0% | âœ… |
| SH-02 | single_hop | What is ManteniApp's pricing? | 896ms | 3.2/5 | 100% | 20% | âœ… |
| SH-03 | single_hop | Who is Leonardo working with? | 1052ms | 2.6/5 | 100% | 80% | âœ… |
| SH-04 | single_hop | What product are we selling to Rodacenter? | 974ms | 3.8/5 | 100% | 40% | âœ… |
| SH-05 | single_hop | What is Xavier2's current version? | 959ms | 5/5 | 100% | 0% | âœ… |
| SH-06 | single_hop | What is pplx-embed status? | 1481ms | 4.4/5 | 100% | 20% | âœ… |
| SH-07 | single_hop | What is the Tripro demo URL? | 1026ms | 3/5 | 100% | 60% | âœ… |
| SH-08 | single_hop | Where should projects be stored? | 787ms | 5/5 | 100% | 0% | âœ… |
| MH-01 | multi_hop | Who worked on Xavier2 fixes and what decisions w... | 999ms | 5/5 | 100% | 0% | âœ… |
| MH-02 | multi_hop | Find a client interested in maintenance monitor... | 1206ms | 3.4/5 | 80% | 0% | âœ… |
| MH-03 | multi_hop | What projects involve Chile and what is the sta... | 966ms | 3.8/5 | 100% | 40% | âœ… |
| MH-04 | multi_hop | What are the active SWAL cron jobs? | 898ms | 4.4/5 | 100% | 20% | âœ… |
| MH-05 | multi_hop | What security measures are active for SWAL? | 829ms | 4/5 | 75% | 0% | âœ… |
| MH-06 | multi_hop | What is the Xavier2 memory architecture? | 901ms | 3.4/5 | 100% | 40% | âœ… |
| TR-01 | temporal | When was pplx-embed fixed? | 1016ms | 4.8/5 | 100% | 0% | âœ… |
| TR-02 | temporal | What decisions were made about SurrealDB persis... | 888ms | 4.6/5 | 100% | 0% | âœ… |
| TR-03 | temporal | What happened in the last session about Xavier2? | 844ms | 4.8/5 | 100% | 0% | âœ… |
| TR-04 | temporal | What is the timeline of Xavier2 versions? | 1111ms | 4.4/5 | 100% | 0% | âœ… |
| OD-01 | open_domain | Summarize the Xavier2 memory system improvements... | 881ms | 4/5 | 100% | 20% | âœ… |
| OD-02 | open_domain | What is the overall status of SWAL operations? | 969ms | 3.4/5 | 75% | 0% | âœ… |
| OD-03 | open_domain | What skills are available for sales operations? | 1003ms | 5/5 | 100% | 0% | âœ… |
| OD-04 | open_domain | What is the complete SWAL product portfolio? | 998ms | 5/5 | 100% | 0% | âœ… |
---

## Areas Needing Improvement

### Low Precision Queries (Precision < 3.5):
- **SH-02**: What is ManteniApp's pricing? (Precision: 3.2/5)
- **SH-03**: Who is Leonardo working with? (Precision: 2.6/5)
- **SH-07**: What is the Tripro demo URL? (Precision: 3/5)
- **MH-02**: Find a client interested in maintenance monitoring with AI (Precision: 3.4/5)
- **MH-06**: What is the Xavier2 memory architecture? (Precision: 3.4/5)
- **OD-02**: What is the overall status of SWAL operations? (Precision: 3.4/5)

### High Latency Queries (> 500ms):
- **SH-01**: What is BELA's timezone? (Latency: 813ms)
- **SH-02**: What is ManteniApp's pricing? (Latency: 896ms)
- **SH-03**: Who is Leonardo working with? (Latency: 1052ms)
- **SH-04**: What product are we selling to Rodacenter? (Latency: 974ms)
- **SH-05**: What is Xavier2's current version? (Latency: 959ms)
- **SH-06**: What is pplx-embed status? (Latency: 1481ms)
- **SH-07**: What is the Tripro demo URL? (Latency: 1026ms)
- **SH-08**: Where should projects be stored? (Latency: 787ms)
- **MH-01**: Who worked on Xavier2 fixes and what decisions were made? (Latency: 999ms)
- **MH-02**: Find a client interested in maintenance monitoring with AI (Latency: 1206ms)
- **MH-03**: What projects involve Chile and what is the status? (Latency: 966ms)
- **MH-04**: What are the active SWAL cron jobs? (Latency: 898ms)
- **MH-05**: What security measures are active for SWAL? (Latency: 829ms)
- **MH-06**: What is the Xavier2 memory architecture? (Latency: 901ms)
- **TR-01**: When was pplx-embed fixed? (Latency: 1016ms)
- **TR-02**: What decisions were made about SurrealDB persistence? (Latency: 888ms)
- **TR-03**: What happened in the last session about Xavier2? (Latency: 844ms)
- **TR-04**: What is the timeline of Xavier2 versions? (Latency: 1111ms)
- **OD-01**: Summarize the Xavier2 memory system improvements made (Latency: 881ms)
- **OD-02**: What is the overall status of SWAL operations? (Latency: 969ms)
- **OD-03**: What skills are available for sales operations? (Latency: 1003ms)
- **OD-04**: What is the complete SWAL product portfolio? (Latency: 998ms)

---

## Recommendations

1. **Memory Gaps**: Add more memories for queries with low recall
2. **Embedding Quality**: Consider re-indexing memories with low precision scores
3. **Query Patterns**: Some queries may need semantic search improvements
4. **Cache Warming**: Pre-load frequently accessed memories for faster cold starts

---

*Generated by SWAL LoCoMo Benchmark Script*
