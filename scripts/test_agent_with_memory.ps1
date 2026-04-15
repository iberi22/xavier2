$headers = @{
    "X-Xavier2-Token" = "dev-token"
    "Content-Type" = "application/json"
}

# Reset first
$null = Invoke-RestMethod -Uri "http://localhost:8006/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}' -UseBasicParsing -TimeoutSec 10
Write-Host "Reset complete"

# Add memory
$body = @{
    path = "test/cost_memory"
    content = "Monthly cost is 199 dollars for pro tier and 499 dollars for enterprise tier"
    metadata = @{}
} | ConvertTo-Json -Compress
$null = Invoke-RestMethod -Uri "http://localhost:8006/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $body -UseBasicParsing -TimeoutSec 10
Write-Host "Memory added"

Start-Sleep 300

# Query
$queryBody = @{
    query = "What are the monthly costs?"
    limit = 3
    system3_mode = "disabled"
} | ConvertTo-Json -Compress

try {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $resp = Invoke-RestMethod -Uri "http://localhost:8006/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body $queryBody -TimeoutSec 30
    $sw.Stop()
    Write-Host "Query took:" $sw.ElapsedMilliseconds "ms"
    Write-Host "Response:" $resp.response
} catch {
    Write-Host "Error:" $_.Exception.Message
}
