# LoCoMo Real Benchmark - Using Actual OpenClaw Memories
# Tests recall of REAL data from Xavier2

$XAVIER2 = "http://localhost:8006"
$TOKEN = "dev-token"
$headers = @{"X-Xavier2-Token" = $TOKEN; "Content-Type" = "application/json"}

$results = @{
    total = 0
    passed = 0
    failed = 0
    details = @()
}

function Add-Memory {
    param([string]$Path, [string]$Content)
    try {
        $body = @{path=$Path; content=$Content; metadata=@{benchmark="locomo_real"; source="openclaw"}} | ConvertTo-Json -Compress
        $null = Invoke-RestMethod -Uri "$XAVIER2/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $body
        return $true
    } catch {
        Write-Host "  [ERROR adding $Path]: $_" -ForegroundColor Red
        return $false
    }
}

function Query-Agent {
    param([string]$Query)
    try {
        $resp = Invoke-RestMethod -Uri "$XAVIER2/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body (@{
            query = $Query
            limit = 3
            system3_mode = "disabled"
        } | ConvertTo-Json -Compress)
        return $resp.response
    } catch {
        return "ERROR: $_"
    }
}

function Test-Real {
    param([string]$Name, [string]$MemoryPath, [string]$MemoryContent, [string]$Question, [string]$ExpectedKeyword)

    $results.total++
    Write-Host ""
    Write-Host "[$($results.total)] $Name" -ForegroundColor Cyan
    Write-Host "  Memory: $MemoryPath" -ForegroundColor Gray

    # Add the real memory
    $added = Add-Memory -Path $MemoryPath -Content $MemoryContent
    if (-not $added) {
        $results.failed++
        $results.details += @{name=$Name; status="ERROR"; detail="Failed to add memory"}
        return
    }

    # Query
    Start-Sleep -Milliseconds 300
    $response = Query-Agent -Query $Question

    # Check if expected keyword is in response
    $pass = $response -and $response.ToString().ToLower().Contains($ExpectedKeyword.ToLower())

    if ($pass) {
        Write-Host "  Q: $Question" -ForegroundColor Gray
        Write-Host "  PASS ✅ - Found '$ExpectedKeyword'" -ForegroundColor Green
        $results.passed++
        $results.details += @{name=$Name; status="PASS"; response=$response}
    } else {
        Write-Host "  Q: $Question" -ForegroundColor Gray
        Write-Host "  FAIL ❌ - Expected '$ExpectedKeyword'" -ForegroundColor Red
        Write-Host "  Got: $($response.ToString().Substring(0, [Math]::Min(200, $response.ToString().Length)))" -ForegroundColor Yellow
        $results.failed++
        $results.details += @{name=$Name; status="FAIL"; response=$response; expected=$ExpectedKeyword}
    }
}

# === REAL OPENCLAW MEMORIES ===
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   LoCoMo Real Benchmark" -ForegroundColor Magenta
Write-Host "   Using Actual OpenClaw Memories" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

# Reset
Write-Host "`n[Setup] Resetting Xavier2..." -ForegroundColor Yellow
$null = Invoke-RestMethod -Uri "$XAVIER2/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}'
Start-Sleep 1

# === PEOPLE ===
Write-Host "`n=== PEOPLE RECALL ===" -ForegroundColor Yellow

Test-Real -Name "bel_profile" `
    -MemoryPath "people/bel/profile" `
    -MemoryContent "Bel (Belal) is the founder and lead developer at Southwest AI Labs. Telegram: 2076598024. Expertise: Rust, TypeScript, AI, OpenClaw." `
    -Question "Who is Bel and what is his expertise?" `
    -ExpectedKeyword "rust"

Test-Real -Name "sebas_profile" `
    -MemoryPath "people/sebas/profile" `
    -MemoryContent "Sebastian Belalcazar (Sebas) is the CEO of Southwest AI Labs. Telegram: 5885831693. Timezone: America/Bogota." `
    -Question "Who is the CEO of Southwest AI Labs?" `
    -ExpectedKeyword "sebastian"

Test-Real -Name "leonardo_profile" `
    -MemoryPath "people/leonardo/profile" `
    -MemoryContent "Leonardo Duque is a salesperson for ManteniApp. Market: industrial electrical and mining companies in Chile (Antofagasta). Product: SaaS maintenance tracking." `
    -Question "What does Leonardo sell and to whom?" `
    -ExpectedKeyword "manteniapp"

# === COMPANY ===
Write-Host "`n=== COMPANY RECALL ===" -ForegroundColor Yellow

Test-Real -Name "swal_overview" `
    -MemoryPath "company/swal/overview" `
    -MemoryContent "SouthWest AI Labs (SWAL) Website: github.com/southwest-ai-labs. Projects in E:\scripts-python. GitHub: iberi22/*. Business lines: Software Development, Content Creation, Crypto Trading, AI Agents, Finetuning." `
    -Question "What is SouthWest AI Labs and where are their projects?" `
    -ExpectedKeyword "southwest"

