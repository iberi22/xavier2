param(
    [int]$IntervalSeconds = 900,
    [switch]$RebuildCortex,
    [switch]$StopWhenGreen,
    [switch]$Once
)

$ErrorActionPreference = "Continue"

$XavierRoot = "E:\scripts-python\xavier2"
$CortexRoot = "E:\scripts-python\cortex"
$LogRoot = Join-Path $XavierRoot "benchmark_results\loop"
New-Item -ItemType Directory -Force -Path $LogRoot | Out-Null

function Now-Stamp {
    Get-Date -Format "yyyyMMdd_HHmmss"
}

function Write-LoopLog {
    param([string]$Message)
    $line = "[{0}] {1}" -f (Get-Date -Format o), $Message
    Add-Content -Path (Join-Path $LogRoot "memory_triad_loop.log") -Value $line -Encoding utf8
    [Console]::WriteLine($line)
}

function Invoke-Step {
    param(
        [string]$Name,
        [string]$WorkingDirectory,
        [string]$Command,
        [int]$TimeoutSeconds = 900
    )

    $stamp = Now-Stamp
    $safeName = $Name -replace '[^a-zA-Z0-9_-]', '_'
    $outFile = Join-Path $LogRoot "$stamp-$safeName.out.log"
    $errFile = Join-Path $LogRoot "$stamp-$safeName.err.log"
    $stepScript = Join-Path $LogRoot "$stamp-$safeName.ps1"
    $wrappedCommand = "& { $Command; if (`$LASTEXITCODE -ne `$null) { exit `$LASTEXITCODE } else { exit 0 } }"
    Set-Content -Path $stepScript -Value $wrappedCommand -Encoding utf8

    Write-LoopLog "START $Name"
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = "cmd.exe"
    $startInfo.Arguments = '/d /c powershell.exe -NoProfile -ExecutionPolicy Bypass -File "{0}" 1> "{1}" 2> "{2}"' -f $stepScript, $outFile, $errFile
    $startInfo.WorkingDirectory = $WorkingDirectory
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    $null = $process.Start()

    $finished = $process.WaitForExit($TimeoutSeconds * 1000)
    if (-not $finished) {
        try { Stop-Process -Id $process.Id -Force } catch {}
        Write-LoopLog "TIMEOUT $Name after ${TimeoutSeconds}s"
        return @{
            name = $Name
            ok = $false
            exit_code = $null
            timeout = $true
            stdout = $outFile
            stderr = $errFile
        }
    }

    $process.WaitForExit()
    $exitCode = $process.ExitCode
    $ok = $exitCode -eq 0
    Write-LoopLog ("END {0} exit={1}" -f $Name, $exitCode)
    return @{
            name = $Name
            ok = $ok
            exit_code = $exitCode
        timeout = $false
        stdout = $outFile
        stderr = $errFile
    }
}

function Test-CargoBusy {
    return [bool](Get-Process cargo,rustc -ErrorAction SilentlyContinue)
}

function New-SkippedStep {
    param(
        [string]$Name,
        [string]$Reason
    )
    Write-LoopLog "SKIP $Name reason=$Reason"
    return @{
        name = $Name
        ok = $true
        skipped = $true
        reason = $Reason
        exit_code = $null
        timeout = $false
        stdout = $null
        stderr = $null
    }
}

function Get-LatestBenchmarkSummary {
    $latest = Get-ChildItem -Path (Join-Path $XavierRoot "benchmark_results") -Filter "memory_triad_*.json" -File |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $latest) {
        return $null
    }
    try {
        $json = Get-Content -Raw -Path $latest.FullName | ConvertFrom-Json
        return @{
            report = $latest.FullName
            cortex_memory = $json.summary.cortex.matched_expected
            xavier2_memory = $json.summary.xavier2.matched_expected
            engram_memory = $json.summary.engram.matched_expected
            cortex_code = $json.code_context.summary.cortex.matched_expected
            xavier2_code = $json.code_context.summary.xavier2.matched_expected
        }
    } catch {
        return @{
            report = $latest.FullName
            error = $_.Exception.Message
        }
    }
}

function Invoke-Cycle {
    $cycle = Now-Stamp
    Write-LoopLog "CYCLE $cycle begin"
    $results = @()

    if (Test-CargoBusy) {
        $reason = "cargo_or_rustc_already_running"
        $results += New-SkippedStep -Name "xavier2_code_graph_tests" -Reason $reason
        $results += New-SkippedStep -Name "xavier2_check" -Reason $reason
        $results += New-SkippedStep -Name "cortex_code_graph_tests" -Reason $reason
        $results += New-SkippedStep -Name "cortex_check" -Reason $reason
    } else {
        $results += Invoke-Step -Name "xavier2_code_graph_tests" -WorkingDirectory $XavierRoot -Command "cargo test -p code-graph" -TimeoutSeconds 300
        $results += Invoke-Step -Name "xavier2_check" -WorkingDirectory $XavierRoot -Command "cargo check -p xavier2 --bin xavier2" -TimeoutSeconds 600
        $results += Invoke-Step -Name "cortex_code_graph_tests" -WorkingDirectory $CortexRoot -Command "cargo test -p code-graph" -TimeoutSeconds 300
        $results += Invoke-Step -Name "cortex_check" -WorkingDirectory $CortexRoot -Command "cargo check -p xavier2 --features ci-safe --bin xavier2" -TimeoutSeconds 600
    }

    if ($RebuildCortex) {
        $results += Invoke-Step -Name "cortex_docker_build" -WorkingDirectory $CortexRoot -Command "docker compose build cortex" -TimeoutSeconds 1800
        $results += Invoke-Step -Name "cortex_docker_restart" -WorkingDirectory $CortexRoot -Command "docker compose up -d --force-recreate --no-deps cortex" -TimeoutSeconds 300
    }

    $results += Invoke-Step -Name "memory_triad_benchmark" -WorkingDirectory $XavierRoot -Command "python scripts\memory_triad_benchmark.py --start-xavier2" -TimeoutSeconds 600
    $summary = Get-LatestBenchmarkSummary

    $cycleReport = @{
        cycle = $cycle
        timestamp = (Get-Date).ToString("o")
        rebuild_cortex = [bool]$RebuildCortex
        steps = $results
        benchmark = $summary
    }
    $cyclePath = Join-Path $LogRoot "$cycle-cycle.json"
    $cycleReport | ConvertTo-Json -Depth 8 | Set-Content -Path $cyclePath -Encoding utf8

    $allStepsOk = @($results | Where-Object { -not $_.ok }).Count -eq 0
    $green = $false
    if ($summary -and -not $summary.error) {
        $green = $allStepsOk -and
            $summary.cortex_memory -eq 3 -and
            $summary.xavier2_memory -eq 3 -and
            $summary.xavier2_code -eq 5 -and
            $summary.cortex_code -eq 5
    }

    Write-LoopLog ("CYCLE {0} end all_steps_ok={1} green={2} report={3}" -f $cycle, $allStepsOk, $green, $cyclePath)
    return $green
}

Write-LoopLog "memory_triad_loop started interval=${IntervalSeconds}s rebuild_cortex=$([bool]$RebuildCortex) stop_when_green=$([bool]$StopWhenGreen) once=$([bool]$Once)"

while ($true) {
    $green = Invoke-Cycle
    if ($Once -or ($StopWhenGreen -and $green)) {
        break
    }
    Start-Sleep -Seconds $IntervalSeconds
}

Write-LoopLog "memory_triad_loop stopped"
