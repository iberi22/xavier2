# 📦 Documentation Migration Guide

> **"¿Qué hago con mis archivos viejos al adoptar Diátaxis?"**

## 🎯 El Problema

Tienes un proyecto existente con documentación que quieres migrar a Git-Core Protocol v3.2 con Diátaxis. ¿Qué hacer con:

- `TODO.md`, `TASKS.md`, `PLANNING.md` (tracking files)
- `GUIDE.md`, `HOWTO.md`, `TUTORIAL.md` (guías sin clasificar)
- `NOTES.md`, `SCRATCH.md` (notas temporales)
- `IMPLEMENTATION.md`, `SUMMARY.md` (documentos de progreso)

---

## ✅ Estrategia de Migración (Mejor Enfoque)

### Fase 1: Clasificar (5 minutos)

Revisa cada archivo existente y clasifícalo:

| Archivo Antiguo | ¿Qué contiene? | Destino en Diátaxis |
|----------------|----------------|---------------------|
| `TODO.md` | Lista de tareas | → **GitHub Issues** |
| `PLANNING.md` | Planificación | → **GitHub Issues** + `ARCHITECTURE.md` |
| `NOTES.md` | Notas temporales | → **Issue Comments** o eliminar |
| `GUIDE.md` | Tutorial paso a paso | → `tutorials/` o `how-to/` |
| `REFERENCE.md` | Sintaxis, API | → `reference/` |
| `ABOUT_X.md` | Explicación de diseño | → `explanation/` |
| `SETUP.md` | Instalación | → `setup/` o `how-to/` |

### Fase 2: Migrar Contenido Valioso (10-20 minutos)

#### Opción A: Migración Automática (Recomendada)

```powershell
# Ejecutar script de migración
./scripts/migrate-docs-to-diataxis.ps1 -ProjectPath "."

# El script:
# 1. Analiza cada .md en la raíz
# 2. Detecta su tipo (tutorial, how-to, reference, explanation)
# 3. Lo mueve a la carpeta correcta
# 4. Crea issues para TODO/PLANNING/TASKS
```

#### Opción B: Migración Manual

**Para archivos de tracking:**

```bash
# 1. Extraer tareas de TODO.md y crear issues
cat TODO.md | grep "^- \[ \]" | while read task; do
  gh issue create --title "$task" --label "migrated"
done

# 2. Eliminar el archivo
rm TODO.md
```

**Para guías y tutoriales:**

```bash
# Determinar tipo y mover
# ¿Es una lección? → tutorials/
mv BEGINNER_GUIDE.md docs/tutorials/GETTING_STARTED.md

# ¿Resuelve un problema? → how-to/
mv HOW_TO_DEPLOY.md docs/how-to/DEPLOYMENT.md

# ¿Es referencia técnica? → reference/
mv API_REFERENCE.md docs/reference/API.md

# ¿Explica el "por qué"? → explanation/
mv ARCHITECTURE_DECISIONS.md docs/explanation/DESIGN_RATIONALE.md
```

### Fase 3: Actualizar Links (5 minutos)

```bash
# Buscar links rotos
grep -r "](./OLD_FILE.md)" . --include="*.md"

# Actualizar con nuevas rutas
# Ejemplo: [link](./GUIDE.md) → [link](./docs/tutorials/GETTING_STARTED.md)
```

---

## 🗂️ Tabla de Decisión Rápida

### "¿A dónde va este archivo?"

| Si el contenido... | Entonces va a... | Ejemplo |
|-------------------|------------------|---------|
| **Te enseña paso a paso** | `tutorials/` | "Tutorial: Tu primer workflow" |
| **Resuelve un problema específico** | `how-to/` | "Cómo exportar una sesión" |
| **Lista comandos, sintaxis, API** | `reference/` | "Referencia de git-core CLI" |
| **Explica diseño o filosofía** | `explanation/` | "Por qué GitHub Issues y no archivos" |
| **Es una lista de tareas** | **GitHub Issues** | Crear issues individuales |
| **Son notas temporales** | **Eliminar** o → issue comments | N/A |
| **Es configuración inicial** | `setup/` | "Guía de instalación" |
| **Es para agentes AI** | `agent-docs/` | "Especificación de protocolo" |

---

## 📋 Casos Específicos

### Caso 1: `TODO.md` con 50 tareas

**Problema:** Archivo enorme con tareas mezcladas.

**Solución:**

