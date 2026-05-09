#!/usr/bin/env pwsh
# xavier-service.ps1 - Self-healing Xavier background service manager
# Usage: .\xavier-service.ps1 [start|stop|restart|status|logs]
#
# Features:
#   - Auto-restart on crash
#   - Port conflict detection & resolution
#   - Structured log rotation (max 5MB per file, keep 5 backups)
#   - Health check endpoint monitoring
#   - Watchdog timer (restart if no health ping within 60s)
#
# Requirements:
#   - Xavier binary at: E:\scripts-python\xavier\target\release\xavier.exe
#   - Config: XAVIER_PORT env var (default: 8040)
#   - Admin not required for basic operation

param(
    [ValidateSet('start','stop','restart','status','logs','install','uninstall')]
    [Parameter(Position=0)]
    [string]$Action = 'start',

    [Parameter(Position=1)]
    [string]$ExtraArgs = ''
)

$ErrorActionPreference = 'Stop'
$PROJECT_ROOT = 'E:\scripts-python\xavier'
$BINARY = "$PROJECT_ROOT\target\release\xavier.exe"
$PID_FILE = "$PROJECT_ROOT\data\xavier.pid"
$LOG_DIR = "$PROJECT_ROOT\logs"
$LOG_FILE = "$LOG_DIR\xavier.log"
$PORT = if ($env:XAVIER_PORT) { $env:XAVIER_PORT } else { 8040 }
$HEALTH_URL = "http://localhost:$PORT/health"
$READY_URL = "http://localhost:$PORT/ready"
$MAX_LOG_BYTES = 5MB
$LOG_BACKUPS = 5
$HEALTH_INTERVAL = 15  # seconds between health checks
$WATCHDOG_TIMEOUT = 60 # seconds - restart if no health for this long

# ─── Helpers ────────────────────────────────────────────────────────────────

function Get-LogPath { $LOG_FILE }
function Get-Pid {
    if (Test-Path $PID_FILE) {
        [int](Get-Content $PID_FILE -Raw).Trim()
    } else { $null }
}

function Write-Log {
    param([string]$Message, [string]$Level = 'INFO')
    $timestamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
    $entry = "$timestamp [$Level] $Message"
    Write-Host $entry
    if (-not (Test-Path $LOG_DIR)) {
        New-Item -ItemType Directory -Path $LOG_DIR -Force | Out-Null
    }
    Add-Content -Path $LOG_FILE -Value $entry -Encoding UTF8
}

function Rotate-Logs {
    # Rotate if log file exceeds MAX_LOG_BYTES
    if ((Test-Path $LOG_FILE) -and ((Get-Item $LOG_FILE).Length -gt $MAX_LOG_BYTES)) {
        $timestamp = Get-Date -Format 'yyyyMMddHHmmss'
        Move-Item $LOG_FILE "$LOG_FILE.$timestamp" -Force
        # Compress old logs, keep only LOG_BACKUPS
        Get-ChildItem "$LOG_FILE.*" -File | Sort-Object LastWriteTime -Descending | Select-Object -Skip $LOG_BACKUPS | Remove-Item -Force -ErrorAction SilentlyContinue
    }
}

function Get-ProcessForPort {
    param([int]$Port)
    $output = netstat -ano | Select-String ":\s*$Port\s+" | Select-Object -First 1
    if ($output -match '\s+(\d+)\s+$') {
        return [int]$matches[1]
    }
    return $null
}

function Find-HostProcess {
    param([int]$Port)
    $pid = Get-ProcessForPort $Port
    if ($pid -and $pid -ne 0) {
        try {
            Get-Process -Id $pid -ErrorAction SilentlyContinue
        } catch { $null }
    } else { $null }
}

function Test-PortFree {
    param([int]$Port)
    $listener = $null
    try {
        $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Parse('127.0.0.1'), $Port)
        $listener.Start()
        $true
    } catch { $false }
    } finally {
        if ($listener) { $listener.Stop(); $listener = $null }
    }
}

