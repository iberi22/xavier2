---
title: "FEAT: Semantic Cache Integration for System3 (Tier 1 Savings)"
labels:
  - ai-plan
  - enhancement
  - performance
assignees: []
---

## Description
Integrate the existing `SemanticCache` module directly into the `System3Actor` to intercept duplicate incoming queries and drastically reduce redundant LLM calls, achieving Tier 1 cost savings for repeated analytical queries.

### Background
The `SemanticCache` subsystem was developed in `src/memory/semantic_cache.rs` to measure embedding similarity for incoming queries. However, it is not currently wired into `System3Actor::generate_response()`. As a result, 100% identical questions currently trigger full multi-hop RAG retrieval and costly System3 LLM generation.

### Acceptance Criteria
- [ ] Modify `src/agents/system3.rs` to intercept `query` before `self.llm_client.generate_response()` is executed.
- [ ] Vectorize the query and check `SemanticCache` for a similarity score `> 0.95`.
- [ ] If a cache hit occurs, instantly return the cached response (bypassing the model entirely) and log a successful hit.
- [ ] If a cache miss occurs, proceed with the normal generation flow, but **store the LLM's final response into the cache** with `SemanticCache::set()` before returning it to the user.
- [ ] Add a metric or log tracing `[CACHE HIT]` vs `[CACHE MISS]`.

### Technical Notes for the Agent
- Use `crate::memory::semantic_cache::SemanticCache` instance globally or pass it via `ActorConfig`.
- Remember to serialize `ActionResult` safely before caching if you decide to cache full actions, though caching the raw `String` response is also acceptable for v1 execution.
