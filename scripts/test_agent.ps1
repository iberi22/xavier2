$headers = @{
    "X-Xavier2-Token" = "dev-token"
    "Content-Type" = "application/json"
}

$body = @{
    query = "What is the price?"
    limit = 3
    system3_mode = "disabled"
} | ConvertTo-Json -Compress

try {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $resp = Invoke-RestMethod -Uri "http://localhost:8006/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body $body -TimeoutSec 30
    $sw.Stop()
    Write-Host "Query took:" $sw.ElapsedMilliseconds "ms"
    Write-Host "Response:" $resp.response
} catch {
    Write-Host "Error:" $_.Exception.Message
}
