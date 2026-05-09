---
title: "Manejo de Pull Requests en Draft"
type: SPEC
id: "spec-draft-pr-merge"
created: 2025-12-07
updated: 2025-12-07
agent: copilot
model: claude-sonnet-4
requested_by: user
summary: |
  Procedimiento para convertir PRs en draft a ready y hacer merge automáticamente.
keywords: [github, pull-request, draft, automation, workflow]
tags: ["#github", "#automation", "#pr-workflow"]
project: Git-Core-Protocol
priority: medium
status: approved
---

# 🔄 SPEC: Manejo de Pull Requests en Draft

## 🎯 Problema

Cuando agentes como Jules crean PRs, a veces los marcan como **Draft** por defecto. Los PRs en draft no se pueden hacer merge directamente, bloqueando la automatización.

**Error típico:**

```
gh pr merge 69 --squash
GraphQL: Pull Request is still a draft (mergePullRequest)
```

---

## ✅ Solución: Convertir Draft → Ready via CLI

### Método 1: GraphQL API (Recomendado para Automatización)

```powershell
# Paso 1: Obtener el ID del PR
$prId = (gh pr view <NUMBER> --json id | ConvertFrom-Json).id

# Paso 2: Marcar como ready usando GraphQL
$query = "mutation { markPullRequestReadyForReview(input: {pullRequestId: `"$prId`"}) { pullRequest { id isDraft } } }"
gh api graphql -f query=$query

# Paso 3: Hacer merge normalmente
gh pr merge <NUMBER> --squash --delete-branch
```

**Bash:**

```bash
# Obtener ID
PR_ID=$(gh pr view <NUMBER> --json id -q '.id')

# Marcar como ready
gh api graphql -f query="mutation { markPullRequestReadyForReview(input: {pullRequestId: \"$PR_ID\"}) { pullRequest { id isDraft } } }"

# Merge
gh pr merge <NUMBER> --squash --delete-branch
```

---

## 🛠️ Script Automatizado

### PowerShell: `scripts/merge-draft-pr.ps1`

```powershell
<#
.SYNOPSIS
    Merge un PR incluso si está en draft
.PARAMETER PrNumber
    Número del PR a mergear
.PARAMETER DeleteBranch
    Eliminar rama remota después del merge
#>
param(
    [Parameter(Mandatory=$true)]
    [int]$PrNumber,

    [switch]$DeleteBranch = $true
)

Write-Host "🔍 Verificando estado del PR #$PrNumber..." -ForegroundColor Cyan

# Obtener info del PR
$prInfo = gh pr view $PrNumber --json isDraft,state,id | ConvertFrom-Json

if ($prInfo.state -ne "OPEN") {
    Write-Host "❌ PR #$PrNumber no está abierto (estado: $($prInfo.state))" -ForegroundColor Red
    exit 1
}

