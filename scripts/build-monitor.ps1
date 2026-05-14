<#
.SYNOPSIS
  Build Monitor for Xavier - CI en local con checks post-build
.DESCRIPTION
  Construye la imagen Docker de Xavier, verifica health endpoint,
  mide tiempos y reporta resultados.
.PARAMETER Tag
  Tag de la imagen (default: xavier:latest)
.PARAMETER SkipBuild
  Skip build, solo test image existente
#>

param(
    [string]$Tag = "xavier:latest",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date
$OkColor = "Green"
$WarnColor = "Yellow"
$ErrColor = "Red"
$InfoColor = "Cyan"

function Write-Status($emoji, $msg, $color) {
    Write-Host "${emoji} $msg" -ForegroundColor $color
}

function Write-Pass($m) { Write-Status "`PASS:" $m $OkColor }
function Write-Warn($m) { Write-Status "`WARN:" $m $WarnColor }
function Write-Fail($m) { Write-Status "`FAIL:" $m $ErrColor }

function Info($m) { Write-Status "`INFO:" $m $InfoColor }

# --- Pre-flight checks ---
Info "Pre-flight checks..."

# 1. Docker disponible
try {
    $dockerVer = docker version --format "{{.Server.Version}}" 2>$null
    if (-not $dockerVer) { throw "Docker no responde" }
    Write-Pass "Docker $dockerVer disponible"
} catch {
    Write-Fail "Docker no disponible: $_"
    exit 1
}

# 2. Dockerfile existe
$dfPath = Join-Path (Join-Path $PSScriptRoot "..") "Dockerfile"
if (-not (Test-Path $dfPath)) {
    Write-Fail "Dockerfile no encontrado en $dfPath"
    exit 1
}
Write-Pass "Dockerfile OK"

# 3. Puerto libre
$testPort = 8007
for ($p = 8007; $p -le 8090; $p++) {
    $inUse = netstat -ano | Select-String ":$p "
    if (-not $inUse) { $testPort = $p; break }
}
Info "Usando puerto $testPort"

# --- Build ---
$buildDuration = 0
$imgSize = ""

if (-not $SkipBuild) {
    Info "Building Docker image '$Tag'..."
    $buildStart = Get-Date

    $buildDir = Join-Path $PSScriptRoot ".."
    $buildOutput = docker build -t $Tag $buildDir 2>&1
    $buildDuration = ((Get-Date) - $buildStart).TotalSeconds

    if ($LASTEXITCODE -ne 0) {
        Write-Fail "Build FALLO ($($buildDuration.ToString('F1'))s)"

        $errors = $buildOutput | Select-String "error["
        if ($errors) {
            Write-Status "📋" "Errores de compilacion:" $ErrColor
            $errors | ForEach-Object { Write-Host "   $_" -ForegroundColor $ErrColor }
        }

        $autoFix = @()
        if ($buildOutput -match "pkg-config") {
            $autoFix += "Faltan dependencias -> agregar a Dockerfile: apt-get install -y pkg-config libssl-dev"
        }
        if ($buildOutput -match "is_multiple_of") {
            $autoFix += "Usar 'len() % 2 != 0' en lugar de 'is_multiple_of' (nightly feature)"
        }
        if ($buildOutput -match "failed to solve") {
            $autoFix += "Error del solver Docker -> docker system prune -f y reintentar"
        }
        if ($buildOutput -match "no space left on device") {
            $autoFix += "Disco lleno -> docker system prune -af o limpiar C:"
        }

        if ($autoFix.Count -gt 0) {
            Write-Status "`FIX:" "Posibles soluciones:" $WarnColor
            $autoFix | ForEach-Object { Write-Host "   -> $_" -ForegroundColor $WarnColor }
        }
        exit 1
    }

    Write-Pass "Build completado en $($buildDuration.ToString('F1'))s"
    $imgSize = docker images $Tag --format "{{.Size}}" 2>$null
    Info "Tamano: $imgSize"
} else {
    Write-Warn "Build saltado"
    $imgSize = docker images $Tag --format "{{.Size}}" 2>$null
}

# --- Smoke test ---
Info "Smoke test..."
$suffix = Get-Random -Minimum 1000 -Maximum 9999
$containerName = "xavier-monitor-$suffix"

try {
    docker run -d --name $containerName `
        -p ${testPort}:8006 `
        -e XAVIER_DEV_MODE=true `
        -e XAVIER_HOST=0.0.0.0 `
        $Tag http 8006 2>&1 | Out-Null

    Start-Sleep -Seconds 3

    $status = docker ps --filter name=$containerName --format "{{.Status}}" 2>$null
    if (-not $status) { throw "Container no esta running" }
    Write-Pass "Container running ($status)"

    $health = (New-Object System.Net.WebClient).DownloadString("http://localhost:${testPort}/health") 2>$null
    if (-not $health) { throw "Health endpoint no responde" }

    $healthObj = $health | ConvertFrom-Json
    if ($healthObj.status -ne "ok") { throw "Status no OK: $health" }
    Write-Pass "Health check OK -> $health"

    $version = docker exec $containerName /app/xavier --version 2>$null
    if ($version) { Write-Pass "Version: $version" }

    $totalDuration = ((Get-Date) - $startTime).TotalSeconds
    $testDuration = $totalDuration - $buildDuration
    Write-Status "`SUMMARY:" "=== RESUMEN ===" $OkColor
    Write-Pass "Build: $($buildDuration.ToString('F1'))s"
    Write-Pass "Tests: $($testDuration.ToString('F1'))s"
    Write-Pass "Total: $($totalDuration.ToString('F1'))s"
    Write-Pass "Imagen: $Tag ($imgSize)"

} catch {
    Write-Fail "Smoke test fallo: $_"
    $logs = docker logs $containerName 2>&1
    if ($logs) {
        Write-Status "📋" "Container logs (ultimas 10 lineas):" $ErrColor
        $logs -split "`n" | Select-Object -Last 10 | ForEach-Object { Write-Host "   $_" -ForegroundColor $ErrColor }
    }
    exit 1
} finally {
    docker rm -f $containerName 2>$null | Out-Null
    Info "Container $containerName eliminado"
}

Write-Host ""
Write-Status "`DONE:" "Build monitor completo - todo OK" $OkColor
