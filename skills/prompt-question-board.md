# Prompt Template: Question Board Card Generator

## Purpose
This template guides the LLM to generate structured question cards that can be stored in Xavier and rendered in the Questions Board UI.

## Question Data Structure

```typescript
interface Question {
  id: string;                    // Unique identifier: q-{timestamp}
  title: string;                 // Short, descriptive title
  content: string;               // Full question details and context
  priority: 'urgent' | 'this_week' | 'backlog';
  project: string;               // Project name
  category: 'negocio' | 'tecnico' | 'ventas' | 'legal' | 'producto';
  status: 'open' | 'answered' | 'dismissed';
  answer?: string;               // Optional answer when status is 'answered'
  created_at: string;             // ISO timestamp
  updated_at: string;            // ISO timestamp
}
```

## Xavier Storage Path
```
sweat-operations/questions/{id}
```

## Prompt Template for Generating Questions

```
Eres un asistente de SWAL (SouthWest AI Labs) especializado en gestionar preguntas de negocio y operaciones.

Genera una pregunta estructurada basada en la siguiente descripción:

{TEMA/DESCRIPCIÓN}

## Reglas de Generación:

1. **Título**: Sea conciso y descriptivo (máx. 100 caracteres)
2. **Contenido**: Incluir contexto, stakeholders involucrados, y qué se necesita decidir
3. **Prioridad**:
   - 'urgent': Requiere acción inmediata, bloquea otros trabajos
   - 'this_week': Importante para esta semana
   - 'backlog': Nice to have, sin urgencia
4. **Categoría**:
   - 'negocio': Estrategia, modelo de negocio, partnerships
   - 'tecnico': Desarrollo, arquitectura, infraestructura
   - 'ventas': Conversiones, clientes, pitch
   - 'legal': Compliance, contratos, términos
   - 'producto': Features, UX, roadmap
5. **Proyecto**: Nombre del proyecto SWAL relacionado

## Ejemplo de Output JSON:

```json
{
  "id": "q-1712345678900",
  "title": "¿Cuál es el modelo de pricing para Synapse?",
  "content": "Necesitamos definir cómo cobramos por el uso del protocolo Synapse. Opciones: subscription, usage-based, o hybrid. Stakeholders: equipo Synapse, Bel.",
  "priority": "urgent",
  "project": "Synapse Protocol",
  "category": "negocio",
  "status": "open",
  "created_at": "2026-03-23T12:00:00.000Z",
  "updated_at": "2026-03-23T12:00:00.000Z"
}
```

## Para Actualizar/Rresponder Preguntas:

```json
{
  "id": "q-1712345678900",
  "status": "answered",
  "answer": "El modelo será hybrid: $99/mes base + $0.001 por mensaje. Efectivo desde Q2 2026.",
  "updated_at": "2026-03-23T14:30:00.000Z"
}
```

## Integration with Xavier

Para guardar en Xavier:
```bash
curl -X POST "http://localhost:8003/memory/add" \
  -H "X-Xavier-Token: dev-token" \
  -H "Content-Type: application/json" \
  -d '{
    "path": "sweat-operations/questions/q-{id}",
    "content": "{json_string}",
    "metadata": {
      "type": "question",
      "priority": "{priority}",
      "status": "{status}",
      "category": "{category}"
    }
  }'
```

Para buscar preguntas:
```bash
curl -X POST "http://localhost:8003/memory/search" \
  -H "X-Xavier-Token: dev-token" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "sweat-operations questions",
    "limit": 100
  }'
```

## UI Rendering

Los componentes React disponibles en `panel-ui/src/components/`:
- `QuestionCard.tsx`: Renderiza una pregunta individual
- `QuestionForm.tsx`: Formulario para crear/editar preguntas
- `QuestionsBoard.tsx`: Lista completa con filtros y estadísticas

Todos usan `@openuidev/react-ui` para styling consistente.
