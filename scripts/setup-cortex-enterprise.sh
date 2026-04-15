#!/bin/bash
# Xavier2 Enterprise Setup Script
# Creates Cloudflare Tunnel and starts Xavier2

set -e

echo "====================================="
echo "Xavier2 Enterprise Setup"
echo "====================================="

# Check if we have a tunnel token
if [ -z "$CLOUDFLARE_TUNNEL_TOKEN" ]; then
    echo "[INFO] No tunnel token found. Creating new tunnel..."

    # Create new tunnel
    TUNNEL_NAME="xavier2-enterprise-$(date +%s)"
    cloudflared tunnel create "$TUNNEL_NAME" 2>/dev/null || true

    # Get tunnel credentials
    CREDENTIALS_FILE="$HOME/.cloudflared/credentials.json"

    if [ -f "$CREDENTIALS_FILE" ]; then
        export CLOUDFLARE_TUNNEL_TOKEN=$(cat "$CREDENTIALS_FILE" | grep -oP '"TunnelID": "\K[^"]+' | head -1)
        echo "[OK] Tunnel credentials found"
    else
        echo "[ERROR] Could not create tunnel. Please set CLOUDFLARE_TUNNEL_TOKEN manually"
        echo "        Or run: cloudflared login"
        exit 1
    fi
fi

# Copy config to cloudflared folder
mkdir -p cloudflared
cp docker/cloudflared/Xavier2file cloudflared/

# Set environment
export XAVIER2_API_KEY=${XAVIER2_API_KEY:-xavier2-enterprise-$(date +%s)}

# Start services
echo "[INFO] Starting Xavier2 Enterprise..."
docker compose -f docker/docker-compose.xavier2-enterprise.yml up -d

echo ""
echo "====================================="
echo "Setup Complete!"
echo "====================================="
echo ""
echo "Xavier2 is running at:"
echo "  - Local: http://localhost:8003"
echo "  - Cloudflare: https://xavier2.swallowai.com (when DNS propagates)"
echo ""
echo "API Key: $XAVIER2_API_KEY"
echo ""
echo "To check status:"
echo "  docker ps"
echo "  docker logs xavier2-enterprise"
echo "  docker logs cloudflared-xavier2"
