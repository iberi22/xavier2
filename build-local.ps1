# build-local.ps1 - Build Xavier2 binary locally, package in Docker
# This avoids compiling 500+ crates inside Docker (10-15 min -> ~30 sec)

param(
    [ValidateSet("windows", "linux")]
    [string]$Platform = "linux",
    [switch]$SkipFrontend
)

$ErrorActionPreference = "Stop"
$StartTime = Get-Date

# Determine Rust target
if ($Platform -eq "linux") {
    $rustTarget = "x86_64-unknown-linux-gnu"
} else {
    $rustTarget = "x86_64-pc-windows-msvc"
}

Write-Host "=== Xavier2 Local Build ===" -ForegroundColor Cyan
Write-Host "Platform: $Platform ($rustTarget)" -ForegroundColor Gray
Write-Host ""

$ErrorActionPreference = "Stop"
$StartTime = Get-Date

Write-Host "=== Xavier2 Local Build ===" -ForegroundColor Cyan
Write-Host "Target: $Target" -ForegroundColor Gray
Write-Host ""

# Check Rust toolchain
Write-Host "Checking Rust toolchain..." -ForegroundColor Yellow
$rustc = rustc --version 2>$null
$cargo = cargo --version 2>$null
if (-not $rustc -or -not $cargo) {
    Write-Error "Rust toolchain not found. Please install Rust first."
    exit 1
}
Write-Host "  $rustc" -ForegroundColor Green
Write-Host "  $cargo" -ForegroundColor Green

# Build frontend if not skipped
if (-not $SkipFrontend) {
    Write-Host ""
    Write-Host "Building frontend..." -ForegroundColor Yellow
    Push-Location $PSScriptRoot
    try {
        npm run build --workspace panel-ui
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Frontend build failed"
            exit 1
        }
    }
    finally {
        Pop-Location
    }
    Write-Host "  Frontend built successfully" -ForegroundColor Green
}

# Build Rust binary
Write-Host ""
Write-Host "Building Rust binary for $rustTarget..." -ForegroundColor Yellow
Write-Host "(First run: ~10-15 min. Subsequent runs: ~2-5 min with sccache)" -ForegroundColor Gray
Push-Location $PSScriptRoot
try {
    # Cross-compile for Linux target
    cargo build --release `
        --target $rustTarget `
        --workspace `
        --features ci-safe `
        --exclude xavier2-web `
        --bin xavier2 `
        --bin xavier2-tui
    
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Rust build failed"
        exit 1
    }
    
    # Copy binaries to dist/ for Docker build
    Write-Host ""
    Write-Host "Copying binaries to dist/..." -ForegroundColor Yellow
    if (-not (Test-Path "dist")) {
        New-Item -ItemType Directory -Path "dist" | Out-Null
    }
    
    $targetDir = "target/$rustTarget/release"
    Copy-Item -Path "$targetDir/xavier2" -Destination "dist/xavier2" -Force
    Copy-Item -Path "$targetDir/xavier2-tui" -Destination "dist/xavier2-tui" -Force
    
    Write-Host "  dist/xavier2 ($rustTarget)" -ForegroundColor Green
    Write-Host "  dist/xavier2-tui ($rustTarget)" -ForegroundColor Green
}
finally {
    Pop-Location
}

$elapsed = (Get-Date) - $StartTime
Write-Host ""
Write-Host "=== Build Complete ===" -ForegroundColor Green
Write-Host "  Binary: target/release/xavier2" -ForegroundColor Gray
Write-Host "  TUI Binary: target/release/xavier2-tui" -ForegroundColor Gray
Write-Host "  Time: $($elapsed.TotalSeconds) seconds" -ForegroundColor Gray
Write-Host ""
Write-Host "To build Docker image, run:" -ForegroundColor Cyan
Write-Host "  docker build -t xavier2:local ." -ForegroundColor White
