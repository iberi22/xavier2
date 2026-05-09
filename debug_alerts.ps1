$headers = @{ Authorization = "Bearer $(gh auth token)"; Accept = "application/vnd.github+json" }
$base = "https://api.github.com/repos/iberi22/xavier/dependabot/alerts"

# Get full alert data to find the correct ID field
$alerts = Invoke-RestMethod -Uri $base -Headers $headers -Method Get
$alerts[0] | ConvertTo-Json -Depth 5
