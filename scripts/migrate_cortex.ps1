# Migrate memories from old Xavier (8003) to Xavier (8006)
# Run this script to transfer all memories

$ErrorActionPreference = "Stop"

$OLD_XAVIER = "http://localhost:8003"
$NEW_XAVIER = "http://localhost:8006"
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

Write-Host "=== Xavier Migration ===" -ForegroundColor Cyan
Write-Host "From: $OLD_XAVIER"
Write-Host "To:   $NEW_XAVIER"
Write-Host ""

# Get all memories from old Xavier
Write-Host "Fetching memories from old Xavier..." -ForegroundColor Yellow
$response = Invoke-RestMethod -Uri "$OLD_XAVIER/memory/search" -Method POST -Headers $headers -ContentType "application/json" -Body '{"query":"*","limit":100}'

if ($response.status -ne "ok") {
    Write-Host "ERROR: Failed to fetch memories" -ForegroundColor Red
    exit 1
}

$memories = $response.results
Write-Host "Found $($memories.Count) memories to migrate"
Write-Host ""

# Reset Xavier first (optional - comment out if you want to append)
Write-Host "Resetting Xavier..." -ForegroundColor Yellow
$null = Invoke-RestMethod -Uri "$NEW_XAVIER/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}'
Start-Sleep -Seconds 2

# Migrate each memory
$success = 0
$failed = 0

foreach ($m in $memories) {
    $payload = @{
        path = $m.path
        content = $m.content
        metadata = $m.metadata
        kind = $m.kind
    } | ConvertTo-Json -Compress

    try {
        $result = Invoke-RestMethod -Uri "$NEW_XAVIER/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $payload
        if ($result.status -eq "ok") {
            $success++
            Write-Host "[OK] $($m.path)" -ForegroundColor Green
        } else {
            $failed++
            Write-Host "[FAIL] $($m.path)" -ForegroundColor Red
        }
    } catch {
        $failed++
        Write-Host "[ERROR] $($m.path): $_" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== Migration Complete ===" -ForegroundColor Cyan
Write-Host "Success: $success" -ForegroundColor Green
Write-Host "Failed:  $failed" -ForegroundColor Red

# Verify
Write-Host ""
Write-Host "Verifying in Xavier..." -ForegroundColor Yellow
$verify = Invoke-RestMethod -Uri "$NEW_XAVIER/memory/search" -Method POST -Headers $headers -ContentType "application/json" -Body '{"query":"*","limit":100}'
Write-Host "Xavier now has $($verify.results.Count) memories"
