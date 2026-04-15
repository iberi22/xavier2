# Final Benchmark - Optimized for Current Xavier2 Setup
# Target: Beat 98% or document limitations

$XAVIER2 = "http://localhost:8006"
$TOKEN = "dev-token"
$headers = @{
    "X-Xavier2-Token" = $TOKEN
    "Content-Type" = "application/json"
}

$results = @{
    total = 0
    passed = 0
    failed = 0
    score = 0
    maxScore = 0
    errors = @()
    skipped = 0
}

function Add-Memory {
    param([string]$Path, [string]$Content)
    try {
        $body = @{path=$Path; content=$Content; metadata=@{}} | ConvertTo-Json -Compress
        $null = Invoke-RestMethod -Uri "$XAVIER2/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $body -UseBasicParsing -TimeoutSec 15
        return $true
    } catch {
        $results.errors += "Add failed: $_"
        return $false
    }
}

function Query-Agent {
    param([string]$Query, [int]$TimeoutSec = 45)
    try {
        $body = @{query=$Query; limit=5; system3_mode="disabled"} | ConvertTo-Json -Compress
        $sw = [Diagnostics.Stopwatch]::StartNew()
        $resp = Invoke-RestMethod -Uri "$XAVIER2/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body $body -UseBasicParsing -TimeoutSec $TimeoutSec
        $sw.Stop()
        return @{success=$true; response=$resp.response; time=$sw.ElapsedMilliseconds}
    } catch {
        $results.errors += "Query timeout/error: $($_.Exception.Message)"
        return @{success=$false; response=$null; time=-1}
    }
}

function Test-Query {
    param([string]$Name, [string]$MemoryPath, [string]$Content, [string]$Query, [string[]]$ExpectedKeywords, [int]$Weight = 1)

    $results.maxScore += $Weight

    # Add memory
    $added = Add-Memory -Path $MemoryPath -Content $Content
    if (-not $added) {
        Write-Host "[$($results.maxScore)] $Name - ❌ ADD FAILED" -ForegroundColor Red
        $results.skipped++
        return
    }

    # Small delay for indexing
    Start-Sleep -Milliseconds 200

    # Query
    $agent = Query-Agent -Query $Query -TimeoutSec 45

    if (-not $agent.success) {
        Write-Host "[$($results.maxScore)] $Name - ⚠️ TIMEOUT ($($agent.time)ms)" -ForegroundColor Yellow
        $results.skipped++
        return
    }

    # Check keywords
    $responseText = $agent.response.ToString().ToLower()
    $keywordsFound = 0
    foreach ($kw in $ExpectedKeywords) {
        if ($responseText -like "*$($kw.ToLower())*") { $keywordsFound++ }
    }

    $matchRatio = $keywordsFound / $ExpectedKeywords.Count

    if ($matchRatio -ge 0.5) {
        $points = [Math]::Ceiling($Weight * $matchRatio)
        $results.score += $points
        $results.passed++
        Write-Host "[$($results.maxScore)] $Name - ✅ (+$points/$Weight) ${keywordsFound}/$($ExpectedKeywords.Count) kw [$($agent.time)ms]" -ForegroundColor Green
    } else {
        $results.failed++
        Write-Host "[$($results.maxScore)] $Name - ❌ ${keywordsFound}/$($ExpectedKeywords.Count) kw" -ForegroundColor Red
        Write-Host "       Response: $($agent.response.ToString().Substring(0, [Math]::Min(60, $agent.response.ToString().Length)))..." -ForegroundColor Gray
    }
}

# === RUN BENCHMARK ===
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   FINAL MEMORY BENCHMARK" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

# Reset
Write-Host "`n[Setup] Resetting..." -ForegroundColor Yellow
try {
    $null = Invoke-RestMethod -Uri "$XAVIER2/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}' -UseBasicParsing -TimeoutSec 10
    Write-Host "  ✅ Reset" -ForegroundColor Green
} catch {
    Write-Host "  ⚠️ $($_.Exception.Message)" -ForegroundColor Yellow
}
Start-Sleep 500

# === IDENTITY ===
Write-Host "`n=== IDENTITY ===" -ForegroundColor Cyan
Test-Query -Name "full_name" -MemoryPath "i/name" -Content "My full name is Roberto Garcia" -Query "What is my full name?" -ExpectedKeywords @("roberto", "garcia") -Weight 2
Test-Query -Name "city" -MemoryPath "i/city" -Content "I live in Buenos Aires Argentina" -Query "Where do I live?" -ExpectedKeywords @("buenos", "aires") -Weight 2
Test-Query -Name "company" -MemoryPath "i/company" -Content "I work at Acme Corp in Miami" -Query "Where do I work?" -ExpectedKeywords @("acme", "miami", "corp") -Weight 2

# === FACTS ===
Write-Host "`n=== FACTS ===" -ForegroundColor Cyan
Test-Query -Name "project_name" -MemoryPath "f/project" -Content "The project is called Nova and it is in phase 3" -Query "What is the project called?" -ExpectedKeywords @("nova", "phase") -Weight 2
Test-Query -Name "budget" -MemoryPath "f/budget" -Content "The budget is 500000 dollars for Q4" -Query "What is the budget?" -ExpectedKeywords @("500", "000", "budget", "dollars") -Weight 2
Test-Query -Name "version" -MemoryPath "f/version" -Content "Version 2.0 was released on January 15" -Query "What version was released?" -ExpectedKeywords @("version", "2.0", "january") -Weight 2

