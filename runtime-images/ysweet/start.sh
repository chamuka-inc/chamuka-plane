#!/usr/bin/env bash
set -euo pipefail

YSWEET_HOST="${YSWEET_HOST:-0.0.0.0}"
YSWEET_PORT="${YSWEET_PORT:-3000}"
YSWEET_DATA_DIR="${YSWEET_DATA_DIR:-/data}"

mkdir -p "$YSWEET_DATA_DIR"

y-sweet serve \
    --host "$YSWEET_HOST" \
    --port "$YSWEET_PORT" \
    "$YSWEET_DATA_DIR" \
    &

for _ in $(seq 1 30); do
    if curl -sf -X POST -H "Content-Type: application/json" -d '{}' \
        "http://127.0.0.1:${YSWEET_PORT}/doc/new" >/dev/null 2>&1; then
        break
    fi
    sleep 0.1
done

exec agentd
