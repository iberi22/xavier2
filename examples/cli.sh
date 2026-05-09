#!/bin/bash
# examples/cli.sh
# Ejemplo de uso de la CLI de Xavier
#
# Prerrequisitos:
#   1. Tener xavier instalado en el PATH
#   2. Tener el servidor corriendo (xavier http --port 8006)
#
# Uso:
#   chmod +x cli.sh
#   ./cli.sh

set -e

echo "=== Xavier CLI Examples ==="
echo ""

# ── 1. Iniciar el servidor ──
echo ">> Starting Xavier server..."
xavier http --port 8006 &
SERVER_PID=$!
sleep 2

# ── 2. Agregar memorias ──
echo ""
echo ">> Adding memories..."
xavier add "Mi primera memoria en Xavier" --title "Hola mundo"
xavier add "Arquitectura hexagonal con Rust para sistemas de memoria" --kind semantic
xavier add "Roadmap Q2: estabilizar API REST y MCP" --kind decision --title "Roadmap Q2 2026"

# ── 3. Búsqueda semántica ──
echo ""
echo ">> Searching for 'arquitectura'..."
xavier search "arquitectura"

# ── 4. Recall por keyword ──
echo ""
echo ">> Recalling 'memoria'..."
xavier recall "memoria"

# ── 5. Estadísticas ──
echo ""
echo ">> Stats..."
xavier stats

# ── 6. Limpiar ──
kill $SERVER_PID 2>/dev/null || true
echo ""
echo "=== Done ==="
