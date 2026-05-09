function Get-XavierToken {
    $token = $env:XAVIER_TOKEN
    if (-not $token) { $token = $env:XAVIER_API_KEY }
    if (-not $token) { $token = $env:XAVIER_TOKEN }
    if (-not $token) {
        throw "Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN."
    }
    return $token
}

$headers = @{
    "X-Xavier-Token" = Get-XavierToken
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
