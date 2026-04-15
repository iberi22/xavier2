$TS = Get-Date -Format "yyyyMMdd_HHmmss"
Write-Output "=== PERSISTENCE TEST ==="
Write-Output "Timestamp: $TS"

# Add memory
$body = @{
    content = "SURREALDB_PERSIST_TEST_$TS"
    metadata = @{ source = "persist-final-test" }
} | ConvertTo-Json

$R = Invoke-RestMethod -Method Post -Uri "http://localhost:8003/memory/add" -ContentType "application/json" -Headers @{"X-Xavier2-Token"="dev-token"} -Body $body
Write-Output "Added: $($R.status) - $($R.message)"

Start-Sleep 3

# Verify it exists
Write-Output "`n--- Before restart ---"
$MEMS = Invoke-RestMethod -Uri "http://localhost:8003/v1/memories?limit=200" -Headers @{"X-Xavier2-Token"="dev-token"}
$FOUND_BEFORE = $MEMS.memories | Where-Object { $_.content -like "*SURREALDB_PERSIST_TEST*$TS*" }
Write-Output "Found before restart: $($FOUND_BEFORE.content)"

# Restart xavier2
Write-Output "`n--- Restarting xavier2 ---"
docker stop xavier2
Start-Sleep 3
docker start xavier2
Start-Sleep 20

# Check health
try {
    $H = Invoke-RestMethod -Uri "http://localhost:8003/health" -Headers @{"X-Xavier2-Token"="dev-token"} -TimeoutSec 5
    Write-Output "Health after restart: $($H | ConvertTo-Json)"
} catch {
    Write-Output "Health check failed: $_"
}

# Verify persists
Write-Output "`n--- After restart ---"
$MEMS = Invoke-RestMethod -Uri "http://localhost:8003/v1/memories?limit=200" -Headers @{"X-Xavier2-Token"="dev-token"}
$FOUND_AFTER = $MEMS.memories | Where-Object { $_.content -like "*SURREALDB_PERSIST_TEST*$TS*" }
if ($FOUND_AFTER) {
    Write-Output "SUCCESS! Memory persisted: $($FOUND_AFTER.content)"
} else {
    Write-Output "FAILED! Memory NOT found after restart"
}
