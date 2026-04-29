# Install Xavier2 Development Coordinator as Windows Task
# Run as: powershell -File Install-Xavier2DevTask.ps1

$TASK_NAME = "Xavier2DevelopmentCoordinator"
$SCRIPT_PATH = "E:\scripts-python\xavier2\scripts\xavier2-dev-coordinator.ps1"
$LOG_DIR = "E:\scripts-python\xavier2\logs"

# Ensure log directory exists
if (!(Test-Path $LOG_DIR)) {
    New-Item -ItemType Directory -Path $LOG_DIR -Force | Out-Null
}

# Check if task already exists
$existingTask = Get-ScheduledTask -TaskName $TASK_NAME -ErrorAction SilentlyContinue

if ($existingTask) {
    Write-Host "Task '$TASK_NAME' already exists. Removing..."
    Unregister-ScheduledTask -TaskName $TASK_NAME -Confirm:$false
}

# Create action
$action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "-NoExit -ExecutionPolicy Bypass -File `"$SCRIPT_PATH`" -Mode continuous -CycleMinutes 30"

# Create trigger (every 30 minutes, indefinitely)
$trigger = New-ScheduledTaskTrigger -Once -At (Get-Date) -RepetitionInterval (New-TimeSpan -Minutes 30) -RepetitionDuration ([TimeSpan]::MaxValue)

# Create settings
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -RunOnlyIfNetworkAvailable:$false -DontStopOnIdleEnd

# Create principal (run as current user)
$principal = New-ScheduledTaskPrincipal -UserId ([System.Security.Principal.WindowsIdentity]::GetCurrent().Name) -LogonType Interactive -RunLevel Limited

# Register task
try {
    Register-ScheduledTask -TaskName $TASK_NAME -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Description "Xavier2 Development Coordinator - Persistent development loop for Xavier2 project" | Out-Null
    Write-Host "✅ Task '$TASK_NAME' registered successfully!"
    Write-Host ""
    Write-Host "To view task:"
    Write-Host "  Get-ScheduledTask -TaskName '$TASK_NAME'"
    Write-Host ""
    Write-Host "To start immediately:"
    Write-Host "  Start-ScheduledTask -TaskName '$TASK_NAME'"
    Write-Host ""
    Write-Host "To remove:"
    Write-Host "  Unregister-ScheduledTask -TaskName '$TASK_NAME' -Confirm:`$false"
} catch {
    Write-Host "❌ Failed to register task: $_"
    Write-Host ""
    Write-Host "Alternative: Run manually with:"
    Write-Host "  powershell -File $SCRIPT_PATH -Mode continuous"
}
