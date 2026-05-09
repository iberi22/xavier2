## Severity: HIGH

## Finding
`#[derive(Debug)]` en structs con información sensible expone credenciales en logs y output de debug.

## Files affected
- `src/embedding/openai.rs` — `OpenAIEmbedder` (contiene `api_key`)
- `src/security/auth.rs` — `Claims` y `User` (contienen token/password)

## Fix Applied
- Removido `#[derive(Debug)]` de `OpenAIEmbedder`, `Claims`, `User`
- Agregado manual `impl fmt::Debug for OpenAIEmbedder` que redacta `api_key`

## Status: FIXED
