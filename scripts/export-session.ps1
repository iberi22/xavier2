<#
.SYNOPSIS
    Export current session context as a continuation prompt for new chat sessions.
    Enhanced version with full metrics and analysis for continuous improvement.

.DESCRIPTION
    Generates a structured .md file that captures:
    - Session summary and accomplishments
    - Modified files and git log
    - Next actions for continuation
    - Metrics for retrospective analysis

.PARAMETER Summary
    Brief summary of current work (required).

.PARAMETER Topic
    Topic identifier for filename (default: "session").

.PARAMETER Accomplishments
    Array of completed tasks this session.

.PARAMETER NextActions
    Array of next steps for the following session.

.PARAMETER Model
    AI model used in this session (claude, gemini, gpt, etc.)

.PARAMETER DurationMinutes
    Approximate session duration in minutes.

.PARAMETER Archive
    If set, saves to sessions archive instead of prompts.

.EXAMPLE
    ./scripts/export-session.ps1 -Summary "Implementing Guardian Agent" -Topic "guardian-v3"

.EXAMPLE
    ./scripts/export-session.ps1 `
        -Summary "Protocol v3.1 hybrid dispatcher" `
        -Topic "protocol-v3-1" `
        -Accomplishments @("Created dispatcher-core.ps1", "Updated risk-map.json") `
        -NextActions @("Test in production", "Update docs") `
        -Model "claude-sonnet-4" `
        -DurationMinutes 45
#>

param(
    [Parameter(Mandatory=$true, HelpMessage="Brief summary of current work")]
    [string]$Summary,

    [Parameter(HelpMessage="Topic identifier for filename")]
    [string]$Topic = "session",

    [Parameter(HelpMessage="Optional list of accomplishments")]
    [string[]]$Accomplishments = @(),

    [Parameter(HelpMessage="Optional list of next actions")]
    [string[]]$NextActions = @(),

    [Parameter(HelpMessage="AI model used (claude, gemini, gpt, grok)")]
    [string]$Model = "unknown",

    [Parameter(HelpMessage="Session duration in minutes")]
    [int]$DurationMinutes = 0,

    [Parameter(HelpMessage="Include git status")]
    [bool]$IncludeGitStatus = $true,

    [Parameter(HelpMessage="Include assigned issues")]
    [bool]$IncludeIssues = $true,

    [Parameter(HelpMessage="Include recent commits")]
    [bool]$IncludeRecentCommits = $true,

    [Parameter(HelpMessage="Number of recent commits")]
    [int]$CommitCount = 10,

    [Parameter(HelpMessage="Additional context to include")]
    [string]$AdditionalContext = "",

    [Parameter(HelpMessage="Archive session instead of creating temp prompt")]
    [switch]$Archive
)

# Ensure we're in a git repository
if (-not (Test-Path ".git")) {
    Write-Error "Not in a git repository. Run from project root."
    exit 1
}

# Determine output directory
$date = Get-Date -Format "yyyy-MM-dd"
$yearMonth = Get-Date -Format "yyyy-MM"
$timestamp = Get-Date -Format "HHmm"
$safeTopic = $Topic -replace '[^a-zA-Z0-9\-]', '-'
$filename = "SESSION_${date}_${safeTopic}.md"

if ($Archive) {
    $outputDir = "docs/agent-docs/sessions/$yearMonth"
} else {
    $outputDir = "docs/prompts"
}

if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    Write-Host "📁 Created $outputDir directory" -ForegroundColor Cyan
}

$filepath = Join-Path $outputDir $filename

# Gather context
$repoName = Split-Path -Leaf (Get-Location)
$branch = git branch --show-current 2>$null
if (-not $branch) { $branch = "unknown" }

# Git status and modified files
$modifiedFiles = @()
$filesCount = 0
$stagedCount = 0
$gitStatusSection = ""

if ($IncludeGitStatus) {
    $statusOutput = git status --porcelain 2>$null
    if ($statusOutput) {
        $modifiedFiles = $statusOutput | ForEach-Object { $_.Substring(3) }
        $filesCount = $modifiedFiles.Count
    }
    $stagedCount = (git diff --cached --name-only 2>$null | Measure-Object).Count

    $modifiedList = if ($modifiedFiles.Count -gt 0) {
        ($modifiedFiles | ForEach-Object { "  - ``$_``" }) -join "`n"
    } else { "  - (ninguno)" }

    $gitStatusSection = @"

## 📊 Estado Git

| Métrica | Valor |
|---------|-------|
| Branch | ``$branch`` |
| Archivos modificados | $filesCount |
| Archivos staged | $stagedCount |

### Archivos Modificados
$modifiedList

"@
}

# Recent commits with full info
$commitsSection = ""
$commitsCount = 0
if ($IncludeRecentCommits) {
    $commitsRaw = git log --oneline -n $CommitCount 2>$null
    $commitsCount = ($commitsRaw | Measure-Object).Count
    if ($commitsRaw) {
        $commitsList = ($commitsRaw | ForEach-Object { "- ``$_``" }) -join "`n"
        $commitsSection = @"

## 📝 Commits Recientes ($commitsCount)

$commitsList

"@
    }
}

