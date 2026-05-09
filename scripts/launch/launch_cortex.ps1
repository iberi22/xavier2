# launch_xavier.ps1 - Lanza Xavier y espera a que esté listo

param(
    [switch]$Force
)

Write-Host "🚀 Launching Xavier..." -ForegroundColor Cyan

# Cambiar al directorio de Xavier
Set-Location "E:\scripts-python\xavier"

# Verificar si ya está corriendo
try {
    $response = Invoke-WebRequest -Uri "http://localhost:8003/health" -UseBasicParsing -TimeoutSec 2 -ErrorAction SilentlyContinue
    if ($response.StatusCode -eq 200) {
        Write-Host "✅ Xavier already running" -ForegroundColor Green
        exit 0
    }
} catch {
    # No está corriendo
}

# Verificar Docker
$docker = Get-Command docker -ErrorAction SilentlyContinue
if (-not $docker) {
    Write-Host "❌ Docker not found" -ForegroundColor Red
    exit 1
}

# Verificar si hay docker-compose
if (Test-Path "docker-compose.yml") {
    Write-Host "📦 Starting with docker-compose..." -ForegroundColor Yellow

    docker-compose up -d

    # Esperar a que esté listo
    Write-Host "⏳ Waiting for Xavier..." -ForegroundColor Yellow

    for ($i = 0; $i -lt 30; $i++) {
        try {
            $response = Invoke-WebRequest -Uri "http://localhost:8003/health" -UseBasicParsing -TimeoutSec 2 -ErrorAction SilentlyContinue
            if ($response.StatusCode -eq 200) {
                Write-Host "✅ Xavier is ready!" -ForegroundColor Green

                # Guardar en memoria
                $memoryData = @{
                    content = "Xavier launched automatically at $(Get-Date -Format 'yyyy-MM-dd HH:mm')"
                    path = "system/xavier"
                    metadata = @{type = "health_check"; status = "online"}
                } | ConvertTo-Json -Depth 3

                try {
                    Invoke-WebRequest -Uri "http://localhost:8003/memory/add" -Method POST -Headers @{"X-Xavier-Token"="dev"; "Content-Type"="application/json"} -Body $memoryData -UseBasicParsing -ErrorAction SilentlyContinue
                } catch {}

                exit 0
            }
        } catch {}

        Start-Sleep -Seconds 2
    }

    Write-Host "❌ Xavier failed to start" -ForegroundColor Red
    exit 1
}

Write-Host "❌ docker-compose.yml not found" -ForegroundColor Red
exit 1
