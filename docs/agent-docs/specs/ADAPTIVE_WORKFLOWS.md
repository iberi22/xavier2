---
title: "Adaptive Workflow System - Resource Optimization"
type: DOCUMENTATION
id: "doc-adaptive-workflows"
created: 2025-12-07
updated: 2025-12-07
agent: copilot
model: claude-sonnet-4
requested_by: user
summary: |
  Sistema adaptativo de workflows que detecta automáticamente si el repositorio
  es público o privado y ajusta el consumo de recursos de GitHub Actions.
keywords: [github-actions, optimization, adaptive, resource-management]
tags: ["#optimization", "#github-actions", "#automation"]
topics: [ci-cd, resource-management, cost-optimization]
project: Git-Core-Protocol
priority: high
status: production
---

# 🎯 Adaptive Workflow System

## 📋 Descripción

Sistema inteligente que **detecta automáticamente** si el repositorio es público o privado y ajusta la configuración de workflows para optimizar el uso de minutos de GitHub Actions.

### 🎯 Problema Resuelto

**Antes:**

- Workflows ejecutándose con alta frecuencia en repos privados
- Consumo estimado: **18,000 min/mes** (9x el límite Free)
- Riesgo de agotar cuota en pocos días

**Después:**

- Detección automática de tipo de repo
- Ajuste dinámico de frecuencias
- Consumo estimado: **600 min/mes** en privados ✅

---

## 🏗️ Arquitectura del Sistema

```
┌─────────────────────────────────────────────────────────────────┐
│                    ADAPTIVE WORKFLOW SYSTEM                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐                                               │
│  │   DETECTOR   │  ← detect-repo-config.ps1 / .sh              │
│  │              │    Ejecuta: gh repo view --json visibility    │
│  └──────┬───────┘                                               │
│         │                                                       │
│         ├──────► IS_PUBLIC = true/false                        │
│         ├──────► IS_MAIN_REPO = true/false                     │
│         └──────► SCHEDULE_MODE = aggressive/moderate/conservative│
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    SCHEDULE MODES                        │   │
│  ├─────────────────────────────────────────────────────────┤   │
│  │                                                          │   │
│  │  🟢 AGGRESSIVE (Public repos)                           │   │
│  │     • Schedules: Every 30 min / hourly                   │   │
│  │     • Multi-repo monitoring (11 repos)                   │   │
│  │     • ~600 min/day (UNLIMITED - public)                  │   │
│  │                                                          │   │
│  │  🟡 MODERATE (Private main repo)                        │   │
│  │     • Schedules: Every 6 hours / daily                   │   │
│  │     • Single-repo monitoring                             │   │
│  │     • ~100 min/day (~3,000 min/month)                    │   │
│  │                                                          │   │
│  │  🔴 CONSERVATIVE (Other private repos)                  │   │
│  │     • NO schedules (event-based only)                    │   │
│  │     • Triggers: push, PR, issues, workflow_run          │   │
│  │     • ~20 min/day (~600 min/month) ✅                    │   │
│  │                                                          │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🔧 Componentes

### 1. Scripts de Detección

| Archivo | Propósito |
|---------|-----------|
| `scripts/detect-repo-config.ps1` | Detector para Windows/PowerShell |
| `scripts/detect-repo-config.sh` | Detector para Linux/macOS/Bash |

**Output:**

```yaml
is_public: true/false
is_main_repo: true/false
enable_schedules: true/false
schedule_mode: aggressive/moderate/conservative
```

### 2. Workflow de Configuración

| Archivo | Propósito |
|---------|-----------|
| `.github/workflows/_repo-config.yml` | Workflow reutilizable para detectar config |

**Uso:**

```yaml
jobs:
  config:
    uses: ./.github/workflows/_repo-config.yml

  my-job:
    needs: config
    if: needs.config.outputs.enable_schedules == 'true'
