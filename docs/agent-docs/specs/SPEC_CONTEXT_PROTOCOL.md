---
title: "Context Protocol Specification"
type: SPECIFICATION
id: "spec-context-protocol"
created: 2025-12-03
updated: 2025-12-03
agent: protocol-gemini
model: gemini-3-pro
requested_by: system
summary: |
  Standardized protocol for agents to persist state and context in GitHub Issues,
  enabling stateless, pausable, and resumable workflows (12-Factor Agents).
  v2.1: Adds Dynamic Planning, Human-as-Tool, and Metrics.
keywords: [context, protocol, state, xml, 12-factor, acp]
tags: ["#protocol", "#context", "#state"]
protocol_version: 1.5.0
project: Git-Core-Protocol
---

# 🧠 Context Protocol v2.1 (Git-Core)

> **"Own Your Context Window & Unify State"**

Este protocolo define cómo los agentes de IA deben persistir su estado de ejecución y contexto dentro de los GitHub Issues. Esto transforma a los agentes en **Stateless Reducers** (Factor 12) y permite flujos de trabajo pausables y resumibles.

## 1. Principio Fundamental

**El Agente es una Función Pura:**
`Agente(Historial del Issue + Contexto del Repo) -> Siguiente Acción`

No debe existir "memoria" oculta en el chat. Todo el estado necesario para continuar una tarea debe estar visible en el Issue.

## 2. Bloque de Estado del Agente (`<agent-state>`)

Cada vez que un agente realiza una acción significativa o pausa su trabajo, DEBE dejar un comentario en el Issue con un bloque `<agent-state>`.

### Formato XML (v2.1)

```xml
<agent-state>
  <!-- 1. INTENCIÓN Y ESTADO -->
  <intent>implement_auth_flow</intent>
  <step>waiting_for_input</step>
  <progress>45</progress>

  <!-- 2. PLANIFICACIÓN DINÁMICA (Dynamic Workflow) -->
  <plan>
    <item status="done">Analizar requisitos de OAuth</item>
    <item status="done">Crear estructura de base de datos</item>
    <item status="in_progress">Implementar endpoints de API</item>
    <item status="pending">Crear frontend de login</item>
    <item status="pending">Tests de integración</item>
  </plan>

  <!-- 3. HUMAN AS A TOOL (Input Request) -->
  <!-- Usar cuando se necesita información específica, no solo aprobación -->
  <input_request>
    <status>waiting</status> <!-- waiting | received | none -->
    <question>¿Cuál es el Client ID de Google para el entorno de staging?</question>
  </input_request>

  <!-- 4. OBSERVABILIDAD (Metrics) -->
  <!-- Para detectar bucles o consumo excesivo -->
  <metrics>
    <tool_calls>12</tool_calls>
    <errors>0</errors>
    <cost_estimate>0.15</cost_estimate>
  </metrics>

  <!-- 5. MEMORIA TÉCNICA -->
  <memory>
    {
      "last_file_edited": "src/auth/router.ts",
      "db_migration_applied": "20251203_init_users",
      "blockers": []
    }
  </memory>

  <next_action>check_user_response</next_action>
</agent-state>
```

### Campos Permitidos

| Campo | Descripción | Valores Ejemplo |
|-------|-------------|-----------------|
| `<intent>` | Objetivo de alto nivel | `fix_bug`, `refactor`, `deploy` |
| `<step>` | Estado actual del flujo | `planning`, `coding`, `waiting_for_input` |
| `<plan>` | Lista de tareas viva | Items con status `done`, `in_progress`, `pending` |
| `<input_request>` | Solicitud de datos al humano | Pregunta específica + estado |
| `<metrics>` | Telemetría de la sesión | Contadores de uso y errores |
| `<memory>` | Datos clave para retomar | JSON string |
| `<next_action>` | Sugerencia siguiente paso | `review_pr`, `merge`, `ask_user` |

## 3. Flujo de Lectura/Escritura

### Al Iniciar (Lectura)

1. El agente lee el Issue asignado.
2. Busca el **último** bloque `<agent-state>` en los comentarios.
3. Si existe, carga ese estado como su contexto inicial.
4. Si no existe, inicia con estado `planning`.

### Al Finalizar/Pausar (Escritura)

1. El agente determina su nuevo estado.
2. Actualiza el `<plan>` (marca items como done/in_progress).
3. Si necesita info, llena `<input_request>`.
4. Incrementa `<metrics>`.
5. Publica un comentario en el Issue con el bloque `<agent-state>` actualizado.

## 4. Ejemplo de Interacción "Human-as-a-Tool"

**Agente:**
> Necesito credenciales para continuar.
>
> ```xml
> <agent-state>
>   <step>waiting_for_input</step>
>   <input_request>
>     <status>waiting</status>
>     <question>Por favor proporciona la API Key de SendGrid.</question>
>   </input_request>
> </agent-state>
> ```

**Humano:**
> Aquí tienes: `SG.12345...`

**Agente (siguiente ejecución):**
> Gracias. Continuando con la configuración.
>
> ```xml
> <agent-state>
>   <step>coding</step>
>   <input_request>
>     <status>received</status>
>   </input_request>
>   <plan>
>     <item status="in_progress">Configurar servicio de email</item>
>   </plan>
> </agent-state>
> ```

## 5. Beneficios (ACP Alignment)

- **Dynamic Planning:** El plan evoluciona con la realidad, no es estático.
- **Human-as-a-Tool:** Formaliza la interacción humano-agente como un intercambio de datos estructurado.
- **Observability:** Permite detectar agentes "atascados" (high tool calls) automáticamente.
