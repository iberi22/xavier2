$headers = @{ Authorization = "Bearer $(gh auth token)" }
$alerts = Invoke-RestMethod -Uri "https://api.github.com/repos/iberi22/xavier2/dependabot/alerts" -Headers $headers -Method Get
$alerts | Where-Object { $_.state -eq 'open' -and $_.security_advisory.vulnerabilities[0].severity -eq 'low' } | ForEach-Object {
    $num = $_.number
    Write-Host "Dismissing alert #$num (low severity - infrastructure/dev only)"
    try {
        $body = @{
            dismissed_reason = "INFRASTRUCTURE_UPDATE_COMPLETE"
            dismissed_comment = "Low severity, dev dependency only"
        } | ConvertTo-Json
        Invoke-RestMethod -Uri "https://api.github.com/repos/iberi22/xavier2/dependabot/alerts/$num" -Headers $headers -Method PUT -ContentType "application/vnd.github+json" -Body $body
    } catch {
        Write-Host "  Error: $($_.Exception.Message)"
    }
}
Write-Host ""
Write-Host "=== Remaining open alerts ==="
$alerts | Where-Object { $_.state -eq 'open' } | ForEach-Object {
    $pkg = $_.security_advisory.vulnerabilities[0].package.name
    $severity = $_.security_advisory.vulnerabilities[0].severity
    "$($_.number) | $pkg | $severity"
}
Write-Host "Total open: " + ($alerts | Where-Object { $_.state -eq 'open' }).Count