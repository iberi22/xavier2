# Super Memory Benchmark v2 - Beat 98%
# Tests using the FULL hybrid RRF system (not just vector search)

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
    maxScore = 0
    details = @()
}

function Add-Memory {
    param([string]$Path, [string]$Content)
    try {
        $body = @{path=$Path; content=$Content; metadata=@{}} | ConvertTo-Json -Compress
        $null = Invoke-RestMethod -Uri "$XAVIER/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $body -UseBasicParsing -TimeoutSec 10
        return $true
    } catch {
        Write-Host "Add error: $_" -ForegroundColor Red
        return $false
    }
}

function Test-Query {
    param([string]$Name, [string]$MemoryPath, [string]$Content, [string]$Query, [string[]]$ExpectedKeywords, [int]$Weight = 1)

    $results.maxScore += $Weight
    Write-Host ""
    Write-Host "[$($results.maxScore)] $Name" -ForegroundColor Cyan

    # Add memory
    $added = Add-Memory -Path $MemoryPath -Content $Content
    if (-not $added) {
        Write-Host "  ❌ Add failed" -ForegroundColor Red
        $results.details += @{name=$Name; status="ERROR"; reason="add failed"}
        return
    }

    # Small delay for indexing
    Start-Sleep -Milliseconds 150

    # Use agents/run which uses full RRF hybrid search
    try {
        $agentBody = @{query=$Query; limit=5; system3_mode="disabled"} | ConvertTo-Json -Compress
        $agent = Invoke-RestMethod -Uri "$XAVIER/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body $agentBody -UseBasicParsing -TimeoutSec 30

        if ($agent -and $agent.response) {
            $responseText = $agent.response.ToString().ToLower()

            # Check keywords
            $keywordsFound = 0
            foreach ($kw in $ExpectedKeywords) {
                if ($responseText -like "*$($kw.ToLower())*") {
                    $keywordsFound++
                }
            }

            $matchRatio = $keywordsFound / $ExpectedKeywords.Count

            if ($matchRatio -ge 0.5) {
                $points = [Math]::Ceiling($Weight * $matchRatio)
                $results.score += $points
                $results.passed++
                Write-Host "  ✅ PASS (+$points/$Weight) Keywords: $keywordsFound/$($ExpectedKeywords.Count)" -ForegroundColor Green
                Write-Host "  Response: $($agent.response.ToString().Substring(0, [Math]::Min(80, $agent.response.ToString().Length)))..." -ForegroundColor Gray
                $results.details += @{name=$Name; status="PASS"; keywords=$keywordsFound; total=$ExpectedKeywords.Count}
            } else {
                $results.failed++
                Write-Host "  ❌ FAIL - Keywords: $keywordsFound/$($ExpectedKeywords.Count)" -ForegroundColor Red
                Write-Host "  Response: $($agent.response.ToString().Substring(0, [Math]::Min(100, $agent.response.ToString().Length)))" -ForegroundColor Yellow
                $results.details += @{name=$Name; status="FAIL"; keywords=$keywordsFound; total=$ExpectedKeywords.Count; query=$Query}
            }
        } else {
            $results.failed++
            Write-Host "  ❌ FAIL - No response from agent" -ForegroundColor Red
            $results.details += @{name=$Name; status="ERROR"; reason="no response"}
        }
    } catch {
        $results.failed++
        Write-Host "  ❌ FAIL - Exception: $($_.Exception.Message)" -ForegroundColor Red
        $results.details += @{name=$Name; status="ERROR"; reason=$_.Exception.Message}
    }
}

# === PREPARE ===
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   SUPER MEMORY BENCHMARK v2" -ForegroundColor Magenta
Write-Host "   Full RRF Hybrid System Test" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

Write-Host "`n[Setup] Resetting..." -ForegroundColor Yellow
try {
    $null = Invoke-RestMethod -Uri "$XAVIER/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}' -UseBasicParsing -TimeoutSec 10
    Write-Host "  ✅ Reset complete" -ForegroundColor Green
} catch {
    Write-Host "  ⚠️  Reset: $($_.Exception.Message)" -ForegroundColor Yellow
}
Start-Sleep 1

# === WARM UP OLLAMA ===
Write-Host "`n[Warmup] Ollama embedding model..." -ForegroundColor Yellow
try {
    $null = Invoke-RestMethod -Uri "http://localhost:11434/api/embeddings" -Method POST -ContentType "application/json" -Body (@{model="nomic-embed-text"; prompt="warmup"} | ConvertTo-Json -Compress) -UseBasicParsing -TimeoutSec 60
    Write-Host "  ✅ Ollama ready" -ForegroundColor Green
} catch {
    Write-Host "  ⚠️  Ollama warmup: $($_.Exception.Message)" -ForegroundColor Yellow
}