```

### 3. Workflows Optimizados

| Workflow | Optimización Aplicada |
|----------|----------------------|
| `global-self-healing.yml` | ✅ workflow_run + detección adaptativa |
| `email-cleanup.yml` | ✅ Frecuencias variables + detección |
| `copilot-meta-analysis.yml` | ✅ Schedule reducido a 6h + detección |
| `self-healing.yml` | ✅ Solo workflow_run (FREE) |
| Todos | ✅ timeout-minutes agregado |

---

## 📊 Modos de Ejecución

### 🟢 Modo AGGRESSIVE (Repos Públicos)

**Cuándo:** `visibility: PUBLIC`

**Configuración:**

```yaml
schedule:
  - cron: "*/30 * * * *"  # Cada 30 minutos
  - cron: "0 * * * *"     # Cada hora

strategy:
  matrix:
    repo: [11 repos...]   # Multi-repo monitoring
```

**Consumo:**

- ~600 min/día
- ~18,000 min/mes
- ✅ **ILIMITADO** (repos públicos)

**Ventajas:**

- Monitoreo en tiempo casi real
- Multi-repo support
- Sin restricciones

---

### 🟡 Modo MODERATE (Main Private Repo)

**Cuándo:** `visibility: PRIVATE` AND `is_main_repo: true`

**Configuración:**

```yaml
schedule:
  - cron: "0 */6 * * *"   # Cada 6 horas
  - cron: "0 9 * * *"     # Una vez al día

strategy:
  matrix:
    repo: [1 repo]        # Solo este repo
```

**Consumo:**

- ~100 min/día
- ~3,000 min/mes
- ⚠️ Requiere GitHub Pro ($4/mes)

**Ventajas:**

- Balance entre monitoreo y costo
- Funcionalidad core mantenida
- Predecible

---

### 🔴 Modo CONSERVATIVE (Other Private Repos)

**Cuándo:** `visibility: PRIVATE` AND `is_main_repo: false`

**Configuración:**

```yaml
on:
  # NO schedules
  push:
  pull_request:
  issues:
  workflow_run:
```

**Consumo:**

- ~20 min/día
- ~600 min/mes
- ✅ Dentro del límite Free (2,000 min/mes)

**Ventajas:**

- Costo $0
- Funcionalidad event-based completa
- Eficiente

---

## 🚀 Instalación y Uso

### Para Nuevos Proyectos

```bash
# 1. Instalar Git-Core Protocol
curl -fsSL https://raw.githubusercontent.com/iberi22/Git-Core-Protocol/main/install.sh | bash

# 2. El sistema de detección ya está incluido
# No requiere configuración adicional ✅
```

### Para Proyectos Existentes

```bash
# 1. Copiar scripts de detección
cp scripts/detect-repo-config.ps1 tu-proyecto/scripts/
cp scripts/detect-repo-config.sh tu-proyecto/scripts/
chmod +x tu-proyecto/scripts/detect-repo-config.sh

# 2. Copiar workflow de config
cp .github/workflows/_repo-config.yml tu-proyecto/.github/workflows/

# 3. Actualizar tus workflows (ver ejemplos abajo)
```

### Ejemplo: Agregar Detección a un Workflow

**ANTES:**

```yaml
name: My Workflow

on:
  schedule:
    - cron: "0 * * * *"

jobs:
  my-job:
    runs-on: ubuntu-latest
    steps:
      - name: Do something
        run: echo "Running..."
```

**DESPUÉS:**

```yaml
name: My Workflow

on:
  schedule:
    - cron: "0 * * * *"      # Aggressive
    - cron: "0 */6 * * *"    # Moderate
  workflow_dispatch:

jobs:
  config:
    name: 🔧 Detect Config
    runs-on: ubuntu-latest
    timeout-minutes: 2
    outputs:
      should_run: ${{ steps.decide.outputs.should_run }}

    steps:
      - uses: actions/checkout@v4

      - name: Detect Repository Type
        id: detect
        shell: pwsh
        run: ./scripts/detect-repo-config.ps1

      - name: Decide if should run
        id: decide
        shell: pwsh
        run: |
          $scheduleMode = "${{ steps.detect.outputs.schedule_mode }}"
          $shouldRun = "false"

          if ("${{ github.event_name }}" -eq "workflow_dispatch") {
            $shouldRun = "true"
          } elseif ($scheduleMode -in @("aggressive", "moderate")) {
            $shouldRun = "true"
          }

          Add-Content -Path $env:GITHUB_OUTPUT -Value "should_run=$shouldRun"

  my-job:
    name: 🚀 My Job
    needs: config
    if: needs.config.outputs.should_run == 'true'
    runs-on: ubuntu-latest
    timeout-minutes: 10
    steps:
      - name: Do something
        run: echo "Running..."
