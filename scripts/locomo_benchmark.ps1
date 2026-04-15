# LoCoMo Benchmark for Xavier2
# Based on ACL 2024 Long Conversation Memory benchmark
# Tests: QA (recall), temporal reasoning, multi-hop

$ErrorActionPreference = "Continue"

$XAVIER2 = "http://localhost:8006"
$TOKEN = "dev-token"
$headers = @{"X-Xavier2-Token" = $TOKEN}

$results = @{
    total = 0
    passed = 0
    failed = 0
    details = @()
}

function Test-Case {
    param(
        [string]$Name,
        [string]$SetupMemory,
        [string]$Query,
        [string]$ExpectedSubstring,
        [string]$Description
    )

    $results.total++

    Write-Host ""
    Write-Host "[$($results.total)] $Name" -ForegroundColor Cyan
    Write-Host "  Setup: $SetupMemory" -ForegroundColor Gray
    Write-Host "  Query: $Query" -ForegroundColor Gray
    Write-Host "  Expected: $ExpectedSubstring" -ForegroundColor Gray

    # Add memory
    $null = Invoke-RestMethod -Uri "$XAVIER2/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body (@{
        path = "bench/$([guid]::NewGuid().ToString('N').Substring(0,8))"
        content = $SetupMemory
        metadata = @{benchmark = "locomo"; type = $Name}
    } | ConvertTo-Json -Compress)

    # Query via agents/run (uses LLM + memory)
    try {
        $resp = Invoke-RestMethod -Uri "$XAVIER2/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body (@{
            query = $Query
            limit = 3
            system3_mode = "disabled"
        } | ConvertTo-Json -Compress)

        $response = $resp.response

        if ($response -and $response.ToString().ToLower().Contains($ExpectedSubstring.ToLower())) {
            Write-Host "  Result: PASS ✅" -ForegroundColor Green
            $results.passed++
            $results.details += @{name=$Name; status="PASS"; response=$response}
            return $true
        } else {
            Write-Host "  Result: FAIL ❌" -ForegroundColor Red
            Write-Host "    Got: $($response.ToString().Substring(0, [Math]::Min(200, $response.ToString().Length)))" -ForegroundColor Yellow
            $results.failed++
            $results.details += @{name=$Name; status="FAIL"; response=$response}
            return $false
        }
    }
    catch {
        Write-Host "  Result: ERROR ❌ - $_" -ForegroundColor Red
        $results.failed++
        $results.details += @{name=$Name; status="ERROR"; error=$_.Exception.Message}
        return $false
    }
}

Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   LoCoMo Benchmark for Xavier2" -ForegroundColor Magenta
Write-Host "   Long Conversation Memory (ACL 2024)" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta

# Reset memories
Write-Host "`n[Setup] Resetting memories..." -ForegroundColor Yellow
$null = Invoke-RestMethod -Uri "$XAVIER2/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}'
Start-Sleep -Seconds 2

# === SINGLE-HOP RECALL ===
Write-Host "`n=== SINGLE-HOP RECALL ===" -ForegroundColor Yellow

Test-Case -Name "single_hop_person" `
    -SetupMemory "My name is Bel and I live in Bogotá. I work at Southwest AI Labs." `
    -Query "What is my name?" `
    -ExpectedSubstring "Bel" `
    -Description "Simple name recall"

Test-Case -Name "single_hop_location" `
    -SetupMemory "The capital of Colombia is Bogotá. It is located in Cundinamarca department." `
    -Query "What is the capital of Colombia?" `
    -ExpectedSubstring "Bogotá" `
    -Description "Simple fact recall"

Test-Case -Name "single_hop_company" `
    -SetupMemory "Southwest AI Labs is a software company. They build AI agents and memory systems." `
    -Query "What company builds AI agents and memory systems?" `
    -ExpectedSubstring "Southwest AI Labs" `
    -Description "Company name recall"

# === TEMPORAL REASONING ===
Write-Host "`n=== TEMPORAL REASONING ===" -ForegroundColor Yellow

Test-Case -Name "temporal_before" `
    -SetupMemory "Yesterday I worked on the Xavier2 project. Today I am testing Xavier2." `
    -Query "What did I work on yesterday?" `
    -ExpectedSubstring "Xavier2" `
    -Description "Temporal - what happened before"

Test-Case -Name "temporal_after" `
    -SetupMemory "Before the meeting, I prepared the slides. After the meeting, I updated the docs." `
    -Query "What did I do after the meeting?" `
    -ExpectedSubstring "updated" `
    -Description "Temporal - what happened after"

