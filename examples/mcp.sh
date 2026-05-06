#!/bin/bash
# examples/mcp.sh
# Ejemplo de uso del protocolo MCP (Model Context Protocol) con Xavier2
#
# Prerrequisitos:
#   1. Tener xavier2 instalado en el PATH
#
# Uso:
#   chmod +x mcp.sh
#   ./mcp.sh

echo "=== Xavier2 MCP Examples ==="
echo ""

# ── 1. Inicializar sesión MCP (stdio) ──
echo ">> 1. MCP Initialize"
echo '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-03-26",
    "capabilities": {},
    "clientInfo": {
      "name": "test-client",
      "version": "1.0"
    }
  }
}' | xavier2 mcp
echo ""

# ── 2. Listar herramientas disponibles ──
echo ">> 2. MCP List Tools"
echo '{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}' | xavier2 mcp
echo ""

# ── 3. Llamar a la herramienta add_memory ──
echo ">> 3. MCP Call: add_memory"
echo '{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "add_memory",
    "arguments": {
      "content": "Memoria agregada vía MCP",
      "metadata": {
        "kind": "semantic"
      }
    }
  }
}' | xavier2 mcp
echo ""

# ── 4. Llamar a la herramienta search_memory ──
echo ">> 4. MCP Call: search_memory"
echo '{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "search_memory",
    "arguments": {
      "query": "MCP",
      "limit": 5
    }
  }
}' | xavier2 mcp
echo ""

echo "=== Done ==="
