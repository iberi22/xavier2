# SEVIER Endpoint Validation Script for Xavier
# Tests all required endpoints with proper X-Xavier-Token header
# Usage: .\test-sevier-endpoints.ps1 [-BaseUrl http://localhost:8006] [-Token $env:XAVIER_TOKEN]

param(
    [string]$BaseUrl = "http://localhost:8006",
    [string]$Token = ""
)

if (-not $Token) {
    $Token = $env:XAVIER_TOKEN
    if (-not $Token) { $Token = $env:XAVIER_API_KEY }
    if (-not $Token) { $Token = $env:XAVIER_TOKEN }
    if (-not $Token) {
        throw "Missing Xavier token. Set XAVIER_TOKEN, XAVIER_API_KEY, or XAVIER_TOKEN."
    }
}

$ErrorActionPreference = "Continue"
$startTime = Get-Date

function Log-Test {
    param([string]$Name, [bool]$Pass, [int]$Status, [string]$Detail = "", [string]$Error = "")
    $icon = if ($Pass) { "[PASS]" } else { "[FAIL]" }
    $level = if ($Pass) { "PASS" } else { "FAIL" }
    $timestamp = Get-Date -Format "HH:mm:ss"
    Write-Host "[$timestamp] [$level] $icon $Name | status=$Status" -ForegroundColor $(if ($Pass) { "Green" } else { "Red" })
    if ($Detail) { Write-Host "         -> $Detail" }
    if ($Error)  { Write-Host "         -> ERROR: $Error" }
    return $Pass
}

# Headers
$headers = @{ "X-Xavier-Token" = $Token }
$jsonHeaders = @{
    "X-Xavier-Token" = $Token
    "Content-Type" = "application/json"
}

# Helper: GET
function Test-Get {
    param([string]$Endpoint, [string]$Description, [scriptblock]$Verify = $null)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $r = Invoke-WebRequest -Uri "$BaseUrl$Endpoint" -Method GET `
            -Headers $headers -TimeoutSec 15
        $sw.Stop()
        $content = $r.Content.Trim()
        $pass = $r.StatusCode -eq 200
        if ($Verify) { $pass = $Verify.Invoke($r) }
        $short = if ($content.Length -gt 120) { $content.Substring(0, 120) + "..." } else { $content }
        Log-Test -Name "$Description" -Pass $pass -Status $r.StatusCode -Detail "response: $short"
        return @{ Pass = $pass; Status = $r.StatusCode; Latency = $sw.ElapsedMilliseconds }
    } catch {
        $sw.Stop()
        Log-Test -Name "$Description" -Pass $false -Status 0 -Error $_.Exception.Message
        return @{ Pass = $false; Status = 0; Latency = $sw.ElapsedMilliseconds; Error = $_.Exception.Message }
    }
}

# Helper: POST
function Test-Post {
    param([string]$Endpoint, [object]$Body, [string]$Description, [scriptblock]$Verify = $null)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $json = $Body | ConvertTo-Json -Depth 5 -Compress
        $r = Invoke-WebRequest -Uri "$BaseUrl$Endpoint" -Method POST `
            -Body $json -Headers $jsonHeaders -TimeoutSec 15
        $sw.Stop()
        $content = $r.Content.Trim()
        $pass = $r.StatusCode -eq 200
        if ($Verify) { $pass = $Verify.Invoke($r) }
        $short = if ($content.Length -gt 120) { $content.Substring(0, 120) + "..." } else { $content }
        Log-Test -Name "$Description" -Pass $pass -Status $r.StatusCode -Detail "response: $short"
        return @{ Pass = $pass; Status = $r.StatusCode; Latency = $sw.ElapsedMilliseconds }
    } catch {
        $sw.Stop()
        Log-Test -Name "$Description" -Pass $false -Status 0 -Error $_.Exception.Message
        return @{ Pass = $false; Status = 0; Latency = $sw.ElapsedMilliseconds; Error = $_.Exception.Message }
    }
}

# ═══════════════════════════════════════════════════════════════════════════════
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  SEVIER Endpoint Validation" -ForegroundColor Cyan
Write-Host "  Base: $BaseUrl" -ForegroundColor Cyan
Write-Host "  Token: $Token" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$results = @()

# Test 1: GET /health -> expect {"status":"ok",...}
Write-Host "[TEST 1] GET /health" -ForegroundColor Yellow
$r = Test-Get -Endpoint "/health" -Description "GET /health" -Verify {
    param($resp)
    return $resp.Content -match '"status"'
}
$results += $r

