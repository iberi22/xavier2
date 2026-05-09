# ADR Decision Maker Prompt

## Purpose
Guide AI agents through structured Architecture Decision Record (ADR) creation process.

## When to Use
- When a major technical or business decision needs documentation
- Before starting significant architectural work
- When blocking issues require resolution
- During retrospectives and post-mortems

## ADR Structure

```
## ADR-{number}: {Title}

**Date:** {YYYY-MM-DD}
**Status:** PROPOSED | ACCEPTED | DEPRECATED
**Decisor:** {Name/Role}
**Priority:** HIGH | MEDIUM | LOW
**Project:** {Project Name}
**Blockers:** {List of blockers if any}
**Deadline:** {YYYY-MM-DD if applicable}

### Context
{Problem description, background, constraints}

### Options Considered
1. **{Option A}:** {Description}
   - Pros: ...
   - Cons: ...

2. **{Option B}:** {Description}
   - Pros: ...
   - Cons: ...

3. **{Option C}:** {Description}
   - Pros: ...
   - Cons: ...

### Decision
{What was decided and why}

### Consequences
#### Positive
- {List positive outcomes}

#### Negative
- {List negative outcomes or trade-offs}

### Notes
{Additional context, links, follow-up items}
```

## Decision Criteria

Always evaluate options against:
1. **Business Impact** - Revenue, growth, costs
2. **Technical Feasibility** - Effort, complexity, risk
3. **Time to Value** - How quickly does it deliver?
4. **Maintainability** - Long-term support burden
5. **Strategic Alignment** - Fits company direction?

## Priority Guidelines

| Priority | Criteria |
|----------|----------|
| HIGH | Blocks core functionality or significant revenue |
| MEDIUM | Improves efficiency or enables new features |
| LOW | Nice-to-have, can be deferred |

## Status Lifecycle

```
PROPOSED → ACCEPTED → DEPRECATED
    ↓          ↓
 (rejected - archived)
```

## Storage
Store all ADRs in:
- **Xavier path:** `sweat-operations/decisions/{id}`
- **Local fallback:** `E:\scripts-python\SWAL-Operations-Dashboard\decisions/`

## Example Decision Questions

**Architecture:**
- "Should we use microservices or modular monolith?"
- "Which database fits our use case?"
- "How should services communicate?"

**Business:**
- "What pricing model should we use?"
- "Which market to enter first?"
- "Build vs buy for component X?"

**Technical:**
- "What language/framework for new service?"
- "How to handle authentication?"
- "API design approach?"

## Agent Behavior

1. **Identify** the decision that needs to be made
2. **Gather** context and constraints
3. **List** at least 3 viable options with trade-offs
4. **Recommend** the best option with rationale
5. **Document** consequences honestly
6. **Store** in Xavier with proper path
7. **Track** blockers and deadlines

## Quality Checklist

- [ ] Clear problem statement in Context
- [ ] At least 3 realistic options considered
- [ ] Trade-offs explicitly stated
- [ ] Decision includes "why" not just "what"
- [ ] Consequences include both positive AND negative
- [ ] Priority matches impact assessment
- [ ] Stored in correct Xavier path
