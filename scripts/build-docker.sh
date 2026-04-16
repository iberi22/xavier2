#!/usr/bin/env bash
# Build Xavier2 using Docker BuildKit (no cross-compilation needed)
set -e

echo "=== Xavier2 Docker Build (Native) ==="

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
    -t iberi22/xavier2:latest \
    -f docker/Dockerfile \
    .

echo ""
echo "=== Build successful ==="
docker images iberi22/xavier2 --format "{{.Repository}}:{{.Tag}} - {{.Size}}"