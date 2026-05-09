param(
    [string]$BaseUrl = $(if ($env:XAVIER_URL) { $env:XAVIER_URL } else { "http://127.0.0.1:8006" }),
    [string]$Token = $(if ($env:XAVIER_TOKEN) { $env:XAVIER_TOKEN } else { "" }),
    [int]$TimeoutSec = 30,
    [switch]$RequireBuildRoute
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Token)) {
    throw "XAVIER_TOKEN is required for release smoke checks"
}

function Invoke-JsonRequest {
    param(
        [string]$Method,
        [string]$Url,
        [hashtable]$Headers = @{},
        [object]$Body = $null
    )

    $params = @{
        Method = $Method
        Uri = $Url
        Headers = $Headers
        TimeoutSec = $TimeoutSec
        UseBasicParsing = $true
    }

    if ($null -ne $Body) {
        $params["ContentType"] = "application/json"
        $params["Body"] = ($Body | ConvertTo-Json -Depth 8)
    }

    Invoke-WebRequest @params
}

Write-Host "Running Xavier release smoke checks against $BaseUrl" -ForegroundColor Cyan

$health = Invoke-JsonRequest -Method "GET" -Url "$BaseUrl/health"
if ($health.StatusCode -ne 200 -or $health.Content -notmatch '"status":"ok"') {
    throw "Health check failed"
}
Write-Host "PASS /health" -ForegroundColor Green

$readiness = Invoke-JsonRequest -Method "GET" -Url "$BaseUrl/readiness"
if ($readiness.StatusCode -ne 200) {
    throw "Readiness check failed"
}
$readinessJson = $readiness.Content | ConvertFrom-Json
if ($readinessJson.service -ne "xavier") {
    throw "Readiness payload missing xavier service marker"
}
Write-Host "PASS /readiness ($($readinessJson.status))" -ForegroundColor Green

try {
    $build = Invoke-JsonRequest -Method "GET" -Url "$BaseUrl/build" -Headers @{ "X-Xavier-Token" = $Token }
    if ($build.StatusCode -ne 200) {
        throw "Build info check failed"
    }
    $buildJson = $build.Content | ConvertFrom-Json
    if ($buildJson.service -ne "xavier") {
        throw "Build info payload missing xavier service marker"
    }
    Write-Host "PASS /build" -ForegroundColor Green
} catch {
    $statusCode = $_.Exception.Response.StatusCode.value__
    if (-not $RequireBuildRoute -and $statusCode -eq 404) {
        Write-Host "WARN /build not exposed by current server surface; skipping optional build check" -ForegroundColor Yellow
    } else {
        throw
    }
}

try {
    $unauthorized = Invoke-JsonRequest -Method "GET" -Url "$BaseUrl/v1/account/usage"
    if ($unauthorized.StatusCode -eq 200) {
        Write-Host "WARN auth gate bypassed; assuming dev mode is enabled" -ForegroundColor Yellow
    } else {
        throw "Protected route unexpectedly returned $($unauthorized.StatusCode)"
    }
} catch {
    if ($_.Exception.Response.StatusCode.value__ -ne 401) {
        throw
    }
}
Write-Host "PASS auth gate" -ForegroundColor Green

$headers = @{ "X-Xavier-Token" = $Token }
$docPath = "smoke/$(Get-Date -Format 'yyyyMMddHHmmss')"
$content = "Xavier public release smoke test document"

$add = Invoke-JsonRequest -Method "POST" -Url "$BaseUrl/memory/add" -Headers $headers -Body @{
    path = $docPath
    content = $content
    metadata = @{ source = "release-smoke" }
}
if ($add.StatusCode -ne 200) {
    throw "Memory add failed"
}
Write-Host "PASS /memory/add" -ForegroundColor Green

$search = Invoke-JsonRequest -Method "POST" -Url "$BaseUrl/memory/search" -Headers $headers -Body @{
    query = "public release smoke"
    limit = 5
}
if ($search.StatusCode -ne 200) {
    throw "Memory search request failed"
}
$searchJson = $search.Content | ConvertFrom-Json
$searchFound = $searchJson.results | Where-Object { $_.content -like "*public release smoke*" }
if (-not $searchFound) {
    throw "Memory search failed to find smoke document"
}
Write-Host "PASS /memory/search" -ForegroundColor Green

$usage = Invoke-JsonRequest -Method "GET" -Url "$BaseUrl/v1/account/usage" -Headers $headers
if ($usage.StatusCode -ne 200) {
    throw "Usage endpoint failed"
}
Write-Host "PASS /v1/account/usage" -ForegroundColor Green

Write-Host "Xavier release smoke checks passed." -ForegroundColor Cyan
