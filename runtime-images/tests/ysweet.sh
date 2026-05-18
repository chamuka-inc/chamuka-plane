#!/usr/bin/env bash
# Smoke test for sandbox-runtime:ysweet
# Verifies: agentd healthy, y-sweet doc creation, y-sweet auth
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

setup_test "ysweet"
build_image "ysweet/Dockerfile.multistage"
run_container 3000
YSWEET_PORT=$(docker port "$CONTAINER_NAME" 3000 | head -1 | cut -d: -f2)
echo "==> y-sweet on port $YSWEET_PORT"
wait_for_health 30

echo "==> Testing y-sweet doc creation..."
DOC=""
for _ in $(seq 1 30); do
    DOC=$(curl -sf -X POST -H "Content-Type: application/json" -d '{}' \
        "http://localhost:$YSWEET_PORT/doc/new" 2>/dev/null) && break
    DOC=""
    sleep 0.5
done

if [[ -z "$DOC" ]]; then
    echo "FAILED: y-sweet did not become ready within 15s"
    exit 1
fi
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

assert_exec "data dir accessible" \
    '{"cmd":"/bin/sh","args":["-c","ls -la /data"],"timeout_ms":3000}' \
    ""

echo "==> PASSED: ysweet image smoke test"
