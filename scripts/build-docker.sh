#!/usr/bin/env bash
# Build Xavier using Docker BuildKit (no cross-compilation needed)
set -e

echo "=== Xavier Docker Build (Native) ==="

cd "$(dirname "$0")/.."

# Ensure Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "ERROR: Docker is not running"
    exit 1
fi

# Build with BuildKit
echo "Building Docker image..."
DOCKER_BUILDKIT=1 docker build \
    --platform linux/amd64 \
    -t iberi22/xavier:latest \
    -f docker/Dockerfile \
    .

echo ""
echo "=== Build successful ==="
docker images iberi22/xavier --format "{{.Repository}}:{{.Tag}} - {{.Size}}"