$BaseUrl = "http://localhost:8003/mcp"

function Test-MCPMethod {
    param($Method, $Params = @{})

    $Body = @{
        jsonrpc = "2.0"
        id = 1
        method = $Method
        params = $Params
    } | ConvertTo-Json

    Write-Host "Testing MCP Method: $Method..." -ForegroundColor Cyan
    try {
        $Response = Invoke-RestMethod -Method Post -Uri $BaseUrl -ContentType "application/json" -Body $Body
        Write-Host "Response received:" -ForegroundColor Green
        $Response | ConvertTo-Json | Write-Host
        return $Response
    } catch {
        Write-Host "Error calling $Method: $_" -ForegroundColor Red
        return $null
    }
}

Write-Host "Starting MCP Verification..." -ForegroundColor Yellow

# 1. Initialize
$Init = Test-MCPMethod -Method "initialize"
if ($Init.result.serverInfo.name -ne "xavier-memory") {
    Write-Host "FAILED: Wrong server info" -ForegroundColor Red
    exit 1
}

# 2. List Tools
$Tools = Test-MCPMethod -Method "tools/list"
if ($Tools.result.tools.Count -lt 1) {
    Write-Host "FAILED: No tools found" -ForegroundColor Red
    exit 1
}

Write-Host "`nAll MCP checks PASSED!" -ForegroundColor Green
