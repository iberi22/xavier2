# Xavier Context Compressor (Experimental)
# Part of the agentic-memory-ops skill

param (
    [string]$Query = "current project status",
    [int]$Limit = 5
)

$TOKEN = "dev-token"
$URL = "http://localhost:8003/memory/search"

Write-Host "Xavier Context Compressor: Retrieving context for '$Query'..." -ForegroundColor Cyan

$Headers = @{
    "X-Xavier-Token" = $TOKEN
    "Content-Type" = "application/json"
}

$Payload = @{
    query = $Query
    limit = $Limit
} | ConvertTo-Json

try {
    $Response = Invoke-RestMethod -Uri $URL -Method Post -Headers $Headers -Body $Payload
    
    if ($Response.count -gt 0) {
        Write-Host "Found $($Response.count) relevant memories. Synthesizing..." -ForegroundColor Green
        
        $CompressedContext = "--- XAVIER REGENERATED CONTEXT ---`n"
        foreach ($res in $Response.results) {
            $CompressedContext += "[Memory ID: $($res.id)]`n$($res.content)`n`n"
        }
        $CompressedContext += "--- END OF CONTEXT ---"
        
        return $CompressedContext
    } else {
        Write-Host "No relevant memories found in Xavier." -ForegroundColor Yellow
        return ""
    }
} catch {
    Write-Host "Error connecting to Xavier: $_" -ForegroundColor Red
    return ""
}
