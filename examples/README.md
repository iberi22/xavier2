# Xavier2 Examples

Ejemplos funcionales para interactuar con **Xavier2**, el motor de contexto y memoria con embeddings vectoriales.

## Requisitos

- **CLI:** `xavier2` instalado en el PATH (ver [instalación](../README.md))
- **Server:** El servicio corriendo en `http://localhost:8006`
- **HTTP:** `curl` y `jq` instalados
- **MCP:** `xavier2 mcp` disponible desde la línea de comandos

## Archivos

| Archivo | Descripción |
|---------|-------------|
| `cli.sh` | Uso de la CLI de Xavier2: agregar, buscar, recordar y estadísticas |
| `http.sh` | API REST con curl: health, agregar memoria, búsqueda semántica y stats |
| `mcp.sh` | Protocolo MCP sobre stdio: inicializar, listar herramientas y llamadas |

## Cómo usar

### 1. Iniciar el servidor

```bash
xavier2 http --port 8006
```

> También puedes usar Docker: `docker compose up xavier2`

### 2. Ejecutar ejemplos

```bash
# CLI
chmod +x cli.sh
./cli.sh

# HTTP
chmod +x http.sh
TOKEN="tu-token-aqui" ./http.sh

# MCP
chmod +x mcp.sh
./mcp.sh
```

## Notas

- Los ejemplos usan `tu-token-aqui` como placeholder del token. Reemplázalo por tu token real.
- Los comandos asumen que el binario `xavier2` está disponible globalmente.
- Para entornos Windows, usa los scripts equivalentes en PowerShell que se encuentran en [`scripts/`](../scripts/).

## Siguientes pasos

- Revisa la [documentación de API](../docs/api.md) para ver todos los endpoints disponibles
- Explora el [panel de control web](../panel-ui/) para gestión visual
- Lee [`CONTRIBUTING.md`](../CONTRIBUTING.md) si quieres contribuir
