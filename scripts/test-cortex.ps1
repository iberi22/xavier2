$body = @{query="SWAL"; limit=5} | ConvertTo-Json -Compress
$temp = [System.IO.Path]::GetTempFileName() + ".json"
[System.IO.File]::WriteAllText($temp, $body)
$TOKEN = $env:XAVIER_TOKEN
if (-not $TOKEN) { $TOKEN = $env:XAVIER_API_KEY }
if (-not $TOKEN) { $TOKEN = $env:XAVIER_TOKEN }
if (-not $TOKEN) {
    throw "Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN."
}
curl.exe -s -X POST "http://localhost:8003/memory/search" -H "X-Xavier-Token: $TOKEN" -H "Content-Type: application/json" --data-binary "@$temp"
Remove-Item $temp
