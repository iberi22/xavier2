#!/bin/bash
# launch_xavier2.sh - Lanza Xavier2 y espera a que esté listo

echo "🚀 Launching Xavier2..."

# Cambiar al directorio de Xavier2
cd E:/scripts-python/xavier2

# Verificar si ya está corriendo
if curl -s http://localhost:8003/health > /dev/null 2>&1; then
    echo "✅ Xavier2 already running"
    exit 0
fi

# Verificar Docker
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found"
    exit 1
fi

# Verificar si hay docker-compose
if [ -f "docker-compose.yml" ]; then
    echo "📦 Starting with docker-compose..."
    docker-compose up -d

    # Esperar a que esté listo
    echo "⏳ Waiting for Xavier2..."
    for i in {1..30}; do
        if curl -s http://localhost:8003/health > /dev/null 2>&1; then
            echo "✅ Xavier2 is ready!"
            exit 0
        fi
        sleep 2
    done

    echo "❌ Xavier2 failed to start"
    exit 1
else
    # Intentar con docker run
    echo "📦 Starting with docker run..."
    docker run -d --name xavier2 -p 8003:8003 iberi22/xavier2:latest

    echo "⏳ Waiting..."
    sleep 10

    if curl -s http://localhost:8003/health > /dev/null 2>&1; then
        echo "✅ Xavier2 is ready!"
    else
        echo "❌ Failed"
    fi
fi
