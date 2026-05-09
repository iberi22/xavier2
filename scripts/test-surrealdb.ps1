$TS = Get-Date -Format "yyyyMMdd_HHmmss"
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

$body = @{
    content = "SURREALDB_TEST_$TS"
    metadata = @{
        source = "final-test"
    }
} | ConvertTo-Json

$R = Invoke-RestMethod -Method Post -Uri "http://localhost:8003/memory/add" -ContentType "application/json" -Headers @{"X-Xavier-Token"=$TOKEN} -Body $body
Write-Output "Response: $($R | ConvertTo-Json -Depth 3)"

Start-Sleep 5

Write-Output "`nVerificando persistencia..."
$MEMS = Invoke-RestMethod -Uri "http://localhost:8003/v1/memories?limit=200" -Headers @{"X-Xavier-Token"=$TOKEN}
$MEMS.memories | Where-Object { $_.content -like "*SURREALDB_TEST*$TS*" } | ForEach-Object { Write-Output "FOUND: $($_.content)" }