```bash
# Opción A: Script automático
./scripts/migrate-tasks-to-issues.ps1 -File "TODO.md"

# Opción B: Manual con bulk creation
# 1. Clasificar tareas por tipo (bug, feature, chore)
# 2. Crear issues en lote:
gh issue create --title "Task 1" --label "migrated,enhancement"
gh issue create --title "Task 2" --label "migrated,bug"
# ...

# 3. Archivar el archivo
mkdir -p docs/archive/2025-12
mv TODO.md docs/archive/2025-12/TODO_MIGRATED.md
```

### Caso 2: `IMPLEMENTATION_GUIDE.md` - ¿Tutorial o How-To?

**Test de clasificación:**

| Pregunta | Respuesta | Tipo |
|----------|-----------|------|
| ¿Es para aprender desde cero? | Sí | Tutorial |
| ¿Resuelve un problema específico? | Sí | How-To |
| ¿Es paso a paso? | Ambos pueden serlo | Depende |
| ¿Espera que ya sepas los básicos? | No → Tutorial / Sí → How-To | - |

**Ejemplo:**

```markdown
# Si dice: "Aprenderás a implementar autenticación paso a paso"
→ tutorials/AUTHENTICATION_IMPLEMENTATION.md

# Si dice: "Cómo implementar OAuth en tu proyecto"
→ how-to/IMPLEMENT_OAUTH.md
```

### Caso 3: `NOTES.md` con ideas y decisiones

**Clasificar contenido:**

| Contenido en NOTES.md | Destino |
|----------------------|---------|
| Decisiones de arquitectura | → `explanation/DESIGN_DECISIONS.md` |
| Ideas para features | → GitHub Issues con label `idea` |
| TODOs | → GitHub Issues |
| Notas de reunión | → Issue comments en issue relevante |
| Borradores | → Eliminar o completar primero |

### Caso 4: Múltiples archivos `GUIDE_*.md`

**Estrategia:**

```bash
# 1. Analizar cada uno
for file in GUIDE_*.md; do
  echo "Analyzing $file..."
  # Leer primeras líneas para determinar tipo
done

# 2. Renombrar con prefijo semántico
GUIDE_GETTING_STARTED.md → tutorials/GETTING_STARTED.md
GUIDE_DEPLOYMENT.md      → how-to/DEPLOYMENT.md
GUIDE_ARCHITECTURE.md    → explanation/ARCHITECTURE_OVERVIEW.md
```

---

## 🔄 Script de Migración Automatizado

Crear `scripts/migrate-docs-to-diataxis.ps1`:

```powershell
<#
.SYNOPSIS
Migra documentación existente a estructura Diátaxis

.PARAMETER ProjectPath
Ruta del proyecto a migrar

.PARAMETER DryRun
Solo muestra qué haría sin ejecutar

.EXAMPLE
./scripts/migrate-docs-to-diataxis.ps1 -ProjectPath "." -DryRun
#>
param(
    [string]$ProjectPath = ".",
    [switch]$DryRun
)

# Palabras clave para clasificación
$tutorialKeywords = @("tutorial", "learn", "beginner", "guide", "lesson")
$howToKeywords = @("how to", "howto", "recipe", "solve")
$referenceKeywords = @("reference", "api", "command", "syntax")
$explanationKeywords = @("about", "why", "philosophy", "design", "architecture")
$trackingKeywords = @("todo", "tasks", "planning", "backlog", "progress")

# Archivos a migrar
$files = Get-ChildItem -Path $ProjectPath -Filter "*.md" -File

foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw
    $name = $file.Name.ToLower()

    # Detectar tipo
    $type = if ($trackingKeywords | Where-Object { $name -match $_ }) {
        "TRACKING"
    } elseif ($tutorialKeywords | Where-Object { $name -match $_ -or $content -match $_ }) {
        "TUTORIAL"
    } elseif ($howToKeywords | Where-Object { $name -match $_ -or $content -match $_ }) {
        "HOWTO"
    } elseif ($referenceKeywords | Where-Object { $name -match $_ -or $content -match $_ }) {
        "REFERENCE"
    } elseif ($explanationKeywords | Where-Object { $name -match $_ -or $content -match $_ }) {
        "EXPLANATION"
    } else {
        "UNKNOWN"
    }

    # Determinar destino
    $destination = switch ($type) {
        "TUTORIAL"     { "docs/tutorials/$($file.Name)" }
        "HOWTO"        { "docs/how-to/$($file.Name)" }
        "REFERENCE"    { "docs/reference/$($file.Name)" }
        "EXPLANATION"  { "docs/explanation/$($file.Name)" }
        "TRACKING"     { "→ GitHub Issues (manual)" }
        "UNKNOWN"      { "docs/archive/2025-12/$($file.Name)" }
    }

    Write-Host "$($file.Name) → $type → $destination"

    if (-not $DryRun -and $type -ne "TRACKING" -and $type -ne "UNKNOWN") {
        Move-Item $file.FullName $destination -Force
    }
}
```

