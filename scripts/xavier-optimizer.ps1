# Xavier Proactive Optimizer
# Runs every 20 minutes to improve Xavier and sync context

param(
    [int]$IntervalMinutes = 20
)

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

$LogFile = "E:\scripts-python\xavier\xavier-optimizer.log"

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $ts = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    "$ts [$Level] $Message" | Add-Content -Path $LogFile
    Write-Host "$ts [$Level] $Message"
}

function Test-XavierHealth {
    try {
        $h = Invoke-RestMethod -Uri "$XAVIER/health" -UseBasicParsing -TimeoutSec 5
        return $h.status -eq "ok"
    } catch { return $false }
}

function Get-XavierStats {
    try {
        $build = Invoke-RestMethod -Uri "$XAVIER/build" -Headers $headers -UseBasicParsing -TimeoutSec 10
        return $build
    } catch { return $null }
}

function Sync-WorkspaceContext {
    Write-Log "Syncing workspace context..."

    # Check xavier memories
    try {
        $memories = Invoke-RestMethod -Uri "$XAVIER/memory/list" -Headers $headers -UseBasicParsing -TimeoutSec 10
        $count = if ($memories.memories) { $memories.memories.Count } else { 0 }
        Write-Log "Xavier memories: $count"
    } catch {
        Write-Log "Could not list memories: $_" "WARN"
    }
}

function Run-Benchmark {
    Write-Log "Running memory benchmark..."

    # Reset
    try {
        $null = Invoke-RestMethod -Uri "$XAVIER/memory/reset" -Method POST -Headers $headers -ContentType "application/json" -Body '{}' -UseBasicParsing -TimeoutSec 10
    } catch {
        Write-Log "Reset failed: $_" "WARN"
        return
    }

    # Add test memories
    $testMemories = @(
        @{path="bench/person"; content="My name is Roberto Garcia and I work at TechCorp"; keywords=@("roberto","garcia")},
        @{path="bench/city"; content="I live in Buenos Aires Argentina"; keywords=@("buenos","aires")},
        @{path="bench/company"; content="Company Acme Corp is based in Miami Florida"; keywords=@("acme","miami")},
        @{path="bench/project"; content="Project Nova is in phase 3 with budget"; keywords=@("nova","phase","budget")},
        @{path="bench/tech"; content="API uses Python FastAPI and PostgreSQL"; keywords=@("python","fastapi","postgresql")}
    )

    $passed = 0
    $total = $testMemories.Count

    foreach ($mem in $testMemories) {
        try {
            $jsonBody = $mem | ConvertTo-Json -Compress
            $null = Invoke-RestMethod -Uri "$XAVIER/memory/add" -Method POST -Headers $headers -ContentType "application/json" -Body $jsonBody -UseBasicParsing -TimeoutSec 10
            Start-Sleep -Milliseconds 200

            $queryBody = @{query="What is my name and where do I live?"; limit=3; system3_mode="disabled"} | ConvertTo-Json -Compress
            $agent = Invoke-RestMethod -Uri "$XAVIER/agents/run" -Method POST -Headers $headers -ContentType "application/json" -Body $queryBody -UseBasicParsing -TimeoutSec 30

            $response = $agent.response.ToString().ToLower()
            $keywordsFound = 0
            foreach ($kw in $mem.keywords) {
                if ($response -like "*$kw*") { $keywordsFound++ }
            }

            $threshold = [Math]::Floor($mem.keywords.Count * 0.5)
            if ($keywordsFound -ge $threshold) {
                $passed++
                Write-Log "PASS $($mem.path)" "INFO"
            } else {
                Write-Log "FAIL $($mem.path)" "WARN"
            }
        } catch {
            Write-Log "ERROR $($mem.path): $($_.Exception.Message)" "ERROR"
        }
    }

    $score = [Math]::Round(($passed / $total) * 100)
    Write-Log "Benchmark score: $score% ($passed/$total)"
    return $score
}

function Check-ProjectUpdates {
    Write-Log "Checking project updates..."

    $projects = @(
        "E:\scripts-python\xavier",
        "E:\scripts-python\gestalt-rust",
        "E:\scripts-python\manteniapp"
    )

    foreach ($proj in $projects) {
        if (Test-Path $proj) {
            Push-Location $proj
            try {
                $status = git status --short 2>&1
                if ($status) {
                    $changeCount = ($status | Measure-Object -Line).Lines
                    Write-Log "CHANGES $($proj): $changeCount" "WARN"
                }
            } catch { }
            Pop-Location
        }
    }
}

# MAIN
Write-Log "=== Xavier Optimizer Started ===" "INFO"

# 1. Health Check
Write-Log "Checking Xavier health..."
if (Test-XavierHealth) {
    Write-Log "Xavier is healthy"
} else {
    Write-Log "Xavier is DOWN!" "ERROR"
}

# 2. Get Stats
$stats = Get-XavierStats
if ($stats) {
    Write-Log "Version: $($stats.version), Backend: $($stats.memory_store.selected_backend)"
}

# 3. Sync Context
Sync-WorkspaceContext

# 4. Run Benchmark
$score = Run-Benchmark

# 5. Check Projects
Check-ProjectUpdates

Write-Log "=== Optimizer Complete ===" "INFO"

return $score
