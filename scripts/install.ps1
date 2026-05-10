# Xavier - Windows Installer
# Ejecutar en PowerShell como Administrador

$ErrorActionPreference = "Stop"
$XavierDir = "$env:LOCALAPPDATA\Xavier"
$BinDir = "$XavierDir\bin"
$BinPath = "$BinDir\xavier.exe"
$DataDir = "$XavierDir\data"
$ConfigDir = "$XavierDir\config"
$SourceExe = "$PSScriptRoot\target\release\xavier.exe"

Write-Host "=== Xavier Windows Installer ===" -ForegroundColor Cyan

# 1. Verificar binario
if (-not (Test-Path $SourceExe)) {
    Write-Host "ERROR: No se encuentra target\release\xavier.exe" -ForegroundColor Red
    Write-Host "Ejecuta primero: cargo build --release --bin xavier -j 1" -ForegroundColor Yellow
    exit 1
}

# 2. Crear directorios
Write-Host "Creando directorios..." -ForegroundColor Yellow
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null
New-Item -ItemType Directory -Force -Path $ConfigDir | Out-Null

# 3. Copiar binario
Write-Host "Copiando binario..." -ForegroundColor Yellow
Copy-Item -Path $SourceExe -Destination $BinPath -Force
Write-Host "  xavier.exe -> $BinPath ($((Get-Item $BinPath).Length / 1MB) MB)" -ForegroundColor Green

# 4. Copiar config de ejemplo
$ConfigDest = "$ConfigDir\.env"
if (-not (Test-Path $ConfigDest)) {
    if (Test-Path "$PSScriptRoot\.env.example") {
        Copy-Item "$PSScriptRoot\.env.example" $ConfigDest
        Write-Host "  .env.example -> $ConfigDest (EDITALO con tus valores)" -ForegroundColor Green
    }
} else {
    Write-Host "  Config exists: $ConfigDest (skip)" -ForegroundColor Gray
}

# 5. Agregar al PATH
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$BinDir*") {
    Write-Host "Agregando al PATH de usuario..." -ForegroundColor Yellow
    [Environment]::SetEnvironmentVariable("PATH", "$BinDir;$UserPath", "User")
    Write-Host "  $BinDir agregado al PATH" -ForegroundColor Green
    Write-Host "  REINICIA la terminal para que surta efecto" -ForegroundColor Yellow
} else {
    Write-Host "  $BinDir ya está en el PATH" -ForegroundColor Gray
}

# 6. Setear XAVIER_TOKEN si no existe
$CurrentToken = [Environment]::GetEnvironmentVariable("XAVIER_TOKEN", "User")
if (-not $CurrentToken) {
    $NewToken = -join ((65..90) + (97..122) + (48..57) | Get-Random -Count 32 | % { [char]$_ })
    [Environment]::SetEnvironmentVariable("XAVIER_TOKEN", $NewToken, "User")
    Write-Host "  XAVIER_TOKEN generado y seteado" -ForegroundColor Green

    # También actualizar en el .env si existe
    if (Test-Path $ConfigDest) {
        $EnvContent = Get-Content $ConfigDest -Raw
        $EnvContent = $EnvContent -replace "XAVIER_TOKEN=.*", "XAVIER_TOKEN=$NewToken"
        Set-Content $ConfigDest $EnvContent
        Write-Host "  Token actualizado en $ConfigDest" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "=== Instalación completa ===" -ForegroundColor Cyan
Write-Host "Binario:     $BinPath" -ForegroundColor White
Write-Host "Datos:       $DataDir" -ForegroundColor White
Write-Host "Config:      $ConfigDir\.env" -ForegroundColor White
Write-Host ""
Write-Host "Ejecutar: xavier --help" -ForegroundColor Cyan
Write-Host "Servidor: xavier serve" -ForegroundColor Cyan
Write-Host "Modo dev:  xavier monitor" -ForegroundColor Cyan
