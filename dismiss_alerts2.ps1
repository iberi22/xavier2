$headers = @{ Authorization = "Bearer $(gh auth token)"; Accept = "application/vnd.github+json" }
$base = "https://api.github.com/repos/iberi22/xavier2/dependabot/alerts"

# Dismiss all open low-severity alerts
$lowSeverity = @(26, 13)
foreach ($num in $lowSeverity) {
    try {
        $body = @{ dismissed_reason = "not_used"; dismissed_comment = "Dev dependency, no production impact" } | ConvertTo-Json
        Invoke-RestMethod -Uri "$base/$($num)" -Headers $headers -Method PUT -ContentType "application/vnd.github+json" -Body $body
        Write-Host "Dismissed #$num"
    } catch {
        Write-Host "Error #$num : $($_.Exception.Message)"
    }
}

# Check remaining
Start-Sleep 2
$alerts = Invoke-RestMethod -Uri $base -Headers $headers -Method Get
Write-Host ""
Write-Host "Remaining open: " + ($alerts | Where-Object { $_.state -eq 'open' }).Count
$alerts | Where-Object { $_.state -eq 'open' } | ForEach-Object {
    $pkg = $_.security_advisory.vulnerabilities[0].package.name
    "$($_.number) | $pkg"
}