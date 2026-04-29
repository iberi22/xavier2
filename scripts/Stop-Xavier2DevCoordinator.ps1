# Stop Xavier2 Development Coordinator
$PID_FILE = "E:\scripts-python\xavier2\logs\dev-coordinator.pid"
$LOG_FILE = "E:\scripts-python\xavier2\logs\dev-coordinator.log"

if (Test-Path $PID_FILE) {
    $pid = Get-Content $PID_FILE -ErrorAction SilentlyContinue
    if ($pid -and (Get-Process -Id $pid -ErrorAction SilentlyContinue)) {
        Stop-Process -Id $pid -Force
        Write-Host "Stopped process $pid"
    }
    Remove-Item $PID_FILE -Force -ErrorAction SilentlyContinue
}

# Also kill by name pattern
Get-Process | Where-Object { $_.CommandLine -match "xavier2-dev-coordinator" } | ForEach-Object {
    Write-Host "Killed: $($_.Id)"
    Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
}

Write-Host "Coordinator stopped."
Write-Host "Logs available at: $LOG_FILE"
