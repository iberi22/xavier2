# Xavier2 — Instalación en Windows

## Requisitos

- Windows 10/11
- Rust toolchain (solo para compilar) → https://rustup.rs
- Opcional: Ollama (para embeddings locales) → https://ollama.com

## Instalación rápida

```powershell
# 1. Compilar (desde la raíz del proyecto)
cargo build --release --bin xavier2 -j 1

# 2. Ejecutar el instalador (como Administrador)
.\install.ps1
```

Esto:
- Copia `xavier2.exe` a `%LOCALAPPDATA%\Xavier2\bin\`
- Agrega esa carpeta al PATH de usuario
- Genera un `XAVIER2_TOKEN` aleatorio
- Crea `%LOCALAPPDATA%\Xavier2\data\` para la base de datos
- Copia `.env.example` como configuración base

> **Reinicia la terminal** después de instalar para que el PATH surta efecto.

## Uso

```powershell
# Ver ayuda
xavier2 --help

# Iniciar servidor
xavier2 serve

# Monitor en vivo
xavier2 monitor

# Ver estado
xavier2 status
```

## Configuración

Editar `%LOCALAPPDATA%\Xavier2\config\.env` con tus valores:

| Variable | Descripción | Default |
|---|---|---|
| `XAVIER2_TOKEN` | Token de API (requerido) | (generado) |
| `XAVIER2_PORT` | Puerto del servidor | 8003 |
| `XAVIER2_MEMORY_BACKEND` | Backend: `vec` (SQLite) o `surreal` | `vec` |
| `XAVIER2_EMBEDDING_URL` | URL de embeddings (Ollama) | `http://localhost:11434/v1` |
| `XAVIER2_EMBEDDING_MODEL` | Modelo de embeddings | `nomic-embed-text` |
| `XAVIER2_MODEL_PROVIDER` | Proveedor LLM | `local` |
| `RUST_LOG` | Nivel de logging | `info` |

## Docker (alternativa)

```powershell
docker compose --profile core up -d
```

## Desinstalación

```powershell
# Remover del PATH
$path = [Environment]::GetEnvironmentVariable("PATH", "User")
$path = ($path.Split(';') | Where-Object { $_ -ne "$env:LOCALAPPDATA\Xavier2\bin" }) -join ';'
[Environment]::SetEnvironmentVariable("PATH", $path, "User")

# Eliminar directorio
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\Xavier2"
```
