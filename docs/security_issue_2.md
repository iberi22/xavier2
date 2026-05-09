## Severity: CRITICAL

## Finding
Múltiples archivos usaban `unwrap_or("dev-token")` en paths de autenticación. Esto permite acceso sin token válido.

## Files affected
- `src/cli.rs` (línea ~1710, variable XAVIER_TOKEN)
- `src/main_tui.rs` (línea ~40)
- `src/workspace.rs` (línea ~169)
- `src/integration_test.rs` (test, aceptable)

## Fix Applied
- Cambiado a `.expect("XAVIER_TOKEN environment variable must be set")`
- El servicio ahora falla al iniciar si no hay token válido

## Status: FIXED
