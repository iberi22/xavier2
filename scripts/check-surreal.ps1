# Use the surreal CLI to check data directly
# Use the 'is-ready' subcommand to verify SurrealDB is running
Write-Host "=== Check if SurrealDB is ready ==="
$r1 = docker exec surrealdb /surreal is-ready --endpoint ws://localhost:8000 2>&1
Write-Host "is-ready: $r1"

# Try using the 'version' subcommand
Write-Host ""
Write-Host "=== SurrealDB version ==="
$r2 = docker exec surrealdb /surreal version 2>&1
Write-Host "Version: $r2"

# Now try to check if we can use the sql REPL with a script
# First, write SQL commands to a file
$sqlContent = @"
SELECT * FROM memory_records;
EXIT
"@
$sqlContent | Out-File -FilePath "E:\scripts-python\xavier\query.sql" -Encoding ascii
docker cp "E:\scripts-python\xavier\query.sql" surrealdb:/tmp/query.sql

Write-Host ""
Write-Host "=== Try surreal sql with file input ==="
try {
    $result = docker exec surrealdb sh -c '/surreal sql --endpoint ws://localhost:8000 --namespace xavier --database memory --username root --password root < /tmp/query.sql 2>&1 | head -30'
    Write-Host "Result: $result"
} catch {
    Write-Host "Error: $_"
}

# Alternative: check via the HTTP API with proper authentication
Write-Host ""
Write-Host "=== Try HTTP API with NS header ==="
$body = @"
{"sql":"SELECT count() FROM memory_records","bindings":{}}
"@
$body | Out-File -FilePath "E:\scripts-python\xavier\query_body.json" -Encoding ascii -NoNewline
docker cp "E:\scripts-python\xavier\query_body.json" xavier:/tmp/query_body.json

$r3 = docker exec xavier sh -c 'curl -s http://surrealdb:8000/sql -X POST -H "Content-Type: application/surrealdb" -H "NS: xavier" -H "DB: memory" -u root:root -d @/tmp/query_body.json 2>&1'
Write-Host "SQL via HTTP: $r3"
