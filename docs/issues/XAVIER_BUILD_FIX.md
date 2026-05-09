## 🎯 Objetivo
Fix all compilation errors in Xavier so `cargo check` passes and the binary can be built.

## 📋 Problema
Xavier has 5 pre-existing compilation errors that prevent the binary from building:

1. **Line 147** - `E0308`: `match reason` expects `Option<()>` but pattern uses `Ok(()) | Err(_)`
2. **E0432** (3 instances) - Import errors for:
   - `ConsolidationTask`
   - `CoherenceReport`, `RetentionRegularizer`
   - `ModelProviderClient`, `AgentRunTrace`, `System3Mode`

## 📁 Archivos con Problemas

### Core Files
- `src/lib.rs`
- `src/main.rs`
- `src/server/http.rs` - ✅ FIXED (memory_reflect)

### Server
- `src/server/http.rs` - Contains the errors

## ✅ Tareas Requeridas

### 1. Diagnóstico
```bash
cd E:\scripts-python\xavier
cargo check 2>&1 | Select-String "error"
```

### 2. Fix Line 147 Error
The `match reason` in shutdown handler has wrong pattern:
```rust
// Current (broken):
match reason {
    Ok(()) | Err(_) => ...
}

// Should be:
match reason {
    Some(Ok(())) | Some(Err(_)) => ...
}
```

### 3. Fix Import Errors
Check if these modules exist and fix imports:
- `ConsolidationTask`
- `CoherenceReport`
- `RetentionRegularizer`
- `ModelProviderClient`
- `AgentRunTrace`
- `System3Mode`

### 4. Build Verification
```bash
cargo check --all-targets --all-features
cargo build --release
```

## 📊 Criterios de Éxito
- [ ] `cargo check` pasa sin errores
- [ ] `cargo build --release` compila
- [ ] Binary exists at `target/release/xavier.exe`
- [ ] `xavier http` inicia servidor HTTP
- [ ] `xavier mcp` inicia modo MCP-stdio

## 🔧 Entorno
- **Repo:** `iberi22/xavier`
- **Rama:** main
- **Stack:** Rust, tokio, axum
- **Puerto default:** 8006
- **Token:** dev-token

## 📝 Notas
- `src/cli.rs` fue creado y funciona correctamente
- `src/main.rs` fue actualizado para usar CLI
- Solo faltan fixes de compilación para poder build

---

## Checklist para Crear Issue

- [x] Usar template completo
- [x] Incluir comandos de verificación
- [x] Listar archivos específicos
- [x] Agregar criterios de éxito
- [x] Añadir label `jules`
- [ ] Verificar que issue se creó correctamente
