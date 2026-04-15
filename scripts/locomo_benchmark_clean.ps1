# LoCoMo Benchmark for Xavier2 - Isolated Tests
# Each test runs independently with fresh memory

$XAVIER2 = "http://localhost:8006"
$TOKEN = "dev-token"
$headers = @{"X-Xavier2-Token" = $TOKEN; "Content-Type" = "application/json"}

function Test-Isolated {
    param([string]$Name, [string]$Content, [string]$Query, [string]$Expected)

    # Fresh reset for each test
    $null = Invoke-RestMethod -Uri "$XAVIER2/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}'
    Start-Sleep -Milliseconds 500

    # Add single memory
    $null = Invoke-RestMethod -Uri "$XAVIER2/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body (@{
        path = "bench/$([guid]::NewGuid().ToString('N').Substring(0,8))"
        content = $Content
        metadata = @{test=$Name}
    } | ConvertTo-Json -Compress)

    # Query
    $resp = Invoke-RestMethod -Uri "$XAVIER2/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body (@{
        query = $Query
        limit = 2
        system3_mode = "disabled"
    } | ConvertTo-Json -Compress)

    $response = $resp.response
    $pass = $response -and $response.ToString().ToLower().Contains($Expected.ToLower())

    Write-Host "$Name : $(if($pass){'PASS ✅'}else{'FAIL ❌'})" -ForegroundColor $(if($pass){'Green'}else{'Red'})
    if (-not $pass) {
        Write-Host "       Expected: $Expected" -ForegroundColor Gray
        Write-Host "       Got: $($response.ToString().Substring(0, [Math]::Min(150, $response.ToString().Length)))" -ForegroundColor Yellow
    }

    return $pass
}

Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   LoCoMo Benchmark - Isolated Tests" -ForegroundColor Magenta
Write-Host "========================================" -ForegroundColor Magenta
Write-Host ""

$passed = 0
$total = 0

# SINGLE HOP
Write-Host "=== SINGLE HOP RECALL ===" -ForegroundColor Yellow
$total++; if (Test-Isolated "person_name" "My name is Bel" "What is my name?" "bel") { $passed++ }
$total++; if (Test-Isolated "city" "I live in Bogota" "Where do I live?" "bogota") { $passed++ }
$total++; if (Test-Isolated "company" "I work at Southwest AI Labs" "Where do I work?" "southwest") { $passed++ }

# TEMPORAL
Write-Host "`n=== TEMPORAL REASONING ===" -ForegroundColor Yellow
$total++; if (Test-Isolated "yesterday" "Yesterday I worked on Xavier2" "What did I work on yesterday?" "xavier2") { $passed++ }
$total++; if (Test-Isolated "after" "After the meeting I updated the docs" "What did I do after the meeting?" "updated") { $passed++ }
$total++; if (Test-Isolated "sequence" "First I wrote code, then I tested, finally I deployed" "What was the sequence?" "wrote") { $passed++ }

# MULTI-HOP
Write-Host "`n=== MULTI-HOP REASONING ===" -ForegroundColor Yellow
$content = "Bel works at Southwest AI Labs. Southwest AI Labs is in Colombia."
$total++; if (Test-Isolated "multihop" $content "Where does Bel work?" "southwest") { $passed++ }

# FACTS
Write-Host "`n=== FACTUAL RECALL ===" -ForegroundColor Yellow
$total++; if (Test-Isolated "capital" "The capital of France is Paris" "What is the capital of France?" "paris") { $passed++ }
$total++; if (Test-Isolated "version" "Project version is 0.4.1" "What version is the project?" "0.4") { $passed++ }

# ENTITY
Write-Host "`n=== ENTITY RECALL ===" -ForegroundColor Yellow
$total++; if (Test-Isolated "skills" "Rust and TypeScript are my main languages" "What languages?" "rust") { $passed++ }
$total++; if (Test-Isolated "project" "Project name is Xavier2" "What is the project name?" "xavier") { $passed++ }

# SUMMARY
Write-Host ""
Write-Host "========================================" -ForegroundColor Magenta
Write-Host "   RESULTS: $passed / $total  ($([Math]::Round($passed/$total*100))%)" -ForegroundColor White
Write-Host "========================================" -ForegroundColor Magenta

if ($passed -eq $total) { Write-Host "🏆 PERFECT!" -ForegroundColor Green }
elseif ($passed -ge ($total * 0.8)) { Write-Host "👍 EXCELLENT!" -ForegroundColor Cyan }
elseif ($passed -ge ($total * 0.6)) { Write-Host "👌 GOOD" -ForegroundColor Yellow }
else { Write-Host "⚠️  NEEDS WORK" -ForegroundColor Red }
