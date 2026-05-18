#!/usr/bin/env bash
# Smoke test for sandbox-runtime:ysweet-latest
# Verifies: agentd healthy, y-sweet doc creation, y-sweet persistence
set -euo pipefail

source "$(dirname "$0")/lib.sh"

IMAGE="sandbox-runtime:ysweet-latest"
CONTAINER="test-ysweet-$$"
AGENTD_PORT=18080
YSWEET_PORT=13000

cleanup() { docker rm -f "$CONTAINER" >/dev/null 2>&1 || true; }
trap cleanup EXIT

section "Starting $IMAGE"
docker run -d --name "$CONTAINER" \
    -p "$AGENTD_PORT":8080 \
    -p "$YSWEET_PORT":3000 \
    "$IMAGE" >/dev/null

wait_for "http://localhost:$AGENTD_PORT/health" 15

section "agentd health"
check "agentd responds" \
    "$(curl -sf http://localhost:$AGENTD_PORT/health)" '{"status":"ready"}'

section "y-sweet doc creation"
DOC=$(curl -sf -X POST -H "Content-Type: application/json" -d '{}' \
    "http://localhost:$YSWEET_PORT/doc/new")
DOC_ID=$(echo "$DOC" | python3 -c "import sys,json; print(json.load(sys.stdin)['docId'])")
check "doc created" "$([ -n "$DOC_ID" ] && echo ok)" "ok"

section "y-sweet auth"
AUTH=$(curl -sf -X POST -H "Content-Type: application/json" -d '{}' \
    "http://localhost:$YSWEET_PORT/doc/$DOC_ID/auth")
WS_URL=$(echo "$AUTH" | python3 -c "import sys,json; print(json.load(sys.stdin)['url'])")
check "auth returns ws url" "$(echo "$WS_URL" | grep -q 'ws://' && echo ok)" "ok"

section "y-sweet data dir exists"
EXEC=$(curl -sf -X POST -H "Content-Type: application/json" \
    -d '{"cmd":"/bin/sh","args":["-c","ls -la /data"],"timeout_ms":3000}' \
    "http://localhost:$AGENTD_PORT/v1/exec")
check "data dir accessible" \
    "$(echo "$EXEC" | python3 -c "import sys,json; r=json.load(sys.stdin); print('ok' if r['exit_code']==0 else 'fail')")" "ok"

summary