---

## 🚫 Qué NO Migrar

**Eliminar directamente (no migrar):**

| Archivo | Razón |
|---------|-------|
| `TODO.md` | Crear issues, eliminar archivo |
| `SCRATCH.md` | Notas temporales sin valor |
| `TEMP_*.md` | Archivos temporales |
| `OLD_*.md` | Versiones obsoletas |
| `BACKUP_*.md` | Respaldos innecesarios |

**Archivar (si tiene valor histórico):**

```bash
mkdir -p docs/archive/2025-12
mv OLD_FILE.md docs/archive/2025-12/
```

---

## 📊 Checklist de Migración

### Antes de empezar

- [ ] Backup del proyecto (commit actual)
- [ ] Listar todos los `.md` en la raíz
- [ ] Identificar archivos de tracking (TODO, TASKS, etc.)
- [ ] Identificar guías sin clasificar

### Durante la migración

- [ ] Convertir TODOs a GitHub Issues
- [ ] Mover tutoriales a `docs/tutorials/`
- [ ] Mover how-tos a `docs/how-to/`
- [ ] Mover referencias a `docs/reference/`
- [ ] Mover explicaciones a `docs/explanation/`
- [ ] Actualizar links internos
- [ ] Archivar documentos obsoletos

### Después de migrar

- [ ] Verificar que no hay links rotos
- [ ] Actualizar `docs/README.md` si agregaste contenido
- [ ] Commit con mensaje descriptivo
- [ ] Actualizar wiki si es necesario

---

## 🎯 Ejemplo Completo: Proyecto "MyApp"

**Estado inicial:**

```
my-app/
├── TODO.md               # 30 tareas
├── SETUP_GUIDE.md        # Instalación
├── USAGE.md              # Cómo usar
├── API_REFERENCE.md      # Referencia de API
├── ARCHITECTURE.md       # Explicación de diseño
├── NOTES.md              # Notas mezcladas
└── src/
```

**Después de migración:**

```
my-app/
├── docs/
│   ├── README.md         # Índice Diátaxis
│   ├── tutorials/
│   │   └── GETTING_STARTED.md  # Ex-SETUP_GUIDE.md (renombrado)
│   ├── how-to/
│   │   └── USAGE.md      # Movido sin cambios
│   ├── reference/
│   │   └── API_REFERENCE.md  # Movido sin cambios
│   ├── explanation/
│   │   └── ARCHITECTURE.md   # Movido sin cambios
│   └── archive/
│       └── 2025-12/
│           └── NOTES_BACKUP.md  # Notas archivadas
├── .github/issues/
│   ├── TASK_001.md       # Ex-TODO línea 1
│   ├── TASK_002.md       # Ex-TODO línea 2
│   └── ...
└── src/
```

**Comandos ejecutados:**

```bash
# 1. Crear issues
cat TODO.md | grep "^-" | while read task; do
  gh issue create --title "$task" --label "migrated"
done

# 2. Mover y renombrar
mv SETUP_GUIDE.md docs/tutorials/GETTING_STARTED.md
mv USAGE.md docs/how-to/
mv API_REFERENCE.md docs/reference/
mv ARCHITECTURE.md docs/explanation/

# 3. Archivar notas
mkdir -p docs/archive/2025-12
mv NOTES.md docs/archive/2025-12/NOTES_BACKUP.md

# 4. Eliminar TODO
rm TODO.md

# 5. Commit
git add -A
git commit -m "docs: migrate to Diátaxis framework

- Converted TODO.md to GitHub Issues (30 issues created)
- Moved guides to appropriate quadrants
- Archived temporary notes"
```

---

## 🤝 Mejores Prácticas

1. **Hazlo en etapas** - No migres todo de golpe
2. **Empieza por tracking** - Primero TODO → Issues
3. **Luego clasifica guías** - Una por una a su cuadrante
4. **Preserva historia** - Usa git mv para mantener history
5. **Actualiza links** - No dejes links rotos
6. **Documenta la migración** - Commit message claro

---

## 🔗 Referencias

- **[Diátaxis Framework](https://diataxis.fr/)** - Framework oficial
- **[docs/DOCUMENTATION_SYSTEM.md](./DOCUMENTATION_SYSTEM.md)** - Sistema completo explicado
- **[docs/README.md](./README.md)** - Índice de documentación

---

*Esta guía es parte de Git-Core Protocol v3.2.0 - Diátaxis Documentation System*
