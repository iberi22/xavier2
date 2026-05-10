<#
.SYNOPSIS
    Xavier2 Installer — Windows one-liner
.DESCRIPTION
    Downloads and installs Xavier2 from GitHub Releases.
.PARAMETER Version
    Specific version to install (default: latest).
.PARAMETER InstallDir
    Installation directory (default: $env:LOCALAPPDATA\Xavier2).
.PARAMETER ConfigDir
    Configuration directory (default: $env:APPDATA\xavier2).
.PARAMETER DataDir
    Data directory (default: $env:LOCALAPPDATA\xavier2-data).
.PARAMETER AddToPath
    Add installation directory to user PATH.
.PARAMETER NoWizard
    Skip running the interactive setup wizard after install.
.EXAMPLE
    irm https://raw.githubusercontent.com/iberi22/xavier/main/install.ps1 | iex
    irm https://raw.githubusercontent.com/iberi22/xavier/main/install.ps1 | iex -args "-Version v0.6.0"
    irm https://raw.githubusercontent.com/iberi22/xavier/main/install.ps1 | iex -args "-InstallDir C:\xavier2 -NoWizard"
.NOTES
    Requires PowerShell 5.1+ or PowerShell Core 6+.
    Requires internet access to download from GitHub Releases.
#>

param(
    [string]$Version = "latest",
    [string]$InstallDir = "$env:LOCALAPPDATA\Xavier2",
    [string]$ConfigDir = "$env:APPDATA\xavier2",
    [string]$DataDir = "$env:LOCALAPPDATA\xavier2-data",
    [switch]$AddToPath,
    [switch]$NoWizard
)

# ── Configuration ──────────────────────────────────────────────
$Repo = "iberi22/xavier"
$GitHubApi = "https://api.github.com/repos/$Repo"
$ProgressPreference = 'SilentlyContinue'
$ErrorActionPreference = 'Stop'

# ── Helpers ────────────────────────────────────────────────────
function Write-Header { param([string]$Text) Write-Host "`n$([char]0x2588) $Text" -ForegroundColor Cyan }
function Write-Info   { param([string]$Text) Write-Host "$([char]0x2139) $Text" -ForegroundColor Cyan }
function Write-Success{ param([string]$Text) Write-Host "$([char]0x2713) $Text" -ForegroundColor Green }
function Write-Warn   { param([string]$Text) Write-Host "$([char]0x26A0) $Text" -ForegroundColor Yellow }
function Write-ErrorMsg { param([string]$Text) Write-Host "$([char]0x2717) $Text" -ForegroundColor Red }

