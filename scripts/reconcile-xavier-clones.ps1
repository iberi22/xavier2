param(
    [switch]$Execute,
    [string]$Root = 'E:\scripts-python',
    [string]$ArchiveName = 'xavier-reconcile-20260508'
)

$ErrorActionPreference = 'Stop'

$archiveRoot = Join-Path $Root (Join-Path '_archive' $ArchiveName)
$candidates = @(
    'xavier-new',
    'xavier-temp',
    'xavier_temp_reclone',
    'xavier-clone',
    'xavier-work',
    'temp_xavier_check'
)

Write-Host "Archive root: $archiveRoot"
Write-Host "Mode: $(if ($Execute) { 'EXECUTE' } else { 'DRY-RUN' })"

if ($Execute -and -not (Test-Path $archiveRoot)) {
    New-Item -ItemType Directory -Force -Path $archiveRoot | Out-Null
}

foreach ($name in $candidates) {
    $path = Join-Path $Root $name
    if (-not (Test-Path $path)) {
        Write-Host "SKIP missing: $path"
        continue
    }

    $metaDir = Join-Path $archiveRoot ("$name.meta")
    $dest = Join-Path $archiveRoot $name

    Write-Host "`nCandidate: $name"
    Write-Host "  Source: $path"
    Write-Host "  Dest:   $dest"

    if (Test-Path (Join-Path $path '.git')) {
        if ($Execute) {
            New-Item -ItemType Directory -Force -Path $metaDir | Out-Null
            git -C $path status --short --branch | Set-Content -Encoding UTF8 (Join-Path $metaDir 'git-status.txt')
            git -C $path log --oneline --decorate -50 | Set-Content -Encoding UTF8 (Join-Path $metaDir 'git-log.txt')
            git -C $path remote -v | Set-Content -Encoding UTF8 (Join-Path $metaDir 'git-remotes.txt')
            git -C $path diff | Set-Content -Encoding UTF8 (Join-Path $metaDir 'git-diff.patch')
        } else {
            git -C $path status --short --branch
        }
    }

    if ($Execute) {
        if (Test-Path $dest) {
            throw "Destination already exists: $dest"
        }
        Move-Item -Path $path -Destination $dest
        Write-Host "  Archived."
    }
}

Write-Host "`nDone. No permanent deletion is performed by this script."
