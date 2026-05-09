## Severity: HIGH

## Finding
`prompt_guard.rs` tenía patrones incompletos para sanitización de inyecciones de prompt.

## Gaps fixed
- Zero-width characters Unicode (U+200B, U+200C, U+200D, U+FEFF, U+2060, U+180E) — usado para ocultar payloads
- Template injection `${...}` patterns
- HTML `<script` injection
- Event handlers `onerror=`, `onload=` injection
- 10+ nuevos patterns agregados a `dangerous_patterns`

## Files affected
- `src/security/prompt_guard.rs`

## Fix Applied
- Agregado detección de zero-width chars en `detect()` con confidence 0.6
- Extendido `sanitize()` con 5 categorías de patterns adicionales
- Múltiples vectors de inyección bloqueados

## Status: FIXED
