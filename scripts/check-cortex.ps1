$envVars = docker inspect xavier2 --format '{{json .Config.Env}}' | ConvertFrom-Json
Write-Host "=== Xavier2 MEMORY/SURREALDB env vars ==="
foreach ($e in $envVars) {
    if ($e -match 'MEMORY_BACKEND|SURREALDB') {
        Write-Host $e
    }
}

Write-Host ""
Write-Host "=== Full start command ==="
$cmd = docker inspect xavier2 --format '{{json .Config.Cmd}}'
Write-Host "Cmd: $cmd"

$entrypoint = docker inspect xavier2 --format '{{json .Config.Entrypoint}}'
Write-Host "Entrypoint: $entrypoint"

Write-Host ""
Write-Host "=== Check if xavier2 is using correct image ==="
$image = docker inspect xavier2 --format '{{.Config.Image}}'
Write-Host "Image: $image"

Write-Host ""
Write-Host "=== Try to use surreal CLI to check data ==="
$sql = 'SELECT id, workspace_id, path, content FROM memory_records;'
$bytes = [System.Text.Encoding]::UTF8.GetBytes($sql)
$memStream = New-Object System.IO.MemoryStream
$memStream.Write($bytes, 0, $bytes.Length)
$memStream.Position = 0

try {
    $result = docker exec -i surrealdb /surreal sql --endpoint ws://localhost:8000 --namespace xavier2 --database memory --username root --password root --pretty 2>&1
    Write-Host "surreal sql result: $result"
} catch {
    Write-Host "Error: $_"
}
