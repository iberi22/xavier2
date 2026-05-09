# Super Memory Benchmark - Target: Beat 98%
# Comprehensive test of Xavier memory capabilities

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
$headers = @{
    "X-Xavier-Token" = $TOKEN
    "Content-Type" = "application/json"
}

$results = @{
    total = 0
    passed = 0
    failed = 0
    score = 0
    details = @()
}

function Add-Memory {
    param([string]$Path, [string]$Content)
    try {
        $body = @{path=$Path; content=$Content; metadata=@{}} | ConvertTo-Json -Compress
        $null = Invoke-RestMethod -Uri "$XAVIER/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $body -UseBasicParsing
        return $true
    } catch { return $false }
}

function Query-Memory {
    param([string]$Query)
    try {
        $body = @{query=$Query; limit=5} | ConvertTo-Json -Compress
        $resp = Invoke-RestMethod -Uri "$XAVIER/memory/search" -Method POST -Headers $headers -ContentType "application/json" -Body $body -UseBasicParsing
        return $resp
    } catch { return $null }
}

function Test-Query {
    param([string]$Name, [string]$MemoryPath, [string]$Content, [string]$Query, [string[]]$ExpectedKeywords, [int]$Weight = 1)

    $results.total += $Weight
    Write-Host ""
    Write-Host "[$($results.total)] $Name" -ForegroundColor Cyan

    # Add memory
    $null = Add-Memory -Path $MemoryPath -Content $Content

    # Query
    Start-Sleep -Milliseconds 100
    $search = Query-Memory -Query $Query

    if (-not $search) {
        Write-Host "  ❌ Query failed" -ForegroundColor Red
        $results.details += @{name=$Name; status="ERROR"; query=$Query}
        return
    }

    # Check if memory is in results
    $found = $false
    $maxScore = 0
    $responseText = ""

    if ($search.results -and $search.results.Count -gt 0) {
        foreach ($r in $search.results) {
            if ($r.path -eq $MemoryPath -or $r.content -like "*$MemoryPath*") {
                $found = $true
                $maxScore = [Math]::Max($maxScore, $r.score)
            }
            $responseText += " " + $r.content
        }
    }

    # Also check with agent
    $agentBody = @{query=$Query; limit=3; system3_mode="disabled"} | ConvertTo-Json -Compress
    $agent = Invoke-RestMethod -Uri "$XAVIER/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body $agentBody -UseBasicParsing
    $agentText = $agent.response

    # Check expected keywords in either search results or agent response
    $keywordsFound = 0
    $combinedText = ($responseText + " " + $agentText).ToLower()

    foreach ($kw in $ExpectedKeywords) {
        if ($combinedText -like "*$($kw.ToLower())*") {
            $keywordsFound++
        }
    }

    $keywordRatio = $keywordsFound / $ExpectedKeywords.Count

    # Pass if found in search OR keywords match
    if ($found -or $keywordRatio -ge 0.5) {
        $passed = $true
        if ($found -and $maxScore -gt 0.8) {
            $points = $Weight  # Full points for high score match
        } else {
            $points = [Math]::Ceiling($Weight * $keywordRatio)  # Partial points for keyword match
        }
        $results.passed += $points
        $results.score += $points
        Write-Host "  ✅ PASS (+$points pts) - Score: $([Math]::Round($maxScore*100))% Keywords: $keywordsFound/$($ExpectedKeywords.Count)" -ForegroundColor Green
        $results.details += @{name=$Name; status="PASS"; score=$maxScore; keywords=$keywordsFound}
    } else {
        $results.failed += $Weight
        Write-Host "  ❌ FAIL - Not found in top results" -ForegroundColor Red
        Write-Host "  Response preview: $($agentText.ToString().Substring(0, [Math]::Min(100, $agentText.ToString().Length)))" -ForegroundColor Yellow
        $results.details += @{name=$Name; status="FAIL"; query=$Query; expected=$ExpectedKeywords}
    }
}

# === RESET AND PREPARE ===
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   SUPER MEMORY BENCHMARK" -ForegroundColor Magenta
Write-Host "   Target: Beat 98% Score" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

