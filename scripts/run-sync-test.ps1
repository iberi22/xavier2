$env:NODE_OPTIONS = '--max-old-space-size=128'
$start = Get-Date
$out = [System.IO.Path]::GetTempFileName()
$err = [System.IO.Path]::GetTempFileName()
$proc = Start-Process node -ArgumentList 'E:\scripts-python\xavier\scripts\sync-all-to-xavier.js' -NoNewWindow -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
$ok = $proc.WaitForExit(110000)
$elapsed = (Get-Date) - $start
if ($ok) {
    Write-Host "EXIT: $($proc.ExitCode) in $($elapsed.TotalSeconds)s"
    Get-Content $out
} else {
    Write-Host "TIMEOUT - killing process"
    Stop-Process $proc.Id -Force -ErrorAction SilentlyContinue
}
if ((Get-Item $err).Length -gt 0) { Get-Content $err }
Remove-Item $out,$err -Force -ErrorAction SilentlyContinue