Test-Case -Name "temporal_sequence" `
    -SetupMemory "First I wrote the code. Then I ran the tests. Finally I deployed to production." `
    -Query "What was the sequence of actions?" `
    -ExpectedSubstring "code" `
    -Description "Temporal - sequence recall"

# === MULTI-HOP REASONING ===
Write-Host "`n=== MULTI-HOP REASONING ===" -ForegroundColor Yellow

Test-Case -Name "multihop_inference" `
    -SetupMemory "Bel works at Southwest AI Labs. Southwest AI Labs is located in Colombia. Bel's timezone is America/Bogota." `
    -Query "Where does Bel work and in what timezone?" `
    -ExpectedSubstring "Southwest" `
    -Description "Multi-hop - combine two facts"

Test-Case -Name "multihop_relationship" `
    -SetupMemory "Leonardo is a salesperson for ManteniApp. ManteniApp is a maintenance tracking SaaS. ManteniApp targets industrial companies in Chile." `
    -Query "What product does Leonardo sell and to whom?" `
    -ExpectedSubstring "ManteniApp" `
    -Description "Multi-hop - product and customer"

# === ENTITY RECALL ===
Write-Host "`n=== ENTITY RECALL ===" -ForegroundColor Yellow

Test-Case -Name "entity_project" `
    -SetupMemory "Project Xavier2 is a cognitive memory system. It uses SQLite and vector embeddings. Current version is 0.4.1." `
    -Query "What is Project Xavier2 and what version is it?" `
    -ExpectedSubstring "0.4" `
    -Description "Entity recall with details"

Test-Case -Name "entity_person_role" `
    -SetupMemory "Bela is the founder and lead developer. He has expertise in Rust, TypeScript, and AI systems." `
    -Query "Who is Bela and what are his skills?" `
    -ExpectedSubstring "Rust" `
    -Description "Person and skills recall"

# === CONTEXT INTEGRATION ===
Write-Host "`n=== CONTEXT INTEGRATION ===" -ForegroundColor Yellow

Test-Case -Name "context_summary" `
    -SetupMemory "Monday: Had a call with the team about the Xavier2 migration. Tuesday: Deployed Xavier2 to production. Wednesday: Found a bug in the vector search. Thursday: Filed a bug report. Friday: Started working on the fix." `
    -Query "Summarize what happened this week" `
    -ExpectedSubstring "Xavier2" `
    -Description "Long context summary"

# === ADVERSARIAL (Similar Names) ===
Write-Host "`n=== ADVERSARIAL RECALL ===" -ForegroundColor Yellow

Test-Case -Name "adversarial_disambiguation" `
    -SetupMemory "Sebastian (CEO) handles business strategy. Bel (Founder) handles technical development. They both work at Southwest AI Labs." `
    -Query "Who handles technical development?" `
    -ExpectedSubstring "Bel" `
    -Description "Disambiguate similar roles"

Test-Case -Name "adversarial_negation" `
    -SetupMemory "The system is NOT using SQLite for the main database. It IS using the vector backend. The old xavier2 WAS using file backend." `
    -Query "What backend is the system using?" `
    -ExpectedSubstring "vector" `
    -Description "Negation understanding"

# === RESULTS ===
Write-Host ""
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   LoCoMo Benchmark Results" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta
Write-Host ""
Write-Host "Total:  $($results.total)" -ForegroundColor White
Write-Host "Passed: $($results.passed)  $([Math]::Round($results.passed/$results.total*100))%" -ForegroundColor Green
Write-Host "Failed: $($results.failed)  $([Math]::Round($results.failed/$results.total*100))%" -ForegroundColor Red
Write-Host ""

if ($results.passed -eq $results.total) {
    Write-Host "🏆 PERFECT SCORE! Xavier2 memory is excellent." -ForegroundColor Green
} elseif ($results.passed -ge ($results.total * 0.7)) {
    Write-Host "👍 GOOD! Xavier2 memory is functional." -ForegroundColor Yellow
} else {
    Write-Host "⚠️  NEEDS WORK. Xavier2 memory has issues." -ForegroundColor Red
}

Write-Host ""
Write-Host "Failed tests:" -ForegroundColor Yellow
foreach ($d in $results.details) {
    if ($d.status -ne "PASS") {
        Write-Host "  - $($d.name): $($d.status)" -ForegroundColor Red
    }
}

# Return results
return $results
