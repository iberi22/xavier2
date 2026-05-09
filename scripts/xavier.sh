#!/bin/sh
set -eu

BACKEND="${XAVIER_BACKEND:-vec}"
PORT="${XAVIER_PORT:-8003}"
VERSION="${XAVIER_VERSION:-unknown}"

if [ -z "${XAVIER_TOKEN:-}" ]; then
    echo "xavier: XAVIER_TOKEN is required" >&2
    exit 1
fi

export XAVIER_BACKEND="${BACKEND}"
export XAVIER_MEMORY_BACKEND="${XAVIER_MEMORY_BACKEND:-$BACKEND}"
export XAVIER_PORT="${PORT}"

echo "xavier: starting version=${VERSION} backend=${XAVIER_MEMORY_BACKEND} port=${XAVIER_PORT}"

exec /usr/local/bin/xavier-bin "$@"