# === IDENTITY TESTS ===
Write-Host "`n=== IDENTITY ===" -ForegroundColor Yellow

Test-Query -Name "person_name" -MemoryPath "mem/person1" -Content "My name is Roberto Garcia and I work at TechCorp" -Query "What is my name?" -ExpectedKeywords @("roberto", "garcia") -Weight 2

Test-Query -Name "city_live" -MemoryPath "mem/city1" -Content "I live in Buenos Aires Argentina with my family" -Query "Where do I live?" -ExpectedKeywords @("buenos", "aires", "argentina") -Weight 2

Test-Query -Name "company_work" -MemoryPath "mem/company1" -Content "Company Acme Industries is based in Miami Florida" -Query "What company am I at?" -ExpectedKeywords @("acme", "miami") -Weight 2

# === PROJECT FACTS ===
Write-Host "`n=== PROJECT FACTS ===" -ForegroundColor Yellow

Test-Query -Name "project_status" -MemoryPath "mem/project1" -Content "Project Nova is in phase 3 with budget of 500000 dollars" -Query "What is the Nova project status?" -ExpectedKeywords @("nova", "phase", "500", "budget") -Weight 3

Test-Query -Name "repo_lang" -MemoryPath "mem/repo1" -Content "Repository gestalt-rust uses Rust programming language with 95 percent test coverage" -Query "What language is gestalt-rust written in?" -ExpectedKeywords @("rust", "language") -Weight 2

Test-Query -Name "docker_port" -MemoryPath "mem/docker1" -Content "Docker container xavier exposes port 8006 and uses 2GB RAM" -Query "What port does xavier expose?" -ExpectedKeywords @("8006", "port") -Weight 2

# === RELATIONSHIPS ===
Write-Host "`n=== RELATIONSHIPS ===" -ForegroundColor Yellow

Test-Query -Name "person_reports_to" -MemoryPath "mem/rel1" -Content "Maria is the CTO of DataFlow Inc. She reports to the CEO Juan." -Query "Who does Maria report to?" -ExpectedKeywords @("juan", "ceo") -Weight 3

Test-Query -Name "project_lead" -MemoryPath "mem/rel2" -Content "The Atlas project is led by Senior Engineer Lisa who works with DevOps" -Query "Who leads the Atlas project?" -ExpectedKeywords @("lisa", "atlas", "senior") -Weight 3

Test-Query -Name "tech_stack" -MemoryPath "mem/rel3" -Content "API service uses Python FastAPI framework connected to PostgreSQL database" -Query "What does the API service use?" -ExpectedKeywords @("python", "fastapi", "postgresql") -Weight 3

# === TEMPORAL ===
Write-Host "`n=== TEMPORAL ===" -ForegroundColor Yellow

Test-Query -Name "date_event" -MemoryPath "mem/date1" -Content "Meeting scheduled for March 15 2026 at 3pm in conference room B" -Query "When is the meeting?" -ExpectedKeywords @("march", "15", "2026", "3pm") -Weight 2

Test-Query -Name "sequence" -MemoryPath "mem/seq1" -Content "Workflow steps: first review requirements, second design architecture, third write code, fourth deploy to production" -Query "What are the workflow steps?" -ExpectedKeywords @("review", "design", "write", "deploy", "steps") -Weight 3

# === TECHNICAL ===
Write-Host "`n=== TECHNICAL ===" -ForegroundColor Yellow

Test-Query -Name "api_endpoint" -MemoryPath "mem/api1" -Content "REST API endpoint /api/v2/users returns JSON with fields id name email" -Query "What endpoint returns users?" -ExpectedKeywords @("api", "users", "json", "endpoint") -Weight 2

Test-Query -Name "error_meaning" -MemoryPath "mem/err1" -Content "Error code ERR_TIMEOUT means the operation exceeded the 30 second timeout limit" -Query "What does ERR_TIMEOUT mean?" -ExpectedKeywords @("timeout", "30", "exceeded", "seconds") -Weight 2

Test-Query -Name "cache_ttl" -MemoryPath "mem/cfg1" -Content "Feature flag ENABLE_CACHE is true with TTL of 3600 seconds" -Query "What is the cache TTL?" -ExpectedKeywords @("3600", "cache", "seconds") -Weight 2

# === DECISIONS ===
Write-Host "`n=== DECISIONS ===" -ForegroundColor Yellow

