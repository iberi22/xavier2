$headers = @{ Authorization = "Bearer $(gh auth token)" }
$alerts = Invoke-RestMethod -Uri "https://api.github.com/repos/iberi22/xavier2/dependabot/alerts" -Headers $headers -Method Get
$alerts | Where-Object { $_.state -eq 'open' } | ForEach-Object {
    $pkg = $_.security_advisory.vulnerabilities[0].package.name
    $severity = $_.security_advisory.vulnerabilities[0].severity
    $fixed = $_.security_advisory.vulnerabilities[0].first_patched_version.identifier
    "$($_.number) | $pkg | $severity | fixed: $fixed"
}
"Total open: " + ($alerts | Where-Object { $_.state -eq 'open' }).Count