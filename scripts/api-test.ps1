$TOKEN = $env:XAVIER_TOKEN
if (-not $TOKEN) { $TOKEN = $env:XAVIER_API_KEY }
if (-not $TOKEN) { $TOKEN = $env:XAVIER_TOKEN }
if (-not $TOKEN) {
    throw "Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN."
}

$body = @{"content"="test123"; "path"="test/debug"} | ConvertTo-Json
$result = Invoke-WebRequest -Uri 'http://localhost:8003/memory/add' -Method POST -ContentType 'application/json' -Headers @{'X-Xavier-Token' = $TOKEN} -Body $body -UseBasicParsing
Write-Host "Status: $($result.StatusCode)"
Write-Host "Body: $($result.Content)"
