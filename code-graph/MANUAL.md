# code-graph Manual de Uso

> **Codebase Understanding without RAG** - Tree-sitter + Agentic Search

## Tabla de Contenidos

1. [Instalación](#instalación)
2. [Modo CLI](#modo-cli)
3. [Modo Servidor HTTP](#modo-servidor-http)
4. [Docker](#docker)
5. [Seguridad](#seguridad)
6. [API Reference](#api-reference)

---

## Instalación

### Desde código fuente

```bash
cd E:\scripts-python\code-graph
cargo build --release
# Binary en target/release/code-graph.exe
```

### Con Docker

```bash
docker build -t code-graph .
```

---

## Modo CLI

### Escaneo de proyectos

```bash
# Escanear un proyecto
code-graph scan ./mi-proyecto

# Encontrar símbolos
code-graph find "function_name"

# Ver estadísticas
code-graph stats
```

---

## Modo Servidor HTTP

El servidor requiere autenticación por token para todas las endpoints protegidas.

### Iniciar servidor

```bash
# Con variable de entorno (recomendado)
export CODE_GRAPH_TOKEN="mi-token-seguro-muy-largo-12345678"
code-graph serve

# Con argumento (menos seguro)
code-graph serve --token "mi-token-seguro"

# Puerto personalizado
code-graph serve --port 9000 --host 127.0.0.1
```

### Variables de entorno

| Variable | Default | Descripción |
|----------|---------|-------------|
| `CODE_GRAPH_TOKEN` | (requerido) | Token de autenticación (mínimo 16 caracteres) |
| `CODE_GRAPH_PORT` | `8080` | Puerto del servidor |
| `CODE_GRAPH_HOST` | `0.0.0.0` | Host del servidor |

---

## Docker

### Construcción

```bash
docker build -t code-graph:0.2.0 .
```

### Ejecución (Producción)

```bash
# Con token desde variable de entorno
docker run -d \
  --name code-graph \
  -p 8080:8080 \
  -e CODE_GRAPH_TOKEN="token-seguro-muy-largo-12345678" \
  -v /path/to/code:/code:ro \
  code-graph:0.2.0
```

### Docker Compose

```yaml
version: '3.8'

services:
  code-graph:
    build: .
    ports:
      - "8080:8080"
    environment:
      - CODE_GRAPH_TOKEN=${CODE_GRAPH_TOKEN:?TOKEN_REQUIRED}
    volumes:
      - ./code:/code:ro
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "--no-verbose", "--tries=1", "--spider", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### Despliegue en VPS

1. **Generar token seguro:**
   ```bash
   openssl rand -base64 32
   ```

2. **Configurar firewall:**
   ```bash
   # Solo permitir acceso al puerto
   ufw allow 8080/tcp
   ufw enable
   ```

3. **Ejecutar con Docker:**
   ```bash
   docker run -d \
     --restart always \
     --network host \
     -e CODE_GRAPH_TOKEN="TU_TOKEN_AQUI" \
     code-graph:0.2.0
   ```

---

## Seguridad

### Características de Seguridad Implementadas

| Feature | Estado | Descripción |
|---------|--------|-------------|
| **HTTP Token Auth** | ✅ | Bearer token en header Authorization |
| **Path Traversal Protection** | ✅ | Canonicalización y validación de paths |
| **Rate Limiting** | ✅ | Límites en archivos (50k) y símbolos (500k) |
| **Input Validation** | ✅ | Validación de caracteres especiales |
| **Non-root User** | ✅ | Docker corre como appuser (UID 1000) |
| **Token Length Check** | ✅ | Warn si < 16 caracteres |
| **Secure Logging** | ✅ | Token no se imprime completo |

### Requisitos del token

- Mínimo **16 caracteres**
- Usar token generado aleatoriamente
- No compartir en texto plano
- Usar variables de entorno en producción

### Endpoints públicas

| Endpoint | Descripción |
|----------|-------------|
| `/health` | Health check (sin auth) |

### Endpoints protegidas

Todas las demás requieren `Authorization: Bearer <token>`:

- `/api/scan`
- `/api/find`
- `/api/stats`

### Headers requeridos

```http
Authorization: Bearer TU_TOKEN_AQUI
```

### Hardening Recomendado para Producción

1. **Restringir CORS** - Cambiar `allow_origin(Any)` por dominio específico
2. **Usar HTTPS** - Proxy con Nginx/Traefik + SSL
3. **Rate limiting** - Implementar a nivel de proxy
4. **Firewall** - Solo permitir puertos necesarios

---

## API Reference

### Health Check

```http
GET /health
```

Response:
```json
{
  "status": "ok",
  "version": "0.2.0"
}
```

---

### Escanear Proyecto

```http
POST /api/scan
Authorization: Bearer <token>

{
  "path": "/path/to/project"
}
```

Response:
```json
{
  "status": "ok",
  "files": 150,
  "symbols": 2340,
  "languages": {
    "Rust": 80,
    "TypeScript": 50,
    "Python": 20
  },
  "duration_ms": 1250
}
```

Errores posibles:
- `400`: Path no existe, path traversal detectado, demasiados archivos/símbolos

---

### Buscar Símbolos

```http
POST /api/find
Authorization: Bearer <token>

{
  "query": "handle_request",
  "lang": "Rust",
  "limit": 20
}
```

Response:
```json
{
  "count": 5,
  "symbols": [
    {
      "name": "handle_request",
      "kind": "Function",
      "file": "/src/http.rs",
      "line": 42,
      "lang": "Rust"
    }
  ]
}
```

---

### Estadísticas

```http
GET /api/stats
Authorization: Bearer <token>
```

Response:
```json
{
  "status": "ok",
  "files": 150,
  "symbols": 2340,
  "languages": {
    "Rust": 80,
    "TypeScript": 50
  },
  "duration_ms": 0
}
```

---

## Ejemplo con curl

```bash
# Health check
curl http://localhost:8080/health

# Escanear proyecto
curl -X POST http://localhost:8080/api/scan \
  -H "Authorization: Bearer TU_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"path": "/code"}'

# Buscar símbolos
curl -X POST http://localhost:8080/api/find \
  -H "Authorization: Bearer TU_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"query": "main", "limit": 10}'

# Ver estadísticas
curl -X GET http://localhost:8080/api/stats \
  -H "Authorization: Bearer TU_TOKEN"
```

---

## Troubleshooting

### Error: TOKEN_REQUIRED

```bash
# Establecer token antes de iniciar
export CODE_GRAPH_TOKEN="mi-token-seguro"
code-graph serve
```

### Error: 401 Unauthorized

- Verificar que el token sea correcto
- Verificar header: `Authorization: Bearer <token>`

### Error: 400 Path traversal

- El path contiene caracteres inválidos
- Intentar acceder a directorios fuera del path escaneado

### Error: 400 Too many files/símbolos

- El proyecto excede los límites (50k archivos, 500k símbolos)
- Escanear subdirectorios específicos

### Puerto en uso

```bash
# Cambiar puerto
code-graph serve --port 9000
```

---

*Documentación v0.2.0 - Southwest AI Labs*
