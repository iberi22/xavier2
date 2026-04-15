# Deploy Self-Healing Workflow to Multiple Repos
# Usage: ./scripts/deploy-self-healing.ps1 -Repos "owner/repo1,owner/repo2"

param(
    [Parameter(Mandatory=$false)]
    [string]$Repos = "",

    [Parameter(Mandatory=$false)]
    [switch]$DryRun
)

$workflowFile = ".github/workflows/self-healing.yml"
$tempBranch = "feat/self-healing-$(Get-Date -Format 'yyyyMMddHHmmss')"

# Repos por defecto (los más críticos detectados)
$defaultRepos = @(
    "iberi22/software-factory",
    "iberi22/domus-otec",
    "iberi22/less-colegio",
    "iberi22/synapse-protocol"
)

$reposToUpdate = if ($Repos) { $Repos -split ',' } else { $defaultRepos }

Write-Host "🚀 Desplegando Self-Healing Workflow a $($reposToUpdate.Count) repositorios..." -ForegroundColor Cyan

foreach ($repo in $reposToUpdate) {
    Write-Host "`n📦 Procesando: $repo" -ForegroundColor Yellow

    if ($DryRun) {
        Write-Host "   [DRY RUN] Se copiaría $workflowFile" -ForegroundColor Gray
        continue
    }

    try {
        # Verificar si el repo existe
        gh repo view $repo --json name | Out-Null
        if ($LASTEXITCODE -ne 0) {
            Write-Host "   ❌ Repo no encontrado o sin acceso" -ForegroundColor Red
            continue
        }

        # Clonar repo temporalmente
        $tempDir = "temp-$($repo.Replace('/', '-'))"
        if (Test-Path $tempDir) {
            Remove-Item -Recurse -Force $tempDir
        }

        gh repo clone $repo $tempDir 2>&1 | Out-Null
        Push-Location $tempDir

        # Crear branch
        git checkout -b $tempBranch 2>&1 | Out-Null

        # Copiar workflow
        $workflowDir = ".github/workflows"
        if (-not (Test-Path $workflowDir)) {
            New-Item -ItemType Directory -Path $workflowDir -Force | Out-Null
        }

        Copy-Item -Path "../../$workflowFile" -Destination $workflowFile -Force

        # Commit y push
        git add $workflowFile
        git commit -m "feat(ci): add self-healing workflow

Adds automated CI/CD failure detection and repair system.

- Auto-retry transient errors (timeouts, rate limits)
- Auto-fix dependency issues
- Auto-fix linting errors
- Create issues for code/test failures

Source: Git-Core-Protocol
Issue: https://github.com/iberi22/Git-Core-Protocol/issues/65"

        git push origin $tempBranch 2>&1 | Out-Null

        # Crear PR
        $prUrl = gh pr create `
            --title "🛡️ Add Self-Healing CI/CD Workflow" `
            --body "## Self-Healing CI/CD Automation

This PR adds an automated system to detect and repair workflow failures.

### Features
- ✅ Auto-retry transient errors (network timeouts, rate limits)
- ✅ Auto-fix dependency issues (npm/pip/yarn lockfiles)
- ✅ Auto-fix linting errors (ESLint, Prettier)
- ✅ Create issues for code/test failures

### How It Works
Uses GitHub Actions \`workflow_run\` event to monitor all workflows.
When a failure is detected, it classifies the error type and takes appropriate action.

### Documentation
- [Research](https://github.com/iberi22/Git-Core-Protocol/blob/main/docs/agent-docs/RESEARCH_SELFHEALING_CICD.md)
- [Workflow File](https://github.com/iberi22/Git-Core-Protocol/blob/main/.github/workflows/self-healing.yml)

**Source:** Git-Core-Protocol #65" `
            --label "ci,automation,enhancement" `
            --base main 2>&1

        if ($LASTEXITCODE -eq 0) {
            Write-Host "   ✅ PR creado: $prUrl" -ForegroundColor Green
        } else {
            Write-Host "   ⚠️ Error creando PR" -ForegroundColor Yellow
        }

        Pop-Location
        Remove-Item -Recurse -Force $tempDir

    } catch {
        Write-Host "   ❌ Error: $_" -ForegroundColor Red
        if (Test-Path $tempDir) {
            Pop-Location
            Remove-Item -Recurse -Force $tempDir
        }
    }
}

Write-Host "`n✅ Deployment completado!" -ForegroundColor Green
Write-Host "📋 Revisa los PRs creados y apruébalos para activar self-healing en cada repo." -ForegroundColor Cyan
