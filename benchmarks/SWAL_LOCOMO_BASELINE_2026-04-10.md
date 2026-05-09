# SWAL LoCoMo Benchmark Results

**Date:** 2026-04-10
**Category:** all
**Xavier URL:** http://localhost:8003

---

## Summary

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Total Queries | 22 | - | - |
| Successful | 0 | - | ⚠️ |
| Failed | 22 | 0 | ❌ |
| Avg Latency | 0ms | < 500ms | ✅ |
| Avg Precision | 0/5 | ≥ 4.0 | ⚠️ |
| Avg Recall | 0% | ≥ 80% | ⚠️ |
| Avg FPR | 0% | < 20% | ✅ |

---

## Results by Category

| Category | Count | Latency | Precision | Recall |
|----------|-------|---------|-----------|--------|
| multi_hop | 0 | 0ms | 0/5 | 0% |
| single_hop | 0 | 0ms | 0/5 | 0% |
| temporal | 0 | 0ms | 0/5 | 0% |
| open_domain | 0 | 0ms | 0/5 | 0% |
---

## Detailed Query Results

| ID | Type | Query | Latency | Precision | Recall | FPR | Status |
|----|------|-------|---------|-----------|--------|-----|--------|
| SH-01 | single_hop | What is BELA's timezone? | 0ms | 0/5 | 0% | 0% | ❌ |
| SH-02 | single_hop | What is ManteniApp's pricing? | 0ms | 0/5 | 0% | 0% | ❌ |
| SH-03 | single_hop | Who is Leonardo working with? | 0ms | 0/5 | 0% | 0% | ❌ |
| SH-04 | single_hop | What product are we selling to Rodacenter? | 0ms | 0/5 | 0% | 0% | ❌ |
| SH-05 | single_hop | What is Xavier's current version? | 0ms | 0/5 | 0% | 0% | ❌ |
| SH-06 | single_hop | What is pplx-embed status? | 0ms | 0/5 | 0% | 0% | ❌ |
| SH-07 | single_hop | What is the Tripro demo URL? | 0ms | 0/5 | 0% | 0% | ❌ |
| SH-08 | single_hop | Where should projects be stored? | 0ms | 0/5 | 0% | 0% | ❌ |
| MH-01 | multi_hop | Who worked on Xavier fixes and what decisions w... | 0ms | 0/5 | 0% | 0% | ❌ |
| MH-02 | multi_hop | Find a client interested in maintenance monitor... | 0ms | 0/5 | 0% | 0% | ❌ |
| MH-03 | multi_hop | What projects involve Chile and what is the sta... | 0ms | 0/5 | 0% | 0% | ❌ |
| MH-04 | multi_hop | What are the active SWAL cron jobs? | 0ms | 0/5 | 0% | 0% | ❌ |
| MH-05 | multi_hop | What security measures are active for SWAL? | 0ms | 0/5 | 0% | 0% | ❌ |
| MH-06 | multi_hop | What is the Xavier memory architecture? | 0ms | 0/5 | 0% | 0% | ❌ |
| TR-01 | temporal | When was pplx-embed fixed? | 0ms | 0/5 | 0% | 0% | ❌ |
| TR-02 | temporal | What decisions were made about SurrealDB persis... | 0ms | 0/5 | 0% | 0% | ❌ |
| TR-03 | temporal | What happened in the last session about Xavier? | 0ms | 0/5 | 0% | 0% | ❌ |
| TR-04 | temporal | What is the timeline of Xavier versions? | 0ms | 0/5 | 0% | 0% | ❌ |
| OD-01 | open_domain | Summarize the Xavier memory system improvements... | 0ms | 0/5 | 0% | 0% | ❌ |
| OD-02 | open_domain | What is the overall status of SWAL operations? | 0ms | 0/5 | 0% | 0% | ❌ |
| OD-03 | open_domain | What skills are available for sales operations? | 0ms | 0/5 | 0% | 0% | ❌ |
| OD-04 | open_domain | What is the complete SWAL product portfolio? | 0ms | 0/5 | 0% | 0% | ❌ |
---

