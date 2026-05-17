#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
IMAGE="sandbox-runtime:node24-test"
CONTAINER_NAME="node24-smoke-$$"

cleanup() {
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> Building node24 image (multi-stage)..."
docker build \
    -f "$SCRIPT_DIR/../node24/Dockerfile.multistage" \
    -t "$IMAGE" \
    "$REPO_ROOT"

echo "==> Running container..."
docker run -d --name "$CONTAINER_NAME" -p 0:8080 "$IMAGE"

PORT=$(docker port "$CONTAINER_NAME" 8080 | head -1 | cut -d: -f2)
echo "==> Container running on port $PORT"

echo "==> Waiting for health endpoint..."
TIMEOUT=30
START=$(date +%s)
while true; do
    if curl -sf "http://localhost:$PORT/health" > /dev/null 2>&1; then
        break
    fi
    ELAPSED=$(( $(date +%s) - START ))
    if [[ $ELAPSED -ge $TIMEOUT ]]; then
        echo "FAILED: /health did not respond within ${TIMEOUT}s"
        docker logs "$CONTAINER_NAME"
        exit 1
    fi
    sleep 0.5
done

echo "==> /health responded OK"

echo "==> Testing /v1/exec (node -e 'console.log(42)')..."
RESP=$(curl -sf -X POST "http://localhost:$PORT/v1/exec" \
    -H "Content-Type: application/json" \
    -d '{"cmd":"node","args":["-e","console.log(42)"],"timeout_ms":10000}')

EXIT_CODE=$(echo "$RESP" | jq -r '.exit_code')
STDOUT=$(echo "$RESP" | jq -r '.stdout')

if [[ "$EXIT_CODE" != "0" ]]; then
    echo "FAILED: exit_code=$EXIT_CODE"
    echo "$RESP" | jq .
    exit 1
fi

if [[ "$(echo "$STDOUT" | tr -d '\n')" != "42" ]]; then
    echo "FAILED: expected stdout '42', got '$STDOUT'"
    exit 1
fi

echo "==> exec returned: exit_code=0, stdout=42"
echo "==> PASSED: node24 image smoke test"