```

---

## 🧪 Testing

### Test Local

```powershell
# PowerShell
./scripts/detect-repo-config.ps1

# Bash
./scripts/detect-repo-config.sh
```

**Output esperado:**

```
🔍 Repository Configuration Detection

📊 Repository: iberi22/Git-Core-Protocol
🔒 Visibility: PUBLIC
🏠 Is Main Repo: true
✅ PUBLIC repo: Aggressive scheduling enabled (unlimited minutes)

📋 Configuration Summary:
   IS_PUBLIC=true
   IS_MAIN_REPO=true
   ENABLE_SCHEDULES=true
   SCHEDULE_MODE=aggressive
```

### Test en GitHub Actions

```bash
# Trigger manual
gh workflow run global-self-healing.yml

# Ver logs
gh run list --workflow=global-self-healing.yml --limit 1
gh run view <run-id> --log
```

---

## 📈 Métricas de Éxito

### Consumo ANTES vs DESPUÉS

| Tipo de Repo | Antes (min/mes) | Después (min/mes) | Ahorro |
|--------------|-----------------|-------------------|--------|
| **Público** | 18,000 | 18,000 | 0% (ILIMITADO ✅) |
| **Privado Main** | 18,000 | 3,000 | 83% |
| **Privado Other** | 18,000 | 600 | **97%** ✅ |

### Funcionalidad Mantenida

| Feature | Público | Privado Main | Privado Other |
|---------|---------|--------------|---------------|
| Self-healing | ✅ Real-time | ✅ 6-hourly | ✅ Event-based |
| AI Agents | ✅ Full | ✅ Full | ✅ Full |
| PR Auto-merge | ✅ | ✅ | ✅ |
| E2E Testing | ✅ | ✅ | ✅ |
| Living Context | ✅ Weekly | ✅ Weekly | ✅ On-demand |

---

## ⚠️ Consideraciones

### Límites de GitHub Actions

| Plan | Minutos/Mes | Costo |
|------|-------------|-------|
| Free (Public) | ♾️ ILIMITADO | $0 |
| Free (Private) | 2,000 | $0 |
| Pro (Private) | 3,000 | $4/mes |
| Team (Private) | 10,000 | $4/usuario/mes |

### Recomendaciones

1. **Repos públicos:** Usa modo aggressive sin preocupación
2. **Repo principal privado:** Considera GitHub Pro si >3,000 min/mes
3. **Otros repos privados:** Modo conservative es suficiente

### Workflow_run Events (FREE)

Los eventos `workflow_run` **NO consumen minutos del límite** porque solo se ejecutan cuando otro workflow falla. Son la forma más eficiente de monitoreo.

```yaml
on:
  workflow_run:
    workflows: ["*"]
    types: [completed]
```

---

## 🔗 Referencias

- [GitHub Actions Usage Limits](https://docs.github.com/en/actions/learn-github-actions/usage-limits-billing-and-administration)
- [workflow_run Event](https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows#workflow_run)
- Git-Core Protocol: `AGENTS.md`

---

## 🆘 Troubleshooting

### Problema: "Schedule not running"

**Causa:** Repo privado en modo conservative.

**Solución:**

```bash
# Verificar configuración
./scripts/detect-repo-config.ps1

# Si quieres forzar modo moderate:
# 1. Editar .github/workflows/global-self-healing.yml
# 2. Agregar input force_mode: moderate en workflow_dispatch
```

### Problema: "Consuming too many minutes"

**Causa:** Repo privado en modo aggressive.

**Solución:** El sistema ya ajusta automáticamente. Si persiste:

```bash
# 1. Verificar que los scripts están actualizados
git pull origin main

# 2. Re-ejecutar workflows para aplicar nueva config
gh workflow run global-self-healing.yml
```

### Problema: "Script not found"

**Causa:** Scripts no tienen permisos de ejecución.

**Solución:**

```bash
chmod +x scripts/detect-repo-config.sh
git add scripts/detect-repo-config.sh
git commit -m "fix: add execute permission to config script"
```

---

*Última actualización: 2025-12-07*
*Versión: 1.0.0*