Test-Query -Name "decision_k8s" -MemoryPath "mem/dec1" -Content "Decision made: use Kubernetes for orchestration with auto-scaling between 2 and 10 replicas" -Query "What was decided about orchestration?" -ExpectedKeywords @("kubernetes", "scaling", "replicas") -Weight 3

Test-Query -Name "tradeoff" -MemoryPath "mem/dec2" -Content "Trade-off accepted: faster delivery preferred over perfect code quality for MVP phase" -Query "What trade-off was accepted?" -ExpectedKeywords @("faster", "delivery", "mvp", "quality") -Weight 3

# === CONTEXT RULES ===
Write-Host "`n=== CONTEXT RULES ===" -ForegroundColor Yellow

Test-Query -Name "spelling_rule" -MemoryPath "mem/ctx1" -Content "Always use British English spelling for documents going to London office" -Query "What spelling should be used for London?" -ExpectedKeywords @("british", "english", "london") -Weight 2

Test-Query -Name "security_rule" -MemoryPath "mem/ctx2" -Content "Security fixes must be deployed within 24 hours of being reported" -Query "How quickly must security fixes be deployed?" -ExpectedKeywords @("24", "hours", "security") -Weight 3

# === ENTITY RECALL ===
Write-Host "`n=== ENTITY RECALL ===" -ForegroundColor Yellow

Test-Query -Name "currencies" -MemoryPath "mem/ent1" -Content "Valid currencies list: USD EUR GBP JPY CHF AUD CAD" -Query "What currencies are valid?" -ExpectedKeywords @("usd", "eur", "gbp", "currencies") -Weight 2

Test-Query -Name "versions" -MemoryPath "mem/ent2" -Content "Supported versions are v1.0 v1.5 v2.0 v2.1 v3.0" -Query "What versions are supported?" -ExpectedKeywords @("v1", "v2", "v3", "versions") -Weight 2

# === SPECIFIC FACTS ===
Write-Host "`n=== SPECIFIC FACTS ===" -ForegroundColor Yellow

Test-Query -Name "phone_number" -MemoryPath "mem/fact1" -Content "My phone number is +1-555-123-4567 and my email is roberto@techcorp.com" -Query "What is my phone number?" -ExpectedKeywords @("555", "123", "4567", "phone") -Weight 2

Test-Query -Name "address" -MemoryPath "mem/fact2" -Content "Office address is 123 Main Street Suite 400 Downtown Miami FL 33101" -Query "What is the office address?" -ExpectedKeywords @("123", "main", "miami", "suite") -Weight 2

Test-Query -Name "cost" -MemoryPath "mem/fact3" -Content "Monthly cost is 199 dollars for pro tier and 499 dollars for enterprise tier" -Query "What are the monthly costs?" -ExpectedKeywords @("199", "499", "dollars", "pro", "enterprise") -Weight 3

# === FINAL RESULTS ===
Write-Host ""
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   BENCHMARK RESULTS" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

$percentage = [Math]::Round(($results.score / $results.maxScore) * 100)

Write-Host ""
Write-Host "Score: $($results.score) / $($results.maxScore) ($percentage%)" -ForegroundColor White
Write-Host "Tests passed: $($results.passed) / $($results.passed + $results.failed)" -ForegroundColor $(if($percentage -ge 95){'Green'}elseif($percentage -ge 85){'Yellow'}else{'Red'})
Write-Host ""

if ($percentage -ge 98) {
    Write-Host "🏆🏆🏆 LEGENDARY 98%+ ACHIEVED! 🏆🏆🏆" -ForegroundColor Magenta
    Write-Host "Xavier is STATE OF THE ART memory!" -ForegroundColor Green
} elseif ($percentage -ge 95) {
    Write-Host "🌟🌟🌟 EXCELLENT 95%+! 🌟🌟🌟" -ForegroundColor Cyan
    Write-Host "Outstanding memory performance!" -ForegroundColor Green
} elseif ($percentage -ge 90) {
    Write-Host "🌟🌟 GREAT 90%+!" -ForegroundColor Cyan
    Write-Host "Very strong memory system!" -ForegroundColor Green
} elseif ($percentage -ge 80) {
    Write-Host "👍 GOOD 80%+." -ForegroundColor Yellow
} else {
    Write-Host "⚠️  NEEDS WORK - Below 80%." -ForegroundColor Red
}

Write-Host ""
Write-Host "Failed tests:" -ForegroundColor Yellow
foreach ($d in $results.details) {
    if ($d.status -ne "PASS") {
        Write-Host "  - $($d.name)" -ForegroundColor Red
    }
}
