---
title: "FEAT: Context Router for LLM Model Selection"
labels:
  - ai-plan
  - enhancement
  - architecture
assignees: []
---

## Description
Develop a dynamic `Context Router` in `src/agents/router.rs` that classifies incoming user queries based on their cognitive complexity in order to dynamically select the cheapest, most efficient LLM model needed to resolve the request.

### Background
Currently, Xavier2 routes all queries indiscriminately through the same heavy System3 LLM path. The objective is to achieve massive cost savings by identifying queries that do not require multi-hop reasoning or vast context token consumption and offloading them to faster, cheaper models.

### Proposed Architecture
Create `src/agents/router.rs` with a router that acts as the entry point before `System1` (Retrieval) and `System3` (Generation).

The Router must classify the query into one of three buckets:
1. **Direct**: Queries that require zero context and can be answered from cache or pre-prompt (e.g. "Hello").
2. **Retrieved**: Queries that require simple facts and 1-hop context retrieval. Assign to lower-tier models (e.g., Claude 3 Haiku / GPT-4o-mini).
3. **Complex**: Queries requiring System2 reflection, multi-hop temporal logic, or extensive synthesis. Assign to elite models (e.g., Claude 3.5 Sonnet / GPT-4o).

### Acceptance Criteria
- [ ] Create `src/agents/router.rs`.
- [ ] Implement `Router::classify(query: &str) -> RouteCategory`.
- [ ] Modify `AgentRuntime` (or `system3.rs` Configuration) to accept dynamic model injection based on the `RouteCategory`.
- [ ] Create simple unit tests proving the Router can heuristically or semantically identify simple vs complex questions.

### Technical Notes for the Agent
- Keep the classification rapid. Do not use an expensive LLM call just to classify the query, as that defeats the purpose of cost savings. Use lightweight local NLP heuristics (keyword complexity, sentence length, named entity counting) or the quickest model tier for parsing.
