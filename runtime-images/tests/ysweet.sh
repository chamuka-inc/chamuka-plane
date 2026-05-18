#!/usr/bin/env bash
# Smoke test for sandbox-runtime:ysweet
# Verifies: agentd healthy, y-sweet doc creation, y-sweet auth
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

setup_test "ysweet"
build_image "ysweet/Dockerfile.multistage"

echo "==> Running container (agentd:8080 + y-sweet:3000)..."
docker run -d --name "$CONTAINER_NAME" -p 0:8080 -p 0:3000 "$IMAGE"
PORT=$(docker port "$CONTAINER_NAME" 8080 | head -1 | cut -d: -f2)
YSWEET_PORT=$(docker port "$CONTAINER_NAME" 3000 | head -1 | cut -d: -f2)
echo "==> Container running on agentd=$PORT, y-sweet=$YSWEET_PORT"

wait_for_health 30

echo "==> Testing y-sweet doc creation..."

for _ in $(seq 1 30); do
    if curl -sf -X POST -H "Content-Type: application/json" -d '{}' \
        "http://localhost:$YSWEET_PORT/doc/new" > /dev/null 2>&1; then
        break
    fi
    sleep 0.5
done

DOC=$(curl -sf -X POST -H "Content-Type: application/json" -d '{}' \
    "http://localhost:$YSWEET_PORT/doc/new")
DOC_ID=$(echo "$DOC" | jq -r '.docId')

if [[ -z "$DOC_ID" || "$DOC_ID" == "null" ]]; then
    echo "FAILED: y-sweet doc creation (response: $DOC)"
    exit 1
fi
echo "==> doc created: $DOC_ID"

echo "==> Testing y-sweet auth..."
AUTH=$(curl -sf -X POST -H "Content-Type: application/json" -d '{}' \
    "http://localhost:$YSWEET_PORT/doc/$DOC_ID/auth")
WS_URL=$(echo "$AUTH" | jq -r '.url')

if ! echo "$WS_URL" | grep -q 'ws://'; then
    echo "FAILED: auth did not return ws:// URL (response: $AUTH)"
    exit 1
fi
echo "==> auth returned ws URL: $WS_URL"

echo "==> Testing data dir via agentd exec..."
RESP=$(curl -sf -X POST "http://localhost:$PORT/v1/exec" \
    -H "Content-Type: application/json" \
    -d '{"cmd":"/bin/sh","args":["-c","ls -la /data"],"timeout_ms":3000}')

EXIT_CODE=$(echo "$RESP" | jq -r '.exit_code')
if [[ "$EXIT_CODE" != "0" ]]; then
    echo "FAILED: /data dir not accessible (exit_code=$EXIT_CODE)"
    echo "$RESP" | jq .
    exit 1
fi
echo "==> /data dir accessible"

echo "==> PASSED: ysweet image smoke test"