Write-Host "`n[Setup] Resetting memories..." -ForegroundColor Yellow
try {
    $null = Invoke-RestMethod -Uri "$XAVIER/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}' -UseBasicParsing
    Write-Host "  ✅ Reset complete" -ForegroundColor Green
} catch {
    Write-Host "  ⚠️  Reset skipped (may already be empty)" -ForegroundColor Yellow
}
Start-Sleep 1

# === IDENTITY TESTS ===
Write-Host "`n=== IDENTITY & FACTS ===" -ForegroundColor Yellow

Test-Query -Name "person_name" -MemoryPath "test/person" -Content "My name is Roberto Garcia and I work at TechCorp" -Query "What is my name?" -ExpectedKeywords @("roberto", "garcia") -Weight 2

Test-Query -Name "company_name" -MemoryPath "test/company" -Content "The company Acme Industries is based in Miami Florida" -Query "What company am I at?" -ExpectedKeywords @("acme", "miami") -Weight 2

Test-Query -Name "city_name" -MemoryPath "test/city" -Content "I live in Buenos Aires Argentina" -Query "Where do I live?" -ExpectedKeywords @("buenos", "aires", "argentina") -Weight 2

# === PROJECT FACTS ===
Write-Host "`n=== PROJECT FACTS ===" -ForegroundColor Yellow

Test-Query -Name "project_status" -MemoryPath "test/project" -Content "The project called Nova is in active development phase 3 with $500K budget" -Query "What is the Nova project status?" -ExpectedKeywords @("nova", "phase", "500", "development") -Weight 3

Test-Query -Name "repo_info" -MemoryPath "test/repo" -Content "Repository gestalt-rust uses Rust with 95% test coverage and runs on port 8080" -Query "What language is gestalt-rust written in?" -ExpectedKeywords @("rust", "95") -Weight 2

Test-Query -Name "docker_config" -MemoryPath "test/docker" -Content "Docker container xavier exposes port 8006 and uses 2GB RAM" -Query "What port does xavier expose?" -ExpectedKeywords @("8006", "xavier") -Weight 2

# === RELATIONSHIPS (Multi-hop) ===
Write-Host "`n=== RELATIONSHIPS ===" -ForegroundColor Yellow

Test-Query -Name "person_company_rel" -MemoryPath "test/rel1" -Content "Maria is the CTO of DataFlow Inc. She reports to the CEO Juan." -Query "Who does Maria report to?" -ExpectedKeywords @("juan", "ceo") -Weight 3

Test-Query -Name "project_lead_rel" -MemoryPath "test/rel2" -Content "The Atlas project is led by Senior Engineer Lisa. Lisa works with the DevOps team." -Query "Who leads the Atlas project?" -ExpectedKeywords @("lisa", "atlas", "senior") -Weight 3

Test-Query -Name "tech_stack_rel" -MemoryPath "test/rel3" -Content "The API service uses Python FastAPI connected to PostgreSQL database" -Query "What does the API service use?" -ExpectedKeywords @("python", "fastapi", "postgresql") -Weight 3

# === TEMPORAL ===
Write-Host "`n=== TEMPORAL ===" -ForegroundColor Yellow

Test-Query -Name "date_event" -MemoryPath "test/date1" -Content "Meeting scheduled for March 15 2026 at 3pm in conference room B" -Query "When is the meeting?" -ExpectedKeywords @("march", "15", "2026", "3pm") -Weight 2

Test-Query -Name "sequence" -MemoryPath "test/seq1" -Content "Step 1 reviewed requirements, Step 2 designed architecture, Step 3 wrote code, Step 4 deployed to production" -Query "What are the steps in the workflow?" -ExpectedKeywords @("reviewed", "designed", "wrote", "deployed", "step") -Weight 3

# === TECHNICAL ===
Write-Host "`n=== TECHNICAL ===" -ForegroundColor Yellow

Test-Query -Name "api_endpoint" -MemoryPath "test/api" -Content "REST API endpoint /api/v2/users returns JSON with fields id name email created_at" -Query "What endpoint returns users?" -ExpectedKeywords @("api", "users", "json") -Weight 2

Test-Query -Name "error_code" -MemoryPath "test/error" -Content "Error code ERR_TIMEOUT indicates the operation exceeded 30 second limit" -Query "What does ERR_TIMEOUT mean?" -ExpectedKeywords @("timeout", "30", "exceeded") -Weight 2

