# Xavier Client - Simple test script
# Usage: .\xavier_client.ps1 <command> [args]

param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("health", "build", "add", "search", "query", "migrate", "test")]
    [string]$Command,
    [string]$Path = "",
    [string]$Content = "",
    [string]$Query = ""
)

$XAVIER = "http://localhost:8006"
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
$headers = @{"X-Xavier-Token" = $TOKEN}

function Show-Health {
    $r = Invoke-RestMethod -Uri "$XAVIER/health" -Headers $headers
    $r | ConvertTo-Json
}

function Show-Build {
    $r = Invoke-RestMethod -Uri "$XAVIER/build" -Headers $headers
    $r | ConvertTo-Json -Depth 3
}

function Add-Memory {
    if (-not $Content) {
        Write-Host "Error: -Content required for add command" -ForegroundColor Red
        return
    }

    $payload = @{
        path = if ($Path) { $Path } else { "test/$(Get-Random)" }
        content = $Content
        metadata = @{}
    }

    $body = $payload | ConvertTo-Json -Compress
    $r = Invoke-RestMethod -Uri "$XAVIER/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $body
    $r | ConvertTo-Json
}

function Search-Memory {
    if (-not $Query) {
        Write-Host "Error: -Query required for search command" -ForegroundColor Red
        return
    }

    $payload = @{
        query = $Query
        limit = 10
    }

    $body = $payload | ConvertTo-Json -Compress
    $r = Invoke-RestMethod -Uri "$XAVIER/memory/search" -Method POST -Headers $headers -ContentType "application/json" -Body $body

    if ($r.status -eq "ok") {
        Write-Host "Found $($r.results.Count) results:" -ForegroundColor Green
        foreach ($result in $r.results) {
            Write-Host ""
            Write-Host "  Path: $($result.path)" -ForegroundColor Cyan
            Write-Host "  Content: $($result.content.Substring(0, [Math]::Min(100, $result.content.Length)))..." -ForegroundColor White
        }
    } else {
        Write-Host "Search failed: $($r | ConvertTo-Json)" -ForegroundColor Red
    }
}

function Query-Agent {
    if (-not $Query) {
        Write-Host "Error: -Query required for query command" -ForegroundColor Red
        return
    }

    $payload = @{
        query = $Query
        limit = 5
        system3_mode = "fast"
    }

    $body = $payload | ConvertTo-Json -Compress
    $r = Invoke-RestMethod -Uri "$XAVIER/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body $body

    Write-Host "Response: $($r.response)" -ForegroundColor White
}

function Test-All {
    Write-Host "=== Xavier Test Suite ===" -ForegroundColor Cyan
    Write-Host ""

    Write-Host "1. Health Check..." -ForegroundColor Yellow
    try {
        $h = Invoke-RestMethod -Uri "$XAVIER/health" -Headers $headers
        Write-Host "   Status: $($h.status)" -ForegroundColor Green
    } catch {
        Write-Host "   FAILED: $_" -ForegroundColor Red
    }

    Write-Host ""
    Write-Host "2. Build Info..." -ForegroundColor Yellow
    try {
        $b = Invoke-RestMethod -Uri "$XAVIER/build" -Headers $headers
        Write-Host "   Version: $($b.version)" -ForegroundColor Green
        Write-Host "   Backend: $($b.memory_store.backend)" -ForegroundColor Green
    } catch {
        Write-Host "   FAILED: $_" -ForegroundColor Red
    }

    Write-Host ""
    Write-Host "3. Add Memory..." -ForegroundColor Yellow
    try {
        $testId = "test/ping-$(Get-Random)"
        $body = @{path=$testId; content="Xavier test $(Get-Date)"; metadata=@{}} | ConvertTo-Json -Compress
        $a = Invoke-RestMethod -Uri "$XAVIER/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $body
        if ($a.status -eq "ok") {
            Write-Host "   Added: $testId" -ForegroundColor Green
        } else {
            Write-Host "   FAILED: $($a | ConvertTo-Json)" -ForegroundColor Red
        }
    } catch {
        Write-Host "   FAILED: $_" -ForegroundColor Red
    }

    Write-Host ""
    Write-Host "4. Search Memory..." -ForegroundColor Yellow
    try {
        $body = @{query="test"; limit=5} | ConvertTo-Json -Compress
        $s = Invoke-RestMethod -Uri "$XAVIER/memory/search" -Method POST -Headers $headers -ContentType "application/json" -Body $body
        Write-Host "   Found: $($s.results.Count) results" -ForegroundColor Green
    } catch {
        Write-Host "   FAILED: $_" -ForegroundColor Red
    }

    Write-Host ""
    Write-Host "=== Test Complete ===" -ForegroundColor Cyan
}

# Execute command
switch ($Command) {
    "health" { Show-Health }
    "build"  { Show-Build }
    "add"    { Add-Memory }
    "search" { Search-Memory }
    "query"  { Query-Agent }
    "test"   { Test-All }
}
