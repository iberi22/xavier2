$body = @{"content"="test123"; "path"="test/debug"} | ConvertTo-Json
$result = Invoke-WebRequest -Uri 'http://localhost:8003/memory/add' -Method POST -ContentType 'application/json' -Headers @{'X-Xavier2-Token' = 'dev-token'} -Body $body -UseBasicParsing
Write-Host "Status: $($result.StatusCode)"
Write-Host "Body: $($result.Content)"
