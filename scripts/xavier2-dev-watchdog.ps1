# Xavier2 Development Watchdog
# Persistent development loop for Xavier2 project improvement
# CEO: Xavier2 (Agent)

param(
    [int]$IntervalSeconds = 600,  # 10 minutes default
    [int]$MaxRuntimeHours = 24
)

$ErrorActionPreference = "Continue"
$XAVIER2_URL = "http://localhost:8006"
$TOKEN = "dev-token"
$REPO = "iberi22/xavier2"
$LOG_FILE = "E:\scripts-python\xavier2\logs\dev-watchdog.log"
$START_TIME = Get-Date
$END_TIME = $START_TIME.AddHours($MaxRuntimeHours)

# Ensure log directory exists
$logDir = Split-Path $LOG_FILE -Parent
if (!(Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir -Force | Out-Null }

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    "$timestamp [$Level] $Message" | Tee-Object -FilePath $LOG_FILE -Append
}

function Test-Xavier2Health {
    try {
        $resp = Invoke-RestMethod "$XAVIER2_URL/health" -TimeoutSec 5
        return $resp.status -eq "ok"
    } catch {
        return $false
    }
}

function Get-GitHubIssues {
    # Using web fetch to avoid gh CLI issues
    $url = "https://api.github.com/repos/$REPO/issues?state=open&per_page=10&sort=updated"
    try {
        $issues = Invoke-RestMethod $url -TimeoutSec 10
        return $issues | Where-Object { $_.pull_request -eq $null } | Select-Object number, title, labels, state, created_at
    } catch {
        Write-Log "Failed to fetch GitHub issues: $_" "WARN"
        return @()
    }
}

function Save-ToXavier2 {
    param([string]$Content, [string]$Path)
    try {
        $body = @{ path = $Path; content = $Content } | ConvertTo-Json -Compress
        $temp = [System.IO.Path]::GetTempFileName() + ".json"
        [System.IO.File]::WriteAllText($temp, $body)
        $null = Invoke-RestMethod "$XAVIER2_URL/memory/add" -Method POST -Body $temp -ContentType "application/json" -Headers @{ "X-Xavier2-Token" = $TOKEN } -TimeoutSec 10
        Remove-Item $temp -Force -ErrorAction SilentlyContinue
        return $true
    } catch {
        Write-Log "Failed to save to Xavier2: $_" "WARN"
        return $false
    }
}

function Get-Xavier2Stats {
    try {
        $stats = Invoke-RestMethod "$XAVIER2_URL/memory/stats" -Headers @{ "X-Xavier2-Token" = $TOKEN } -TimeoutSec 5
        return $stats
    } catch {
        return $null
    }
}

# Main development loop
Write-Log "=== Xavier2 Development Watchdog Started ===" "INFO"
Write-Log "Interval: $IntervalSeconds seconds | Max Runtime: $MaxRuntimeHours hours" "INFO"
Write-Log "Target: $REPO" "INFO"

while ((Get-Date) -lt $END_TIME) {
    $loopStart = Get-Date
    
    Write-Log "--- Development Cycle ---" "INFO"
    
    # 1. Check Xavier2 health
    $healthy = Test-Xavier2Health
    Write-Log "Xavier2 Health: $(if($healthy){'OK'}else{'FAILED'})" $(if($healthy){'INFO'}else{'WARN'})
    
    # 2. Get current stats
    $stats = Get-Xavier2Stats
    if ($stats) {
        Write-Log "Memory Stats: $($stats.total_memories) memories, $($stats.total_tags) tags" "INFO"
    }
    
    # 3. Check GitHub issues
    $issues = Get-GitHubIssues
    $p1Issues = $issues | Where-Object { $_.labels.name -contains "P1" -or $_.title -match "P0|P1" }
    $julesIssues = $issues | Where-Object { $_.labels.name -contains "jules" }
    
    Write-Log "Open Issues: $($issues.Count) | P1/P0: $($p1Issues.Count) | Jules: $($julesIssues.Count)" "INFO"
    
    # 4. Build status report
    $statusReport = @"
# Xavier2 Development Status

**Updated:** $(Get-Date -Format "yyyy-MM-dd HH:mm")

## System Health
- Xavier2 Service: $(if($healthy){'✅ OK'}else{'❌ DOWN'})
- Memory Memories: $($stats.total_memories)
- Memory Tags: $($stats.total_tags)

## GitHub Issues
- Total Open: $($issues.Count)
- P1/P0 Critical: $($p1Issues.Count)
- Assigned to Jules: $($julesIssues.Count)

## Priority Issues
$(
    if ($p1Issues) {
        $p1Issues | ForEach-Object { "- [#$($_.number)] $($_.title)" } | Out-String
    } else {
        "None"
    }
)

## Recent Activity
$(
    $issues | Select-Object -First 5 | ForEach-Object { "- #$($_.number): $($_.title)" } | Out-String
)

## Development Loop
- Last Check: $(Get-Date -Format "HH:mm:ss")
- Next Check: In $IntervalSeconds seconds
"@

    # 5. Save status to Xavier2 memory
    $saved = Save-ToXavier2 -Content $statusReport -Path "xavier2/development/status"
    Write-Log "Status saved to Xavier2: $($saved)" "INFO"
    
    # 6. Calculate sleep time
    $elapsed = (Get-Date) - $loopStart
    $sleepTime = [Math]::Max(0, $IntervalSeconds - $elapsed.TotalSeconds)
    
    if ($sleepTime -gt 0) {
        Write-Log "Sleeping $sleepTime seconds..." "INFO"
        Start-Sleep -Seconds $sleepTime
    }
}

Write-Log "=== Xavier2 Development Watchdog Finished ===" "INFO"