# === PROJECTS ===
Write-Host "`n=== PROJECT RECALL ===" -ForegroundColor Yellow

Test-Real -Name "xavier2_repo" `
    -MemoryPath "repo/xavier2" `
    -MemoryContent "Xavier2 repository keeps the typed memory schema in src/memory/schema.rs. Bridge import path in src/memory/bridge.rs. Version: 0.4.1." `
    -Question "Where is the Xavier2 memory schema stored?" `
    -ExpectedKeyword "schema"

Test-Real -Name "openclaw_status" `
    -MemoryPath "projects/openclaw/status" `
    -MemoryContent "OpenClaw status: Gateway healthy on port 9124. Xavier2 memory healthy. Telegram has delivery errors for heartbeat." `
    -Question "What is the status of OpenClaw?" `
    -ExpectedKeyword "healthy"

# === DECISIONS ===
Write-Host "`n=== DECISIONS RECALL ===" -ForegroundColor Yellow

Test-Real -Name "decision_memory" `
    -MemoryPath "decision/memory-core" `
    -MemoryContent "Decision: Keep System3 optional. Answer factual or temporal questions directly from typed evidence when possible." `
    -Question "What decision was made about System3?" `
    -ExpectedKeyword "optional"

# === TASKS ===
Write-Host "`n=== TASKS RECALL ===" -ForegroundColor Yellow

Test-Real -Name "task_multilingual" `
    -MemoryPath "task/multilingual-recall" `
    -MemoryContent "Task: Review the universal memory roadmap and connect OpenClaw with Engram without depending on fragile English heuristics." `
    -Question "What is the task for the memory roadmap?" `
    -ExpectedKeyword "engram"

# === SESSIONS ===
Write-Host "`n=== SESSION SUMMARIES ===" -ForegroundColor Yellow

Test-Real -Name "session_handoff" `
    -MemoryPath "session/openclaw-handoff" `
    -MemoryContent "Session handoff: Agent openclaw-content imported the YouTube publishing backlog and saved the result in Xavier2 for the ops team." `
    -Question "What did the openclaw-content agent do?" `
    -ExpectedKeyword "youtube"

# === MULTI-HOP ===
Write-Host "`n=== MULTI-HOP (Real Data) ===" -ForegroundColor Yellow

Test-Real -Name "multihop_bel_swal" `
    -MemoryPath "memory/bel-swal-relation" `
    -MemoryContent "Bel is the founder of Southwest AI Labs. Bel's Telegram is 2076598024. Bel works on the Xavier2 project." `
    -Question "What company did Bel found and what project does he work on?" `
    -ExpectedKeyword "southwest"

Test-Real -Name "multihop_leo_manteniapp" `
    -MemoryPath "memory/leonardo-manteniapp" `
    -MemoryContent "Leonardo sells ManteniApp to mining companies in Chile. ManteniApp is a maintenance tracking SaaS. Leonardo works with Rodacenter." `
    -Question "What does Leonardo sell and to which industry?" `
    -ExpectedKeyword "mining"

# === RESULTS ===
Write-Host ""
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   LoCoMo Real Benchmark Results" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta
Write-Host ""
Write-Host "Total:  $($results.total)" -ForegroundColor White
Write-Host "Passed: $($results.passed)  ($([Math]::Round($results.passed/$results.total*100))%)" -ForegroundColor $(if($results.passed -eq $results.total){'Green'}else{'Yellow'})
Write-Host "Failed: $($results.failed)  ($([Math]::Round($results.failed/$results.total*100))%)" -ForegroundColor $(if($results.failed -eq 0){'Green'}else{'Red'})
Write-Host ""

if ($results.passed -eq $results.total) {
    Write-Host "🏆 PERFECT SCORE! Xavier2 perfectly recalls all real OpenClaw memories." -ForegroundColor Green
} elseif ($results.passed -ge ($results.total * 0.8)) {
    Write-Host "👍 EXCELLENT! Xavier2 has strong memory recall." -ForegroundColor Cyan
} elseif ($results.passed -ge ($results.total * 0.6)) {
    Write-Host "👌 GOOD. Some recall gaps but mostly functional." -ForegroundColor Yellow
} else {
    Write-Host "⚠️  NEEDS WORK. Significant recall issues." -ForegroundColor Red
}

Write-Host ""
Write-Host "Failed tests:" -ForegroundColor Yellow
foreach ($d in $results.details) {
    if ($d.status -ne "PASS") {
        Write-Host "  - $($d.name): Expected '$($d.expected)'" -ForegroundColor Red
        Write-Host "    Got: $($d.response.ToString().Substring(0, [Math]::Min(100, $d.response.ToString().Length)))" -ForegroundColor Gray
    }
}
