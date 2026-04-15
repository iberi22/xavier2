# SurrealDB Fix Mission
$PROMPT = @"
You are a senior Rust developer. Your mission: fix the SurrealDB persistence issue in this Xavier2 project at E:\scripts-python\xavier2.

## Current Problem
SurrealDB was disabled because WebSocket writes appeared to succeed but data was NOT persisted to disk. The system is currently using FileMemoryStore as fallback.

## Your Tasks

### 1. Update SurrealDB crate
Read Cargo.toml and find the current surrealdb version. Check crates.io for the latest stable version and update if needed.

### 2. Investigate why SurrealDB writes don't persist
Read src/memory/surreal_store.rs and docker-compose.yml. Diagnose if the issue is:
- Authentication, protocol version mismatch, data format, namespace not existing, or WebSocket vs HTTP

### 3. Apply fix
Fix whatever is broken. If the crate update requires API changes, update the code. If it's a config issue, fix docker-compose.yml.

### 4. Test persistence FOR REAL
After any fix, run this:

```powershell
`$TS = Get-Date -Format yyyyMMdd_HHmmss
curl -H "X-Xavier2-Token: dev-token" -Method POST "http://localhost:8003/memory/add" -ContentType "application/json" -Body "{`"content`": `"SURREALDB_TEST_`$TS`", `"metadata`": {`"source`": `"fix`"}}"
docker compose -f E:\scripts-python\xavier2\docker-compose.yml down
docker compose -f E:\scripts-python\xavier2\docker-compose.yml up -d
Start-Sleep 10
curl -H "X-Xavier2-Token: dev-token" "http://localhost:8003/v1/memories" | Select-String "SURREALDB_TEST_`$TS"
```

### 5. If SurrealDB works, enable it back as the primary backend
### 6. If it still doesn't work, document thoroughly in SURREALDB_INVESTIGATION.md

## Constraints
- NEVER move the project from E:\scripts-python\xavier2
- Keep FileMemoryStore as fallback
- Back up files before modifying

When done, run: openclaw system event --text "Done: SurrealDB fix attempt finished" --mode now
"@

Set-Content -Path "E:\scripts-python\xavier2\task-prompt.txt" -Value $PROMPT