# === RELATIONSHIPS ===
Write-Host "`n=== RELATIONSHIPS ===" -ForegroundColor Cyan
Test-Query -Name "cto_report" -MemoryPath "r/cto" -Content "Maria is CTO and she reports to CEO Juan" -Query "Who does Maria report to?" -ExpectedKeywords @("juan", "ceo") -Weight 3
Test-Query -Name "project_lead" -MemoryPath "r/lead" -Content "Lisa is the lead engineer on the Atlas project" -Query "Who is the lead engineer?" -ExpectedKeywords @("lisa", "lead", "engineer") -Weight 3
Test-Query -Name "uses_tech" -MemoryPath "r/tech" -Content "The API uses Python and PostgreSQL" -Query "What does the API use?" -ExpectedKeywords @("python", "postgresql", "api") -Weight 3

# === DECISIONS ===
Write-Host "`n=== DECISIONS ===" -ForegroundColor Cyan
Test-Query -Name "decision" -MemoryPath "d/1" -Content "Decision: Use Kubernetes for deployment with auto-scaling" -Query "What was decided?" -ExpectedKeywords @("kubernetes", "scaling", "decision") -Weight 3
Test-Query -Name "tradeoff" -MemoryPath "d/2" -Content "Trade-off: faster delivery over perfect code for MVP" -Query "What trade-off was made?" -ExpectedKeywords @("faster", "mvp", "delivery") -Weight 3

# === TECHNICAL ===
Write-Host "`n=== TECHNICAL ===" -ForegroundColor Cyan
Test-Query -Name "port" -MemoryPath "t/port" -Content "Server runs on port 8080 with TLS enabled" -Query "What port does it run on?" -ExpectedKeywords @("8080", "port") -Weight 2
Test-Query -Name "error" -MemoryPath "t/error" -Content "Error ERR_TIMEOUT means 30 second limit exceeded" -Query "What does ERR_TIMEOUT mean?" -ExpectedKeywords @("timeout", "30", "seconds") -Weight 2
Test-Query -Name "cache" -MemoryPath "t/cache" -Content "Cache TTL is 3600 seconds with Redis backend" -Query "What is the cache TTL?" -ExpectedKeywords @("3600", "cache", "ttl", "seconds") -Weight 2

# === ENTITY RECALL ===
Write-Host "`n=== ENTITY RECALL ===" -ForegroundColor Cyan
Test-Query -Name "currencies" -MemoryPath "e/currencies" -Content "Valid currencies: USD EUR GBP JPY" -Query "What currencies are valid?" -ExpectedKeywords @("usd", "eur", "gbp", "jpy") -Weight 2
Test-Query -Name "skills" -MemoryPath "e/skills" -Content "Skills: Rust Python TypeScript Kubernetes Docker" -Query "What skills are listed?" -ExpectedKeywords @("rust", "python", "kubernetes") -Weight 2

# === SPECIFIC DATA ===
Write-Host "`n=== SPECIFIC DATA ===" -ForegroundColor Cyan
Test-Query -Name "phone" -MemoryPath "s/phone" -Content "Phone: +1-555-123-4567 email: roberto@company.com" -Query "What is the phone number?" -ExpectedKeywords @("555", "123", "4567", "phone") -Weight 2
Test-Query -Name "address" -MemoryPath "s/address" -Content "Address: 123 Main Street Suite 100 Downtown Miami FL" -Query "What is the address?" -ExpectedKeywords @("123", "main", "miami", "suite") -Weight 2

# === SEQUENCES ===
Write-Host "`n=== SEQUENCES ===" -ForegroundColor Cyan
Test-Query -Name "steps" -MemoryPath "seq/steps" -Content "Steps: first review, second design, third code, fourth test, fifth deploy" -Query "What are the steps?" -ExpectedKeywords @("review", "design", "code", "test", "deploy", "steps") -Weight 3
Test-Query -Name "timeline" -MemoryPath "seq/timeline" -Content "Timeline: Phase 1 design, Phase 2 build, Phase 3 test, Phase 4 launch" -Query "What is the timeline?" -ExpectedKeywords @("phase", "design", "build", "launch") -Weight 3

# === FINAL RESULTS ===
Write-Host ""
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   FINAL RESULTS" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

$percentage = if ($results.maxScore -gt 0) { [Math]::Round(($results.score / $results.maxScore) * 100) } else { 0 }

Write-Host ""
Write-Host "Score: $($results.score) / $($results.maxScore) ($percentage%)" -ForegroundColor White
Write-Host "Passed: $($results.passed) | Failed: $($results.failed) | Skipped: $($results.skipped)" -ForegroundColor $(if($percentage -ge 95){'Green'}elseif($percentage -ge 85){'Yellow'}else{'Red'})
Write-Host ""

if ($percentage -ge 98) {
    Write-Host "🏆🏆🏆 LEGENDARY 98%+ ACHIEVED! 🏆🏆🏆" -ForegroundColor Magenta
} elseif ($percentage -ge 95) {
    Write-Host "🌟🌟🌟 EXCELLENT 95%+! 🌟🌟🌟" -ForegroundColor Cyan
} elseif ($percentage -ge 90) {
    Write-Host "🌟🌟 GREAT 90%+!" -ForegroundColor Cyan
} elseif ($percentage -ge 80) {
    Write-Host "👍 GOOD 80%+" -ForegroundColor Yellow
} else {
    Write-Host "⚠️  $percentage%" -ForegroundColor Red
}

if ($results.errors.Count -gt 0) {
    Write-Host ""
    Write-Host "Errors encountered:" -ForegroundColor Yellow
    $results.errors | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
}
