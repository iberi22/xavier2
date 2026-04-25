# SEVIER2 Stress Test Runner for Xavier2
# Validates endpoints: /health, /xavier2/events/session, /xavier2/verify/save,
# /xavier2/time/metric, /ready

param(
    [string]$BaseUrl = "http://localhost:8006",
    [string]$Token = "dev-token",
    [switch]$StartDocker   # Start Xavier2 via docker compose if not running
)

$ErrorActionPreference = "Continue"
$startTime = Get-Date

function Log-Message {
    param([string]$msg, [string]$level = "INFO")
    $timestamp = Get-Date -Format "HH:mm:ss"
    $color = switch ($level) {
        "PASS" { "Green" }
        "FAIL" { "Red" }
        "WARN" { "Yellow" }
        default { "White" }
    }
    Write-Host "[$timestamp] [$level] $msg" -ForegroundColor $color
}

# ─── 1. Check/Docker Start ─────────────────────────────────────────────────────
$running = $false
try {
    $healthResp = Invoke-WebRequest -Uri "$BaseUrl/health" -Method GET `
        -Headers @{ "Authorization" = "Bearer $Token" } `
        -TimeoutSec 5 -ErrorAction SilentlyContinue
    if ($healthResp.StatusCode -eq 200) {
        $running = $true
        Log-Message "Xavier2 is already running at $BaseUrl" "INFO"
    }
} catch {
    Log-Message "Xavier2 not responding at $BaseUrl" "WARN"
}

if (-not $running) {
    if ($StartDocker) {
        Log-Message "Starting Xavier2 via docker compose..." "INFO"
        Set-Location "E:\scripts-python\xavier2"
        docker compose up -d xavier2
        Start-Sleep -Seconds 5

        # Poll /health until ready (max 60s)
        $attempts = 0
        while ($attempts -lt 12) {
            try {
                $h = Invoke-WebRequest -Uri "$BaseUrl/health" -Method GET `
                    -TimeoutSec 5 -ErrorAction SilentlyContinue
                if ($h.StatusCode -eq 200) {
                    $running = $true
                    Log-Message "Xavier2 started and healthy" "PASS"
                    break
                }
            } catch { }
            $attempts++
            Log-Message "Waiting for Xavier2... ($attempts/12)" "WARN"
            Start-Sleep -Seconds 5
        }
        if (-not $running) {
            Log-Message "Xavier2 failed to start in 60s" "FAIL"
            exit 1
        }
    } else {
        Log-Message "Xavier2 not running. Use -StartDocker to auto-start." "FAIL"
        exit 1
    }
}

# ─── 2. Helper function for POST with JSON ────────────────────────────────────
function Invoke-Sevier2Post {
    param([string]$Endpoint, [object]$Body, [string]$Description)
    try {
        $json = $Body | ConvertTo-Json -Depth 5 -Compress
        $resp = Invoke-WebRequest -Uri "$BaseUrl$Endpoint" `
            -Method POST `
            -Body $json `
            -ContentType "application/json" `
            -Headers @{ "Authorization" = "Bearer $Token" } `
            -TimeoutSec 30
        return @{
            Success = ($resp.StatusCode -eq 200)
            Status = $resp.StatusCode
            Body = $resp.Content
        }
    } catch {
        return @{
            Success = $false
            Status = 0
            Error = $_.Exception.Message
        }
    }
}

# ─── 3. Test Results ───────────────────────────────────────────────────────────
$results = @()

# ─── Test A: GET /health ───────────────────────────────────────────────────────
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $r = Invoke-WebRequest -Uri "$BaseUrl/health" -Method GET `
        -Headers @{ "Authorization" = "Bearer $Token" } `
        -TimeoutSec 10
    $sw.Stop()
    $bodyOk = $r.Content -eq '"ok"' -or $r.Content -eq 'ok'
    $results += @{
        Name = "GET /health"
        Pass = ($r.StatusCode -eq 200 -and $bodyOk)
        Status = $r.StatusCode
        LatencyMs = $sw.ElapsedMilliseconds
        Detail = if ($bodyOk) { "body = ok" } else { "body = $($r.Content)" }
    }
} catch {
    $sw.Stop()
    $results += @{
        Name = "GET /health"
        Pass = $false
        Status = 0
        LatencyMs = $sw.ElapsedMilliseconds
        Error = $_.Exception.Message
    }
}

# ─── Test B: GET /ready ────────────────────────────────────────────────────────
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $r = Invoke-WebRequest -Uri "$BaseUrl/ready" -Method GET `
        -Headers @{ "Authorization" = "Bearer $Token" } `
        -TimeoutSec 10
    $sw.Stop()
    $json = $r.Content | ConvertFrom-Json
    $hasWorkspace = $null -ne $json.workspace_id
    $results += @{
        Name = "GET /ready"
        Pass = ($r.StatusCode -eq 200 -and $hasWorkspace)
        Status = $r.StatusCode
        LatencyMs = $sw.ElapsedMilliseconds
        Detail = "workspace_id = $($json.workspace_id)"
    }
} catch {
    $sw.Stop()
    $results += @{
        Name = "GET /ready"
        Pass = $false
        Status = 0
        LatencyMs = $sw.ElapsedMilliseconds
        Error = $_.Exception.Message
    }
}

