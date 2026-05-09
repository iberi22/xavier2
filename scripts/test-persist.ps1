$TS = Get-Date -Format "yyyyMMdd_HHmmss"
Write-Output "=== PERSISTENCE TEST ==="
Write-Output "Timestamp: $TS"

function Get-XavierToken {
    $token = $env:XAVIER_TOKEN
    if (-not $token) { $token = $env:XAVIER_API_KEY }
    if (-not $token) { $token = $env:XAVIER_TOKEN }
    if (-not $token) {
        throw "Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN."
    }
    return $token
}

$TOKEN = Get-XavierToken

# Add memory
$body = @{
    content = "SURREALDB_PERSIST_TEST_$TS"
    metadata = @{ source = "persist-final-test" }
} | ConvertTo-Json

$R = Invoke-RestMethod -Method Post -Uri "http://localhost:8003/memory/add" -ContentType "application/json" -Headers @{"X-Xavier-Token"=$TOKEN} -Body $body
Write-Output "Added: $($R.status) - $($R.message)"

Start-Sleep 3

# Verify it exists
Write-Output "`n--- Before restart ---"
$MEMS = Invoke-RestMethod -Uri "http://localhost:8003/v1/memories?limit=200" -Headers @{"X-Xavier-Token"=$TOKEN}
$FOUND_BEFORE = $MEMS.memories | Where-Object { $_.content -like "*SURREALDB_PERSIST_TEST*$TS*" }
Write-Output "Found before restart: $($FOUND_BEFORE.content)"

# Restart xavier
Write-Output "`n--- Restarting xavier ---"
docker stop xavier
Start-Sleep 3
docker start xavier
Start-Sleep 20

# Check health
try {
    $H = Invoke-RestMethod -Uri "http://localhost:8003/health" -Headers @{"X-Xavier-Token"=$TOKEN} -TimeoutSec 5
    Write-Output "Health after restart: $($H | ConvertTo-Json)"
} catch {
    Write-Output "Health check failed: $_"
}

# Verify persists
Write-Output "`n--- After restart ---"
$MEMS = Invoke-RestMethod -Uri "http://localhost:8003/v1/memories?limit=200" -Headers @{"X-Xavier-Token"=$TOKEN}
$FOUND_AFTER = $MEMS.memories | Where-Object { $_.content -like "*SURREALDB_PERSIST_TEST*$TS*" }
if ($FOUND_AFTER) {
    Write-Output "SUCCESS! Memory persisted: $($FOUND_AFTER.content)"
} else {
    Write-Output "FAILED! Memory NOT found after restart"
}