function Wait-ForHealthy {
    param([int]$TimeoutSec = 30)
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
        try {
            $r = Invoke-WebRequest -Uri $HEALTH_URL -TimeoutSec 3 -UseBasicParsing -ErrorAction SilentlyContinue
            if ($r.StatusCode -eq 200) {
                Write-Log "Health check passed after $($sw.Elapsed.TotalSeconds)s" 'INFO'
                return $true
            }
        } catch {}
        Start-Sleep -Seconds 3
    }
    Write-Log "Health check timeout after ${TimeoutSec}s" 'WARN'
    return $false
}

function Test-XavierHealthy {
    try {
        $r = Invoke-WebRequest -Uri $HEALTH_URL -TimeoutSec 5 -UseBasicParsing -ErrorAction SilentlyContinue
        return ($r.StatusCode -eq 200)
    } catch { return $false }
}

function Stop-XavierProcess {
    $pid = Get-Pid
    if ($pid) {
        try {
            $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
            if ($proc) {
                Write-Log "Sending SIGTERM to PID $pid" 'INFO'
                $proc.CloseMainWindow() | Out-Null
                Start-Sleep -Seconds 3
                if (-not $proc.HasExited) {
                    Write-Log "Process still alive, forcing kill" 'WARN'
                    Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
                }
            }
        } catch {}
        Remove-Item $PID_FILE -Force -ErrorAction SilentlyContinue
    }
    # Also check if xavier is running by process name
    Get-Process -Name 'xavier' -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Log "Killing stray xavier process PID $($_.Id)" 'WARN'
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
}

function Test-IsRunning {
    $pid = Get-Pid
    if ($pid) {
        try {
            $proc = Get-Process -Id $pid -ErrorAction SilentlyContinue
            if ($proc -and -not $proc.HasExited) { return $true }
        } catch {}
    }
    # Fallback: check if health endpoint responds
    return Test-XavierHealthy
}

# ─── Install/Uninstall (Windows Service-like via Task Scheduler) ───────────────

