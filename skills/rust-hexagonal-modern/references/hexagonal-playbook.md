# Hexagonal Playbook

Use this reference when mapping code into layers or refactoring an existing Rust module that has started to blur boundaries.

## Target structure

```text
src/
  domain/         # Entities, value objects, invariants, pure rules
  application/    # Use cases, orchestration, policy
  ports/          # Traits and contracts required by the application/domain
  adapters/       # HTTP, CLI, DB, filesystem, remote API implementations
  bootstrap/      # Wiring, config loading, composition root
```

Adjust names to the repo, but keep the dependency direction: adapters depend inward.

## Layer responsibilities

### Domain

- Keep it pure and deterministic.
- Encode invariants and value-level validation here.
- Avoid direct knowledge of HTTP, SQL, environment variables, tracing layers, or transport serialization.

### Application

- Orchestrate use cases.
- Depend on ports, not concrete adapters.
- Own transaction and workflow sequencing decisions.
- Emit domain outputs or application DTOs that adapters can translate.

### Ports

- Define what the application needs from the outside world.
- Keep signatures focused on domain concepts, not vendor payloads.
- If async object safety becomes awkward, prefer one of these:
  - keep the port synchronous and move async I/O outside it,
  - use concrete adapter injection where dynamic dispatch is unnecessary,
  - or use an explicit future-returning contract only where the abstraction truly pays off.

### Adapters

- Translate HTTP, CLI, database, filesystem, and third-party API details into port calls.
- Keep vendor-specific DTOs and serialization here.
- Apply middleware, retries, connection builders, and persistence mappings here.

### Bootstrap

- Build `AppState`, shared clients, and adapter instances in one place.
- Inject ports into use cases explicitly.
- Keep feature flags and environment parsing out of the domain.

## Recommended request flow

```text
HTTP/CLI/TUI input
  -> adapter DTO
  -> application command/query
  -> use case
  -> outbound port
  -> adapter implementation
  -> application result
  -> response mapper
```

## Error boundaries

- Domain errors describe invariant failures and business meaning.
- Adapter errors describe transport or infrastructure failures.
- Application errors compose the two when the use case needs a stable outward contract.
- HTTP or CLI adapters map those errors into status codes, exit codes, and user-facing messages.

## Testing split

- Domain tests: pure, fast, no I/O.
- Port contract tests: verify any adapter obeys the same behavioral contract.
- Adapter integration tests: run real persistence or external integration behavior.
- End-to-end tests: verify the full boundary from input to observable output.

## Refactor triggers

Refactor toward hexagonal boundaries when you see any of these:

- `axum::Json`, `reqwest::Client`, `rusqlite::Connection`, or filesystem paths flowing into domain types
- Business rules encoded in handlers, SQL mappers, or UI callbacks
- One module both validating requests and performing remote I/O
- Repeated status-code mapping or retry logic inside use cases
- Tests that require the full app stack to validate simple domain logic
