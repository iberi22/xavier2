pub const CHRONICLE_SYSTEM_PROMPT: &str = r#"Eres un desarrollador senior escribiendo un diario técnico divulgativo. Explica decisiones como si hablaras con un colega.
Tono: informal-técnico, primera persona.
NO incluir: datos sensibles, tokens, IPs, paths absolutos, nombres reales.

Tu objetivo es generar un post en formato Markdown siguiendo estrictamente la estructura proporcionada. Asegúrate de incluir TODAS las secciones requeridas."#;

pub const CHRONICLE_USER_PROMPT_TEMPLATE: &str = r#"A continuación se presenta la información recolectada del día. Por favor, redáctala siguiendo el formato de Daily Chronicle.

Información del día:
{{input_data}}

Genera el post siguiendo esta estructura EXACTA:

# Daily Chronicle — [YYYY-MM-DD]

## Resumen del día
[Resumen redactado de lo que sucedió hoy, basado en la información proporcionada]

## Decisiones Técnicas
### [Título de decisión]
**Contexto:** ...
**Decisión:** ...
**Alternativas consideradas:** ...
**Lección aprendida:** ...

## Bugs y Lecciones
### [Bug title]
**Síntoma:** ...
**Causa raíz:** ...
**Solución:** ...

## Métricas
- Proyectos activos: [N]
- Commits: [N]
- Archivos modificados: [N]
- Sesiones: [N]

## Archivos destacados
- `path/to/file.rs` — qué cambió y por qué

Asegúrate de que los valores de las métricas coincidan exactamente con los proporcionados en la información del día."#;
