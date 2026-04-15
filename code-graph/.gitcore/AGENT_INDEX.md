# Agent Index - code-graph

## Agents

| Agente | Rol | Estado |
|--------|-----|--------|
| **Clawd** | Principal | ✅ Activo |
| **Jules** | Orquestador de tareas | ✅ Activo |

## Workflow

```
Clawd → index project → query symbols → Claude Code / Codex
```

## Commands

```bash
# Index a project
code-graph scan ./my-project

# Find functions
code-graph find "function_name"

# List all functions
code-graph functions

# List all structs
code-graph structs

# Show stats
code-graph stats
```

## Integration with OpenClaw

This tool is designed to be used by OpenClaw for:
- Understanding codebase structure
- Finding functions/structs quickly
- Agentic search fallback (filesystem navigation)
- Zero RAG overhead for code

---
*Generated: 2026-03-05*