# ─── Test C: POST /xavier2/events/session ─────────────────────────────────────
$body = @{
    session_id = "ps-test-session"
    event_type = "message"
    content = "PowerShell stress test event"
    timestamp = (Get-Date).ToUniversalTime().ToString("o")
}
$result = Invoke-Sevier2Post -Endpoint "/xavier2/events/session" `
    -Body $body -Description "Session Event"
$result.LatencyMs = $sw.ElapsedMilliseconds
$results += @{
    Name = "POST /xavier2/events/session"
    Pass = $result.Success
    Status = $result.Status
    Detail = $result.Body
}

# ─── Test D: POST /xavier2/verify/save ────────────────────────────────────────
$body = @{
    path = "tests/sevier2/powershell-runner"
    content = "PowerShell test content $(Get-Date -Format 'o')"
}
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$result = Invoke-Sevier2Post -Endpoint "/xavier2/verify/save" `
    -Body $body -Description "Verify Save"
$sw.Stop()
if ($result.Success) {
    $json = $result.Body | ConvertFrom-Json
    $results += @{
        Name = "POST /xavier2/verify/save"
        Pass = $json.save_ok -eq $true
        Status = $result.Status
        LatencyMs = $sw.ElapsedMilliseconds
        Detail = "save_ok=$($json.save_ok), match_score=$($json.match_score)"
    }
} else {
    $results += @{
        Name = "POST /xavier2/verify/save"
        Pass = $false
        Status = $result.Status
        LatencyMs = $sw.ElapsedMilliseconds
        Error = $result.Error
    }
}

# ─── Test E: POST /xavier2/time/metric ─────────────────────────────────────────
$body = @{
    metric_type = "powershell-stress-test"
    agent_id = "powershell-runner"
    task_id = "task-1"
    started_at = (Get-Date).ToUniversalTime().ToString("o")
    completed_at = (Get-Date).ToUniversalTime().ToString("o")
    duration_ms = 42
    status = "ok"
    provider = "powershell"
    model = "runner-v1"
    tokens_used = 100
    task_category = "test"
    metadata = @{}
}
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$result = Invoke-Sevier2Post -Endpoint "/xavier2/time/metric" `
    -Body $body -Description "Time Metric"
$sw.Stop()
$results += @{
    Name = "POST /xavier2/time/metric"
    Pass = $result.Success
    Status = $result.Status
    LatencyMs = $sw.ElapsedMilliseconds
    Detail = $result.Body
}

# ─── 4. Concurrent Tests (bonus) ───────────────────────────────────────────────
Log-Message "Running concurrent stress batch (10 parallel events)..." "INFO"
$concurrentStart = Get-Date
$jobs = @()
for ($i = 0; $i -lt 10; $i++) {
    $body = @{
        session_id = "ps-concurrent-$i"
        event_type = "message"
        content = "Concurrent event #$i"
        timestamp = (Get-Date).ToUniversalTime().ToString("o")
    }
    $jobs += Start-Job -ScriptBlock {
        param($url, $token, $endpoint, $b)
        try {
            $r = Invoke-WebRequest -Uri "$url$endpoint" `
                -Method POST `
                -Body ($b | ConvertTo-Json -Compress) `
                -ContentType "application/json" `
                -Headers @{ "Authorization" = "Bearer $token" } `
                -TimeoutSec 15
            return @{ Status = $r.StatusCode; Success = $true }
        } catch {
            return @{ Status = 0; Success = $false; Error = $_.Exception.Message }
        }
    } -ArgumentList $BaseUrl, $Token, "/xavier2/events/session", $body
}
$completed = $jobs | Wait-Job -Timeout 30
$concurrentMs = ((Get-Date) - $concurrentStart).TotalMilliseconds
$okCount = ($completed | Receive-Job | Where-Object { $_.Status -eq 200 }).Count
$jobs | Remove-Job -Force
Log-Message "Concurrent events: $okCount/10 succeeded in $([math]::Round($concurrentMs))ms" `
    $(if ($okCount -eq 10) { "PASS" } else { "WARN" })

# ─── 5. Summary ────────────────────────────────────────────────────────────────
$totalMs = ((Get-Date) - $startTime).TotalMilliseconds
Write-Host ""
Write-Host "══════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  SEVIER2 Stress Test Results" -ForegroundColor Cyan
Write-Host "  Base URL: $BaseUrl" -ForegroundColor Cyan
Write-Host "  Total Time: $([math]::Round($totalMs))ms" -ForegroundColor Cyan
Write-Host "══════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

$passCount = 0
foreach ($r in $results) {
    $icon = if ($r.Pass) { "✅" } else { "❌" }
    $level = if ($r.Pass) { "PASS" } else { "FAIL" }
    Log-Message "$icon $($r.Name) | status=$($r.Status) | latency=$($r.LatencyMs)ms | $($r.Detail ?? $r.Error)" $level
    if ($r.Pass) { $passCount++ }
}

Write-Host ""
$passRate = "{0:P0}" -f ($passCount / $results.Count)
if ($passCount -eq $results.Count) {
    Write-Host "ALL TESTS PASSED ($passRate) 🎉" -ForegroundColor Green
    exit 0
} else {
    Write-Host "SOME TESTS FAILED ($passCount/$($results.Count) = $passRate)" -ForegroundColor Red
    exit 1
}
