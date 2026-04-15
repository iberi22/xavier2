#!/bin/sh
set -eu

BACKEND="${XAVIER2_BACKEND:-vec}"
PORT="${XAVIER2_PORT:-8003}"
VERSION="${XAVIER2_VERSION:-unknown}"

if [ -z "${XAVIER2_TOKEN:-}" ]; then
    echo "xavier2: XAVIER2_TOKEN is required" >&2
    exit 1
fi

export XAVIER2_BACKEND="${BACKEND}"
export XAVIER2_MEMORY_BACKEND="${XAVIER2_MEMORY_BACKEND:-$BACKEND}"
export XAVIER2_PORT="${PORT}"

echo "xavier2: starting version=${VERSION} backend=${XAVIER2_MEMORY_BACKEND} port=${XAVIER2_PORT}"

exec /usr/local/bin/xavier2-bin "$@"