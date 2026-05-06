#!/bin/bash
# examples/cli.sh
# Ejemplo de uso de la CLI de Xavier2
#
# Prerrequisitos:
#   1. Tener xavier2 instalado en el PATH
#   2. Tener el servidor corriendo (xavier2 http --port 8006)
#
# Uso:
#   chmod +x cli.sh
#   ./cli.sh

set -e

echo "=== Xavier2 CLI Examples ==="
echo ""

# ── 1. Iniciar el servidor ──
echo ">> Starting Xavier2 server..."
xavier2 http --port 8006 &
SERVER_PID=$!
sleep 2

# ── 2. Agregar memorias ──
echo ""
echo ">> Adding memories..."
xavier2 add "Mi primera memoria en Xavier2" --title "Hola mundo"
xavier2 add "Arquitectura hexagonal con Rust para sistemas de memoria" --kind semantic
xavier2 add "Roadmap Q2: estabilizar API REST y MCP" --kind decision --title "Roadmap Q2 2026"

# ── 3. Búsqueda semántica ──
echo ""
echo ">> Searching for 'arquitectura'..."
xavier2 search "arquitectura"

# ── 4. Recall por keyword ──
echo ""
echo ">> Recalling 'memoria'..."
xavier2 recall "memoria"

# ── 5. Estadísticas ──
echo ""
echo ">> Stats..."
xavier2 stats

# ── 6. Limpiar ──
kill $SERVER_PID 2>/dev/null || true
echo ""
echo "=== Done ==="
