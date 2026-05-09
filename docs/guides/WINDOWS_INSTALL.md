# Xavier — Instalación en Windows

## Requisitos

- Windows 10/11
- Rust toolchain (solo para compilar) → https://rustup.rs
- Opcional: Ollama (para embeddings locales) → https://ollama.com

## Instalación rápida

```powershell
# 1. Compilar (desde la raíz del proyecto)
cargo build --release --bin xavier -j 1

# 2. Ejecutar el instalador (como Administrador)
.\install.ps1
```

Esto:
- Copia `xavier.exe` a `%LOCALAPPDATA%\Xavier\bin\`
- Agrega esa carpeta al PATH de usuario
- Genera un `XAVIER_TOKEN` aleatorio
- Crea `%LOCALAPPDATA%\Xavier\data\` para la base de datos
- Copia `.env.example` como configuración base

> **Reinicia la terminal** después de instalar para que el PATH surta efecto.

## Uso

```powershell
# Ver ayuda
xavier --help

# Iniciar servidor
xavier serve

# Monitor en vivo
xavier monitor

# Ver estado
xavier status
```

## Configuración

Editar `%LOCALAPPDATA%\Xavier\config\.env` con tus valores:

| Variable | Descripción | Default |
|---|---|---|
| `XAVIER_TOKEN` | Token de API (requerido) | (generado) |
| `XAVIER_PORT` | Puerto del servidor | 8003 |
| `XAVIER_MEMORY_BACKEND` | Backend: `vec` (SQLite) o `surreal` | `vec` |
| `XAVIER_EMBEDDING_URL` | URL de embeddings (Ollama) | `http://localhost:11434/v1` |
| `XAVIER_EMBEDDING_MODEL` | Modelo de embeddings | `nomic-embed-text` |
| `XAVIER_MODEL_PROVIDER` | Proveedor LLM | `local` |
| `RUST_LOG` | Nivel de logging | `info` |

## Docker (alternativa)

```powershell
docker compose --profile core up -d
```

## Desinstalación

```powershell
# Remover del PATH
$path = [Environment]::GetEnvironmentVariable("PATH", "User")
$path = ($path.Split(';') | Where-Object { $_ -ne "$env:LOCALAPPDATA\Xavier\bin" }) -join ';'
[Environment]::SetEnvironmentVariable("PATH", $path, "User")

# Eliminar directorio
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\Xavier"
```
