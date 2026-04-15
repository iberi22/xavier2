$TS = Get-Date -Format "yyyyMMdd_HHmmss"
Write-Output "Timestamp: $TS"

$body = @{
    content = "SURREALDB_TEST_$TS"
    metadata = @{
        source = "final-test"
    }
} | ConvertTo-Json

$R = Invoke-RestMethod -Method Post -Uri "http://localhost:8003/memory/add" -ContentType "application/json" -Headers @{"X-Xavier2-Token"="dev-token"} -Body $body
Write-Output "Response: $($R | ConvertTo-Json -Depth 3)"

Start-Sleep 5

Write-Output "`nVerificando persistencia..."
$MEMS = Invoke-RestMethod -Uri "http://localhost:8003/v1/memories?limit=200" -Headers @{"X-Xavier2-Token"="dev-token"}
$MEMS.memories | Where-Object { $_.content -like "*SURREALDB_TEST*$TS*" } | ForEach-Object { Write-Output "FOUND: $($_.content)" }
