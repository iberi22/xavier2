---
title: "Prompt para Mejorar Recetas"
type: PROMPT
id: "prompt-improve-recipes"
created: 2025-12-01
updated: 2025-12-01
agent: copilot
model: gemini-3-pro
requested_by: system
summary: |
  Prompt para analizar y mejorar el repositorio de recetas de agentes.
keywords: [prompt, ai, recipes, improvement]
tags: ["#prompt", "#ai", "#optimization"]
project: Git-Core-Protocol
---

# 🤖 Prompt para Mejorar el Repositorio de Recetas

Copia y pega este prompt en tu sesión de IA (Copilot, Claude, etc.) para que analice y mejore tu repositorio `agents-flows-recipes`.

---

**Prompt:**

```markdown
Actúa como un **Arquitecto de Sistemas de IA** experto en flujos de trabajo y automatización.

**Objetivo:**
Analizar y mejorar la estructura y contenido del repositorio `agents-flows-recipes` para hacerlo más robusto, modular y fácil de consumir por otros agentes de IA.

**Contexto:**
Este repositorio contiene "Recetas" (Roles/Personas) en formato Markdown y POML que son consumidas dinámicamente por el `Git-Core Protocol`. Queremos que estas recetas sean la fuente de verdad para el comportamiento de los agentes.

**Tareas a realizar:**

1.  **Estandarización de Estructura:**
    *   Revisa la estructura de carpetas (`engineering`, `design`, etc.). ¿Es escalable?
    *   Propón una estructura canónica para cada archivo de receta (Frontmatter, Secciones obligatorias, Ejemplos).

2.  **Mejora de Metadatos (Frontmatter):**
    *   Diseña un esquema de metadatos YAML robusto para cada receta.
    *   Debe incluir: `version`, `author`, `capabilities` (skills requeridos), `context_window_required`, y `related_recipes`.

3.  **Integración de Skills (MCP):**
    *   Define cómo una receta puede declarar explícitamente qué herramientas MCP necesita.
    *   Ejemplo: `mcp_requirements: ["postgres-server", "github-tools"]`.

4.  **Validación:**
    *   Crea un script (o workflow de GitHub Actions) que valide que todas las recetas cumplan con el estándar definido (linting de recetas).

5.  **Documentación:**
    *   Genera un `README.md` raíz que explique cómo contribuir una nueva receta y cómo probarla localmente.

**Entregable:**
Por favor, genera un plan de acción detallado con los cambios propuestos y ejemplos de código para la nueva estructura de una "Receta Maestra".
```