# Test 2: POST /xavier/time/metric
Write-Host "[TEST 2] POST /xavier/time/metric" -ForegroundColor Yellow
$metricBody = @{
    metric_type = "powershell-validation"
    agent_id = "powershell-validator"
    task_id = "task-sevier-001"
    started_at = (Get-Date).ToUniversalTime().ToString("o")
    completed_at = (Get-Date).ToUniversalTime().ToString("o")
    duration_ms = 42
    status = "ok"
    provider = "powershell"
    model = "validator-v1"
    tokens_used = 50
    task_category = "validation"
    metadata = @{ source = "test-sevier-endpoints.ps1" }
}
$r = Test-Post -Endpoint "/xavier/time/metric" -Body $metricBody -Description "POST /xavier/time/metric"
$results += $r

# Test 3: POST /xavier/verify/save
Write-Host "[TEST 3] POST /xavier/verify/save" -ForegroundColor Yellow
$saveBody = @{
    path = "tests/sevier/endpoint-validation-$(Get-Date -Format 'yyyyMMdd-HHmmss')"
    content = "PowerShell endpoint validation test at $(Get-Date -Format 'o')"
}
$r = Test-Post -Endpoint "/xavier/verify/save" -Body $saveBody -Description "POST /xavier/verify/save" -Verify {
    param($resp)
    try {
        $j = $resp.Content | ConvertFrom-Json
        return $j.save_ok -eq $true
    } catch { return $false }
}
$results += $r

# Test 4: POST /xavier/events/session
Write-Host "[TEST 4] POST /xavier/events/session" -ForegroundColor Yellow
$eventBody = @{
    session_id = "sevier-ps-validation-$(Get-Random)"
    event_type = "validation_test"
    content = "PowerShell endpoint validation event"
    timestamp = (Get-Date).ToUniversalTime().ToString("o")
    metadata = @{ source = "test-sevier-endpoints.ps1" }
}
$r = Test-Post -Endpoint "/xavier/events/session" -Body $eventBody -Description "POST /xavier/events/session"
$results += $r

# Test 5: POST /xavier/agents/register
Write-Host "[TEST 5] POST /xavier/agents/register" -ForegroundColor Yellow
$agentBody = @{
    agent_id = "powershell-validator-$(Get-Random)"
    name = "PowerShell Validator"
    capabilities = @("validation", "testing")
    endpoint = "$BaseUrl/xavier/agents/powershell-validator"
}
$r = Test-Post -Endpoint "/xavier/agents/register" -Body $agentBody -Description "POST /xavier/agents/register"
$results += $r

# Test 6: GET /xavier/agents/active
Write-Host "[TEST 6] GET /xavier/agents/active" -ForegroundColor Yellow
$r = Test-Get -Endpoint "/xavier/agents/active" -Description "GET /xavier/agents/active"
$results += $r

# Test 7: POST /xavier/sync/check
Write-Host "[TEST 7] POST /xavier/sync/check" -ForegroundColor Yellow
$syncBody = @{
    source = "powershell-validator"
    timestamp = (Get-Date).ToUniversalTime().ToString("o")
    metadata = @{ source = "test-sevier-endpoints.ps1" }
}
$r = Test-Post -Endpoint "/xavier/sync/check" -Body $syncBody -Description "POST /xavier/sync/check"
$results += $r

# ═══════════════════════════════════════════════════════════════════════════════
# Summary
# ═══════════════════════════════════════════════════════════════════════════════
$totalMs = ((Get-Date) - $startTime).TotalMilliseconds
$passCount = ($results | Where-Object { $_.Pass }).Count
$failCount = $results.Count - $passCount

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Summary" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Passed:  $passCount / $($results.Count)" -ForegroundColor $(if ($passCount -eq $results.Count) { "Green" } else { "Yellow" })
Write-Host "  Failed:  $failCount" -ForegroundColor $(if ($failCount -gt 0) { "Red" } else { "Green" })
Write-Host "  Total:   $([Math]::Round($totalMs))ms" -ForegroundColor Cyan
Write-Host ""

if ($passCount -eq $results.Count) {
    Write-Host "ALL ENDPOINT TESTS PASSED" -ForegroundColor Green
} else {
    Write-Host "SOME ENDPOINT TESTS FAILED" -ForegroundColor Red
}

Write-Host ""
Write-Host "PS SCRIPT COMPLETE" -ForegroundColor Magenta
exit $(if ($passCount -eq $results.Count) { 0 } else { 1 })
