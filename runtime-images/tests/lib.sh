#!/usr/bin/env bash
# Shared test harness for runtime image smoke tests.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[1]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

setup_test() {
    local name="$1"
    IMAGE="sandbox-runtime:${name}-test"
    CONTAINER_NAME="${name}-smoke-$$"
    trap 'docker rm -f "$CONTAINER_NAME" 2>/dev/null || true' EXIT
}

build_image() {
    local dockerfile="$1"
    echo "==> Building image (multi-stage)..."
    docker build \
        -f "$SCRIPT_DIR/../${dockerfile}" \
        -t "$IMAGE" \
        "$REPO_ROOT"
}

run_container() {
    echo "==> Running container..."
    docker run -d --name "$CONTAINER_NAME" -p 0:8080 "$IMAGE"
    PORT=$(docker port "$CONTAINER_NAME" 8080 | head -1 | cut -d: -f2)
    echo "==> Container running on port $PORT"
}

wait_for_health() {
    local timeout="${1:-30}"
    echo "==> Waiting for health endpoint..."
    local start
    start=$(date +%s)
    while true; do
        if curl -sf "http://localhost:$PORT/health" > /dev/null 2>&1; then
            echo "==> /health responded OK"
            return 0
        fi
        local elapsed=$(( $(date +%s) - start ))
        if [[ $elapsed -ge $timeout ]]; then
            echo "FAILED: /health did not respond within ${timeout}s"
            docker logs "$CONTAINER_NAME"
            exit 1
        fi
        sleep 0.5
    done
}
