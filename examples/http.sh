#!/bin/bash
# examples/http.sh
# Ejemplo de uso de la API REST de Xavier con curl
#
# Prerrequisitos:
#   1. Tener el servidor Xavier corriendo en http://localhost:8006
#   2. Tener curl y jq instalados
#
# Uso:
#   chmod +x http.sh
#   TOKEN="mi-token-secreto" ./http.sh

TOKEN="${TOKEN:-tu-token-aqui}"
BASE="${BASE:-http://localhost:8006}"

echo "=== Xavier HTTP API Examples ==="
echo ""

# ── 1. Health Check ──
echo ">> 1. Health Check"
curl -s "$BASE/health" | jq .
echo ""

# ── 2. Agregar memoria (texto simple) ──
echo ">> 2. Add Memory (simple)"
curl -s -X POST "$BASE/memory/add" \
  -H "X-Xavier-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Ejemplo de memoria desde API REST",
    "metadata": {
      "kind": "semantic",
      "source": "http.sh example"
    }
  }' | jq .
echo ""

# ── 3. Agregar memoria con título ──
echo ">> 3. Add Memory (with title)"
curl -s -X POST "$BASE/memory/add" \
  -H "X-Xavier-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Xavier es un sistema de contexto y memoria con embeddings vectoriales",
    "title": "Qué es Xavier",
    "metadata": {
      "kind": "semantic",
      "source": "http.sh example"
    }
  }' | jq .
echo ""

# ── 4. Búsqueda semántica ──
echo ">> 4. Search"
curl -s -X POST "$BASE/memory/search" \
  -H "X-Xavier-Token: $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "query": "ejemplo memoria",
    "limit": 5
  }' | jq .
echo ""

# ── 5. Estadísticas ──
echo ">> 5. Stats"
curl -s "$BASE/memory/stats" \
  -H "X-Xavier-Token: $TOKEN" | jq .
echo ""

echo "=== Done ==="
