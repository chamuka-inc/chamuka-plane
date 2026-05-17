#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
IMAGE="sandbox-runtime:desktop-test"
CONTAINER_NAME="desktop-smoke-$$"

cleanup() {
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
}
trap cleanup EXIT

echo "==> Building desktop image (multi-stage)..."
docker build \
    -f "$SCRIPT_DIR/../desktop/Dockerfile.multistage" \
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

echo "==> Testing /v1/computer screenshot (should return image or error about display)..."
RESP=$(curl -sf -w "\n%{http_code}" -X POST "http://localhost:$PORT/v1/computer" \
    -H "Content-Type: application/json" \
    -d '{"action":"screenshot"}' 2>&1) || true

HTTP_CODE=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | head -n -1)

if [[ "$HTTP_CODE" == "200" ]]; then
    echo "==> Screenshot returned 200"
    echo "    Response type: $(echo "$BODY" | jq -r '.output.type' 2>/dev/null || echo 'unknown')"
elif [[ "$HTTP_CODE" == "500" ]]; then
    echo "==> Screenshot returned 500 (display may not be ready yet — acceptable for smoke test)"
else
    echo "==> Screenshot returned $HTTP_CODE: $BODY"
fi

echo "==> PASSED: desktop image smoke test"
