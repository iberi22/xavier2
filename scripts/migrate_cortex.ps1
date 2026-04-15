# Migrate memories from old Xavier2 (8003) to Xavier2 (8006)
# Run this script to transfer all memories

$ErrorActionPreference = "Stop"

$OLD_XAVIER2 = "http://localhost:8003"
$NEW_XAVIER2 = "http://localhost:8006"
$TOKEN = "dev-token"

$headers = @{"X-Xavier2-Token" = $TOKEN}

Write-Host "=== Xavier2 Migration ===" -ForegroundColor Cyan
Write-Host "From: $OLD_XAVIER2"
Write-Host "To:   $NEW_XAVIER2"
Write-Host ""

# Get all memories from old Xavier2
Write-Host "Fetching memories from old Xavier2..." -ForegroundColor Yellow
$response = Invoke-RestMethod -Uri "$OLD_XAVIER2/memory/search" -Method POST -Headers $headers -ContentType "application/json" -Body '{"query":"*","limit":100}'

if ($response.status -ne "ok") {
    Write-Host "ERROR: Failed to fetch memories" -ForegroundColor Red
    exit 1
}

$memories = $response.results
Write-Host "Found $($memories.Count) memories to migrate"
Write-Host ""

# Reset Xavier2 first (optional - comment out if you want to append)
Write-Host "Resetting Xavier2..." -ForegroundColor Yellow
$null = Invoke-RestMethod -Uri "$NEW_XAVIER2/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}'
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
        $result = Invoke-RestMethod -Uri "$NEW_XAVIER2/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $payload
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
Write-Host "Verifying in Xavier2..." -ForegroundColor Yellow
$verify = Invoke-RestMethod -Uri "$NEW_XAVIER2/memory/search" -Method POST -Headers $headers -ContentType "application/json" -Body '{"query":"*","limit":100}'
Write-Host "Xavier2 now has $($verify.results.Count) memories"