Test-Query -Name "config_setting" -MemoryPath "test/config" -Content "Feature flag ENABLE_CACHE is set to true with TTL of 3600 seconds" -Query "What is the cache TTL?" -ExpectedKeywords @("3600", "cache", "true") -Weight 2

# === DECISIONS ===
Write-Host "`n=== DECISIONS ===" -ForegroundColor Yellow

Test-Query -Name "decision_made" -MemoryPath "test/decision1" -Content "Decision: Use Kubernetes for orchestration with auto-scaling enabled between 2 and 10 replicas" -Query "What was decided about orchestration?" -ExpectedKeywords @("kubernetes", "scaling", "replicas") -Weight 3

Test-Query -Name "tradeoff_decision" -MemoryPath "test/decision2" -Content "Trade-off accepted: faster delivery over perfect code quality for MVP phase" -Query "What trade-off was accepted?" -ExpectedKeywords @("faster", "delivery", "mvp", "quality") -Weight 3

# === CONTEXT RULES ===
Write-Host "`n=== CONTEXT RULES ===" -ForegroundColor Yellow

Test-Query -Name "context_rule" -MemoryPath "test/context1" -Content "Always use British English spelling for documents going to London office" -Query "What spelling should be used for London?" -ExpectedKeywords @("british", "english", "london") -Weight 2

Test-Query -Name "priority_rule" -MemoryPath "test/context2" -Content "Security fixes must be deployed within 24 hours of being reported" -Query "How quickly must security fixes be deployed?" -ExpectedKeywords @("24", "hours", "security") -Weight 3

# === ENTITY RECALL ===
Write-Host "`n=== ENTITY RECALL ===" -ForegroundColor Yellow

Test-Query -Name "entity_list" -MemoryPath "test/entities" -Content "Valid currencies: USD EUR GBP JPY CHF AUD CAD" -Query "What currencies are valid?" -ExpectedKeywords @("usd", "eur", "gbp") -Weight 2

Test-Query -Name "version_list" -MemoryPath "test/versions" -Content "Supported versions: v1.0 v1.5 v2.0 v2.1 v3.0" -Query "What versions are supported?" -ExpectedKeywords @("v1", "v2", "v3") -Weight 2

# === RESULTS ===
Write-Host ""
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   BENCHMARK RESULTS" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

$maxScore = $results.total * 1  # Each test has weight, max per test is weight value
$percentage = [Math]::Round(($results.score / $maxScore) * 100)

Write-Host ""
Write-Host "Score: $($results.score) / $maxScore ($percentage%)" -ForegroundColor White
Write-Host "Passed: $($results.passed) / Failed: $($results.failed)" -ForegroundColor $(if($percentage -ge 95){'Green'}elseif($percentage -ge 85){'Yellow'}else{'Red'})
Write-Host ""

if ($percentage -ge 98) {
    Write-Host "🏆🏆🏆 LEGENDARY! 98%+ ACHIEVED! 🏆🏆🏆" -ForegroundColor Magenta
    Write-Host "Xavier is a STATE OF THE ART memory system!" -ForegroundColor Green
} elseif ($percentage -ge 95) {
    Write-Host "🌟🌟🌟 EXCELLENT! 95%+ Score! 🌟🌟🌟" -ForegroundColor Cyan
    Write-Host "Very close to state of the art performance!" -ForegroundColor Green
} elseif ($percentage -ge 90) {
    Write-Host "🌟🌟 GREAT! 90%+ Score!" -ForegroundColor Cyan
    Write-Host "Strong memory performance!" -ForegroundColor Green
} elseif ($percentage -ge 80) {
    Write-Host "👍 GOOD. 80%+ Score." -ForegroundColor Yellow
    Write-Host "Room for improvement." -ForegroundColor Yellow
} else {
    Write-Host "⚠️  NEEDS WORK. Below 80%." -ForegroundColor Red
}

Write-Host ""
Write-Host "Failed tests:" -ForegroundColor Yellow
foreach ($d in $results.details) {
    if ($d.status -ne "PASS") {
        Write-Host "  - $($d.name)" -ForegroundColor Red
    }
}