function Write-Banner {
    Write-Host @"
$([char]0x001b)[36m$([char]0x001b)[1m
  ██╗  ██╗ █████╗ ██╗   ██╗██╗███████╗██████╗ 
  ╚██╗██╔╝██╔══██╗██║   ██║██║██╔════╝██╔══██╗
   ╚███╔╝ ███████║██║   ██║██║█████╗  ██████╔╝
   ██╔██╗ ██╔══██║╚██╗ ██╔╝██║██╔══╝  ██╔══██╗
  ██╔╝ ██╗██║  ██║ ╚████╔╝ ██║███████╗██║  ██║
  ╚═╝  ╚═╝╚═╝  ╚═╝  ╚═══╝  ╚═╝╚══════╝╚═╝  ╚═╝
$([char]0x001b)[0m
  Cognitive Memory Runtime for AI Agents
"@
}

# ── Main ───────────────────────────────────────────────────────
Write-Banner

# Resolve version
if ($Version -eq "latest") {
    Write-Info "Fetching latest version..."
    try {
        $Release = Invoke-RestMethod -Uri "$GitHubApi/releases/latest"
        $Version = $Release.tag_name -replace '^v', ''
    } catch {
        Write-ErrorMsg "Could not determine latest version. Try specifying one with -Version"
        exit 1
    }
}
Write-Success "Version: v$Version"

$Tag = "v$Version"
$TargetTriple = "x86_64-pc-windows-msvc"
$Archive = "xavier-v$Version-$TargetTriple.zip"
$DownloadUrl = "https://github.com/$Repo/releases/download/$Tag/$Archive"

# Create directories
$null = New-Item -ItemType Directory -Force -Path $InstallDir
$null = New-Item -ItemType Directory -Force -Path $ConfigDir
$null = New-Item -ItemType Directory -Force -Path $DataDir

# Download
Write-Header "Downloading Xavier2 v$Version..."
Write-Info "URL: $DownloadUrl"

$TempDir = Join-Path $env:TEMP "xavier2-install-$PID"
$null = New-Item -ItemType Directory -Force -Path $TempDir
$ZipPath = Join-Path $TempDir $Archive

try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -ErrorAction Stop
} catch {
    Write-ErrorMsg "Download failed. Check that version v$Version exists and has a Windows release."
    Write-ErrorMsg "Available releases: https://github.com/$Repo/releases"
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
    exit 1
}
Write-Success "Downloaded"

# Extract
Write-Header "Extracting..."
try {
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force
} catch {
    # Fallback for older PowerShell
    $shell = New-Object -ComObject Shell.Application
    $shell.Namespace($TempDir).CopyHere($shell.Namespace($ZipPath).Items(), 16)
}
Write-Success "Extracted"

# Find the extracted directory
$ExtractDir = Get-ChildItem -Path $TempDir -Directory | Select-Object -First 1
if (-not $ExtractDir) {
    Write-ErrorMsg "Could not find extracted directory"
    Get-ChildItem $TempDir
    exit 1
}

# Install binaries
Write-Header "Installing..."
$installedBins = @()
foreach ($Bin in @("xavier.exe", "xavier-installer.exe")) {
    $Src = Join-Path $ExtractDir.FullName $Bin
    $Dst = Join-Path $InstallDir $Bin
    if (Test-Path $Src) {
        Copy-Item $Src $Dst -Force
        $installedBins += $Dst
        Write-Success "Installed $Dst"
    } else {
        Write-Warn "$Bin not found in archive (optional)"
    }
}

# Add to PATH
if ($AddToPath -or $installedBins.Count -eq 0) {
    Write-Header "Updating PATH..."
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($currentPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$InstallDir", "User")
        $env:PATH = "$env:PATH;$InstallDir"
        Write-Success "Added $InstallDir to user PATH"
        Write-Warn "Restart your terminal for PATH changes to take effect"
    } else {
        Write-Info "$InstallDir is already in PATH"
    }
} else {
    Write-Warn ""
    Write-Warn "  $InstallDir is not in your PATH"
    Write-Warn "  Re-run with -AddToPath or add it manually:"
    Write-Warn "    setx PATH `"%PATH%;$InstallDir`""
    Write-Warn ""
}

# Set environment variables
[Environment]::SetEnvironmentVariable("XAVIER_CONFIG_PATH", "$ConfigDir\xavier2.config.json", "User")
[Environment]::SetEnvironmentVariable("XAVIER_DATA_DIR", $DataDir, "User")
$env:XAVIER_CONFIG_PATH = "$ConfigDir\xavier2.config.json"
$env:XAVIER_DATA_DIR = $DataDir

# Cleanup
Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue

# Run installer wizard
if (-not $NoWizard) {
    $InstallerPath = Join-Path $InstallDir "xavier-installer.exe"
    if (Test-Path $InstallerPath) {
        Write-Header "Running setup wizard..."
        Write-Info "Launching interactive configuration..."
        Write-Info ""
        & $InstallerPath
    } else {
        Write-Info "No installer binary found. Configure manually in $ConfigDir\xavier2.config.json"
    }
}

# ── Done ───────────────────────────────────────────────────────
Write-Host ""
Write-Host "$([char]0x2554)$('═' * 44)$([char]0x2557)" -ForegroundColor Green
Write-Host "$([char]0x2551)  Xavier2 Installation Complete!         $([char]0x2551)" -ForegroundColor Green
Write-Host "$([char]0x255A)$('═' * 44)$([char]0x255D)" -ForegroundColor Green
Write-Host ""
Write-Host "  Binary:  $InstallDir" -ForegroundColor White
Write-Host "  Config:  $ConfigDir\xavier2.config.json" -ForegroundColor White
Write-Host "  Data:    $DataDir" -ForegroundColor White
Write-Host ""
Write-Host "  Quick start:" -ForegroundColor Cyan
Write-Host "    xavier-installer       # Run setup wizard"
Write-Host "    xavier serve           # Start memory server"
Write-Host "    xavier tui             # Launch dashboard"
Write-Host "    xavier save -k episodic `"text`"  # Save memory"
Write-Host ""
Write-Host "  Docs: https://github.com/iberi22/xavier"
Write-Host ""
