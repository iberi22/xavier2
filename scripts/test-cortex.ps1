$body = @{query="SWAL"; limit=5} | ConvertTo-Json -Compress
$temp = [System.IO.Path]::GetTempFileName() + ".json"
[System.IO.File]::WriteAllText($temp, $body)
curl.exe -s -X POST "http://localhost:8003/memory/search" -H "X-Xavier2-Token: dev-token" -H "Content-Type: application/json" --data-binary "@$temp"
Remove-Item $temp