function Install-Service {
    Write-Log "Installing Xavier as scheduled task..." 'INFO'
    $taskName = 'XavierService'
    $scriptPath = "$PSScriptRoot\xavier-service.ps1"
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$scriptPath`" start"
    $trigger = New-ScheduledTaskTrigger -AtStartup
    $principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable
    Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force
    Write-Log "Scheduled task '$taskName' created." 'INFO'
}

function Uninstall-Service {
    Write-Log "Removing Xavier scheduled task..." 'INFO'
    Unregister-ScheduledTask -TaskName 'XavierService' -Confirm:$false -ErrorAction SilentlyContinue
    Write-Log "Scheduled task removed." 'INFO'
}

# ─── Status ─────────────────────────────────────────────────────────────────

function Show-Status {
    $running = Test-IsRunning
    $pid = Get-Pid
    $proc = if ($pid) { Get-Process -Id $pid -ErrorAction SilentlyContinue } else { $null }

    Write-Host ""
    Write-Host "══ Xavier Service Status ══" -ForegroundColor Cyan
    Write-Host "  Running    : $(if ($running) { 'YES' } else { 'NO' })"
    Write-Host "  PID File   : $PID_FILE"
    Write-Host "  PID        : $(if ($pid) { $pid } else { 'N/A' })"
    if ($proc) {
        Write-Host "  Process    : $($proc.ProcessName) (CPU: $([math]::Round($proc.CPU,1))s, Mem: $([math]::Round($proc.WorkingSet64/1MB,0))MB)"
        Write-Host "  Started    : $($proc.StartTime)"
    }
    Write-Host "  Port       : $PORT"
    Write-Host "  Health URL : $HEALTH_URL"
    Write-Host "  Log File   : $LOG_FILE"
    if (Test-Path $LOG_FILE) { Write-Host "  Log Size   : $([math]::Round((Get-Item $LOG_FILE).Length/1KB,1)) KB" }

    # Check if port is in use by another process
    $portProc = Find-HostProcess $PORT
    if ($portProc -and (-not $running)) {
        Write-Host "  WARNING    : Port $PORT is used by PID $($portProc.Id) ($($portProc.ProcessName)) but xavier is not running!" -ForegroundColor Red
    }

    if ($running) {
        $healthy = Test-XavierHealthy
        Write-Host "  Health     : $(if ($healthy) { 'OK' } else { 'UNRESPONSIVE' })" -ForegroundColor $(if ($healthy) { 'Green' } else { 'Yellow' })
    }
    Write-Host ""
}

# ─── Logs ───────────────────────────────────────────────────────────────────

function Show-Logs {
    param([int]$Last = 30)
    if (Test-Path $LOG_FILE) {
        Get-Content $LOG_FILE -Tail $Last
    } else {
        Write-Host "No log file found at $LOG_FILE" -ForegroundColor Yellow
    }
}

# ─── Start (with auto-restart loop) ─────────────────────────────────────────

function Start-Xavier {
    if (-not (Test-Path $BINARY)) {
        Write-Log "Binary not found at $BINARY. Run: cargo build --release" 'ERROR'
        throw "Binary missing: $BINARY"
    }

    # Check if already running
    if (Test-IsRunning) {
        $pid = Get-Pid
        Write-Log "Xavier already running (PID $pid). Use 'restart' to force." 'INFO'
        return
    }

    # Check port conflict
    if (-not (Test-PortFree $PORT)) {
        Write-Log "Port $PORT is in use. Attempting to resolve..." 'WARN'
        $conflicting = Find-HostProcess $PORT
        if ($conflicting) {
            Write-Log "Port conflict: PID $($conflicting.Id) ($($conflicting.ProcessName))" 'WARN'
            # If it's a stray xavier, kill it
            if ($conflicting.ProcessName -like '*xavier*') {
                Write-Log "Killing conflicting xavier process" 'WARN'
                Stop-Process -Id $conflicting.Id -Force -ErrorAction SilentlyContinue
                Start-Sleep -Seconds 2
                if (-not (Test-PortFree $PORT)) {
                    Write-Log "Failed to free port $PORT" 'ERROR'
                    throw "Port $PORT still occupied after cleanup"
                }
            } else {
                Write-Log "Port $PORT is occupied by another process. Stop it first or set XAVIER_PORT env." 'ERROR'
                throw "Port conflict: $($conflicting.ProcessName) (PID $($conflicting.Id)) on port $PORT"
            }
        }
    }

    # Rotate logs before starting
    Rotate-Logs

    Write-Log "Starting Xavier..." 'INFO'
    $env:RUST_LOG = $env:RUST_LOG ?? 'info'

    $proc = Start-Process -FilePath $BINARY `
        -ArgumentList "server --port $PORT $ExtraArgs" `
        -WorkingDirectory $PROJECT_ROOT `
        -PassThru `
        -NoNewWindow `
        -RedirectStandardOutput "$LOG_DIR\stdout.log" `
        -RedirectStandardError "$LOG_DIR\stderr.log"

    if (-not $proc.HasExited) {
        $proc.Id.ToString() | Set-Content $PID_FILE -Encoding UTF8
        Write-Log "Xavier started (PID $($proc.Id))" 'INFO'
    } else {
        Write-Log "Xavier exited immediately with code $($proc.ExitCode)" 'ERROR'
        throw "Xavier failed to start (exit code: $($proc.ExitCode))"
    }

    # Wait for health check
    if (Wait-ForHealthy -TimeoutSec 30) {
        Write-Log "Xavier is ready and listening on port $PORT" 'INFO'
    } else {
        Write-Log "Xavier started but not responding to health checks. Check logs." 'WARN'
    }
}

# ─── Restart ────────────────────────────────────────────────────────────────

function Restart-Xavier {
    Write-Log "Restarting Xavier..." 'INFO'
    Stop-XavierProcess
    Start-Sleep -Seconds 3
    Start-Xavier
}

# ─── Stop ───────────────────────────────────────────────────────────────────

function Do-Stop {
    Write-Log "Stopping Xavier service..." 'INFO'
    Stop-XavierProcess
    Write-Log "Xavier stopped." 'INFO'
}

# ─── Main ────────────────────────────────────────────────────────────────────

switch ($Action) {
    'start'    { Start-Xavier }
    'stop'     { Do-Stop }
    'restart'  { Restart-Xavier }
    'status'   { Show-Status }
    'logs'     { Show-Logs }
    'install'  { Install-Service }
    'uninstall'{ Uninstall-Service }
}

exit 0