# Convertir draft a ready si es necesario
if ($prInfo.isDraft) {
    Write-Host "📝 PR está en draft. Convirtiendo a ready..." -ForegroundColor Yellow

    $query = "mutation { markPullRequestReadyForReview(input: {pullRequestId: `"$($prInfo.id)`"}) { pullRequest { id isDraft } } }"
    $result = gh api graphql -f query=$query | ConvertFrom-Json

    if ($result.data.markPullRequestReadyForReview.pullRequest.isDraft -eq $false) {
        Write-Host "✅ PR marcado como ready" -ForegroundColor Green
    } else {
        Write-Host "❌ Error al convertir PR a ready" -ForegroundColor Red
        exit 1
    }
}

# Hacer merge
Write-Host "🔀 Haciendo squash merge del PR #$PrNumber..." -ForegroundColor Cyan

if ($DeleteBranch) {
    gh pr merge $PrNumber --squash --delete-branch
} else {
    gh pr merge $PrNumber --squash
}

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ PR #$PrNumber mergeado exitosamente" -ForegroundColor Green
} else {
    Write-Host "❌ Error al mergear PR #$PrNumber" -ForegroundColor Red
    exit 1
}
```

### Bash: `scripts/merge-draft-pr.sh`

```bash
#!/bin/bash
set -e

PR_NUMBER=$1

if [ -z "$PR_NUMBER" ]; then
    echo "❌ Uso: ./merge-draft-pr.sh <PR_NUMBER>"
    exit 1
fi

echo "🔍 Verificando estado del PR #$PR_NUMBER..."

# Obtener info del PR
PR_INFO=$(gh pr view "$PR_NUMBER" --json isDraft,state,id)
IS_DRAFT=$(echo "$PR_INFO" | jq -r '.isDraft')
STATE=$(echo "$PR_INFO" | jq -r '.state')
PR_ID=$(echo "$PR_INFO" | jq -r '.id')

if [ "$STATE" != "OPEN" ]; then
    echo "❌ PR #$PR_NUMBER no está abierto (estado: $STATE)"
    exit 1
fi

# Convertir draft a ready si es necesario
if [ "$IS_DRAFT" = "true" ]; then
    echo "📝 PR está en draft. Convirtiendo a ready..."

    gh api graphql -f query="mutation { markPullRequestReadyForReview(input: {pullRequestId: \"$PR_ID\"}) { pullRequest { id isDraft } } }"

    echo "✅ PR marcado como ready"
fi

# Hacer merge
echo "🔀 Haciendo squash merge del PR #$PR_NUMBER..."
gh pr merge "$PR_NUMBER" --squash --delete-branch

echo "✅ PR #$PR_NUMBER mergeado exitosamente"
```

---

## 📋 Workflow para Agentes

### Cuando un agente (Jules, Copilot) crea un PR

```powershell
# 1. Detectar nuevo PR
$newPR = gh pr list --author "app/google-labs-jules" --limit 1 --json number | ConvertFrom-Json

# 2. Revisar el PR
gh pr view $newPR.number
gh pr diff $newPR.number > pr-review.txt

# 3. Si apruebas, usar el script
./scripts/merge-draft-pr.ps1 -PrNumber $newPR.number
```

---

## 🔧 Integración con Workflows

### `.github/workflows/auto-merge-approved-prs.yml`

```yaml
name: Auto-Merge Approved PRs

on:
  pull_request_review:
    types: [submitted]

jobs:
  auto-merge:
    if: github.event.review.state == 'approved'
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write

    steps:
      - name: Convert draft to ready if needed
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          PR_NUMBER=${{ github.event.pull_request.number }}
          IS_DRAFT=$(gh pr view $PR_NUMBER --json isDraft -q '.isDraft')

          if [ "$IS_DRAFT" = "true" ]; then
            echo "Converting draft PR to ready..."
            PR_ID=$(gh pr view $PR_NUMBER --json id -q '.id')
            gh api graphql -f query="mutation { markPullRequestReadyForReview(input: {pullRequestId: \"$PR_ID\"}) { pullRequest { id } } }"
          fi

      - name: Merge PR
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh pr merge ${{ github.event.pull_request.number }} --squash --delete-branch
```

---

## 📊 Casos de Uso

| Situación | Comando |
|-----------|---------|
| **PR normal (no draft)** | `gh pr merge <N> --squash` |
| **PR en draft** | `./scripts/merge-draft-pr.ps1 -PrNumber <N>` |
| **PR de Jules/Copilot** | Siempre usar script (pueden crear drafts) |
| **Workflow automático** | Usar workflow de auto-merge |

---

## 🚨 Errores Comunes

### Error 1: "Pull Request is still a draft"

**Solución:** Usar el script `merge-draft-pr.ps1` en lugar de `gh pr merge` directo.

### Error 2: Escapado de comillas en PowerShell

**Problema:**

```powershell
gh api graphql -f query='mutation { ... "PR_kwDO..." ... }'  # ❌ Falla
```

**Solución:**

```powershell
$query = 'mutation { ... "PR_kwDO..." ... }'
gh api graphql -f query=$query  # ✅ Funciona
```

### Error 3: PR ya mergeado

**Detección:**

```powershell
$state = (gh pr view <N> --json state | ConvertFrom-Json).state
if ($state -eq "MERGED") {
    Write-Host "PR ya fue mergeado"
}
```

---

## 📚 Referencias

- [GitHub GraphQL API: markPullRequestReadyForReview](https://docs.github.com/en/graphql/reference/mutations#markpullrequestreadyforreview)
- [gh CLI: pr commands](https://cli.github.com/manual/gh_pr)
- `.github/copilot-instructions.md` - Workflow de revisión de PRs

---

*Especificación aprobada: 2025-12-07*
*Probado con: PR #69 (Jules agent)*
