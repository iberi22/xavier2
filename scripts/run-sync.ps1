$env:NODE_OPTIONS = '--max-old-space-size=128'
$out = "$env:TEMP\sync_out_$(Get-Random).txt"
$proc = Start-Process node -ArgumentList 'E:\scripts-python\xavier2\scripts\sync-all-to-xavier2.js' -NoNewWindow -PassThru -RedirectStandardOutput $out
$ok = $proc.WaitForExit(110000)
if ($ok) {
    Write-Host "Exit: $($proc.ExitCode)"
} else {
    Write-Host "TIMEOUT - killed"
    Stop-Process $proc.Id -Force -ErrorAction SilentlyContinue
}
Get-Content $out -ErrorAction SilentlyContinue
Remove-Item $out -Force -ErrorAction SilentlyContinue