# Assigned issues
$issuesSection = ""
$issuesCount = 0
$issuesTouched = @()
if ($IncludeIssues) {
    try {
        $issues = gh issue list --assignee "@me" --state open --limit 10 --json number,title 2>$null | ConvertFrom-Json
        if ($issues -and $issues.Count -gt 0) {
            $issuesCount = $issues.Count
            $issuesTouched = $issues | ForEach-Object { "#$($_.number)" }
            $issueList = ($issues | ForEach-Object { "- **#$($_.number)**: $($_.title)" }) -join "`n"
            $issuesSection = @"

## 📋 Issues Asignados ($issuesCount)

$issueList

"@
        }
    } catch {
        # gh cli not available
    }
}

# Accomplishments section
$accomplishmentsSection = ""
if ($Accomplishments.Count -gt 0) {
    $accList = ($Accomplishments | ForEach-Object { "- ✅ $_" }) -join "`n"
    $accomplishmentsSection = @"

## ✅ Logros de la Sesión

$accList

"@
}

# Next actions section
$nextActionsSection = ""
if ($NextActions.Count -gt 0) {
    $nextList = ($NextActions | ForEach-Object { "- [ ] $_" }) -join "`n"
    $nextActionsSection = @"

## 🚀 Próximas Acciones

$nextList

"@
}

# Additional context
$additionalSection = ""
if ($AdditionalContext) {
    $additionalSection = @"

## 📎 Contexto Adicional

$AdditionalContext

"@
}

# Generate session ID
$sessionId = [guid]::NewGuid().ToString().Substring(0, 8)

# Build YAML frontmatter with full metrics
$issuesTouchedYaml = if ($issuesTouched.Count -gt 0) {
    "`n  - " + ($issuesTouched -join "`n  - ")
} else { " []" }

$accomplishmentsYaml = if ($Accomplishments.Count -gt 0) {
    "`n  - " + ($Accomplishments -join "`n  - ")
} else { " []" }

$nextActionsYaml = if ($NextActions.Count -gt 0) {
    "`n  - " + ($NextActions -join "`n  - ")
} else { " []" }

# Generate the prompt content
$content = @"
---
title: "Session - $Topic"
type: SESSION
created: $date
generated_at: "${date}T${timestamp}"
generator: export-session.ps1
version: "2.0"
project: $repoName
branch: $branch
model: $Model
session_id: "$sessionId"
duration_minutes: $DurationMinutes
files_modified: $filesCount
commits_made: $commitsCount
issues_touched:$issuesTouchedYaml
accomplishments:$accomplishmentsYaml
next_actions:$nextActionsYaml
status: $(if ($Archive) { "archived" } else { "active" })
---

# 🔄 Session: $Topic

> **Generado por:** ``export-session.ps1 v2.0``
> **ID:** ``$sessionId`` | **Modelo:** ``$Model``
$(if (-not $Archive) { "> ⚠️ **Eliminar después de usar** - Este archivo es contexto temporal." })

---

## 🎯 Resumen

$Summary
$accomplishmentsSection$gitStatusSection$commitsSection$issuesSection$nextActionsSection$additionalSection
---

## 📈 Métricas de Sesión

| Métrica | Valor |
|---------|-------|
| Duración | ~$DurationMinutes min |
| Archivos tocados | $filesCount |
| Commits | $commitsCount |
| Issues relacionados | $issuesCount |
| Modelo IA | $Model |

---

## 📋 Para Continuar

**En nueva ventana de chat:**

1. Copia este texto: ``#file:$filepath``
2. Pégalo y presiona Enter
3. El agente tendrá todo el contexto

**Foco principal:** $Summary

---

*Session ID: $sessionId | Generated: $date $timestamp*
"@

# Write the file
$content | Out-File -FilePath $filepath -Encoding utf8

# Copy file reference to clipboard
$clipboardText = "#file:$filepath"
$clipboardText | Set-Clipboard

# Display summary
Write-Host ""
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor DarkCyan
Write-Host "✅ SESSION EXPORTED SUCCESSFULLY" -ForegroundColor Green
Write-Host "═══════════════════════════════════════════════════════════" -ForegroundColor DarkCyan
Write-Host ""
Write-Host "📄 Archivo: " -NoNewline -ForegroundColor Cyan
Write-Host $filepath -ForegroundColor White
Write-Host ""
Write-Host "📊 MÉTRICAS:" -ForegroundColor Yellow
Write-Host "   • Archivos modificados: $filesCount"
Write-Host "   • Commits recientes:    $commitsCount"
Write-Host "   • Issues relacionados:  $issuesCount"
Write-Host "   • Modelo:               $Model"
Write-Host ""
Write-Host "📋 COPIADO AL PORTAPAPELES:" -ForegroundColor Green
Write-Host "   $clipboardText" -ForegroundColor White
Write-Host ""
Write-Host "🔄 Para continuar:" -ForegroundColor Yellow
Write-Host "   1. Abre nueva ventana de chat"
Write-Host "   2. Pega (Ctrl+V)"
Write-Host "   3. ¡Continúa tu trabajo!"
Write-Host ""

if ($Archive) {
    Write-Host "📁 Archivado en: docs/agent-docs/sessions/$yearMonth/" -ForegroundColor Magenta
} else {
    Write-Host "🗑️  Recuerda eliminar después de usar (o usa -Archive para guardar)" -ForegroundColor DarkGray
}

Write-Host ""