## Areas Needing Improvement

### Low Precision Queries (Precision < 3.5):
- **SH-01**: What is BELA's timezone? (Precision: 0/5)
- **SH-02**: What is ManteniApp's pricing? (Precision: 0/5)
- **SH-03**: Who is Leonardo working with? (Precision: 0/5)
- **SH-04**: What product are we selling to Rodacenter? (Precision: 0/5)
- **SH-05**: What is Xavier's current version? (Precision: 0/5)
- **SH-06**: What is pplx-embed status? (Precision: 0/5)
- **SH-07**: What is the Tripro demo URL? (Precision: 0/5)
- **SH-08**: Where should projects be stored? (Precision: 0/5)
- **MH-01**: Who worked on Xavier fixes and what decisions were made? (Precision: 0/5)
- **MH-02**: Find a client interested in maintenance monitoring with AI (Precision: 0/5)
- **MH-03**: What projects involve Chile and what is the status? (Precision: 0/5)
- **MH-04**: What are the active SWAL cron jobs? (Precision: 0/5)
- **MH-05**: What security measures are active for SWAL? (Precision: 0/5)
- **MH-06**: What is the Xavier memory architecture? (Precision: 0/5)
- **TR-01**: When was pplx-embed fixed? (Precision: 0/5)
- **TR-02**: What decisions were made about SurrealDB persistence? (Precision: 0/5)
- **TR-03**: What happened in the last session about Xavier? (Precision: 0/5)
- **TR-04**: What is the timeline of Xavier versions? (Precision: 0/5)
- **OD-01**: Summarize the Xavier memory system improvements made (Precision: 0/5)
- **OD-02**: What is the overall status of SWAL operations? (Precision: 0/5)
- **OD-03**: What skills are available for sales operations? (Precision: 0/5)
- **OD-04**: What is the complete SWAL product portfolio? (Precision: 0/5)

### Low Recall Queries (Recall < 70%):
- **SH-01**: What is BELA's timezone? (Recall: 0%)
- **SH-02**: What is ManteniApp's pricing? (Recall: 0%)
- **SH-03**: Who is Leonardo working with? (Recall: 0%)
- **SH-04**: What product are we selling to Rodacenter? (Recall: 0%)
- **SH-05**: What is Xavier's current version? (Recall: 0%)
- **SH-06**: What is pplx-embed status? (Recall: 0%)
- **SH-07**: What is the Tripro demo URL? (Recall: 0%)
- **SH-08**: Where should projects be stored? (Recall: 0%)
- **MH-01**: Who worked on Xavier fixes and what decisions were made? (Recall: 0%)
- **MH-02**: Find a client interested in maintenance monitoring with AI (Recall: 0%)
- **MH-03**: What projects involve Chile and what is the status? (Recall: 0%)
- **MH-04**: What are the active SWAL cron jobs? (Recall: 0%)
- **MH-05**: What security measures are active for SWAL? (Recall: 0%)
- **MH-06**: What is the Xavier memory architecture? (Recall: 0%)
- **TR-01**: When was pplx-embed fixed? (Recall: 0%)
- **TR-02**: What decisions were made about SurrealDB persistence? (Recall: 0%)
- **TR-03**: What happened in the last session about Xavier? (Recall: 0%)
- **TR-04**: What is the timeline of Xavier versions? (Recall: 0%)
- **OD-01**: Summarize the Xavier memory system improvements made (Recall: 0%)
- **OD-02**: What is the overall status of SWAL operations? (Recall: 0%)
- **OD-03**: What skills are available for sales operations? (Recall: 0%)
- **OD-04**: What is the complete SWAL product portfolio? (Recall: 0%)

---

## Recommendations

1. **Memory Gaps**: Add more memories for queries with low recall
2. **Embedding Quality**: Consider re-indexing memories with low precision scores
3. **Query Patterns**: Some queries may need semantic search improvements
4. **Cache Warming**: Pre-load frequently accessed memories for faster cold starts

---

*Generated by SWAL LoCoMo Benchmark Script*
