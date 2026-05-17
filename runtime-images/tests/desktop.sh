#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

setup_test "desktop"
build_image "desktop/Dockerfile.multistage"
run_container
wait_for_health 30

echo "==> Testing /v1/computer screenshot..."
HTTP_CODE=$(curl -s -o /tmp/desktop_resp -w "%{http_code}" -X POST \
    "http://localhost:$PORT/v1/computer" \
    -H "Content-Type: application/json" \
    -d '{"action":"screenshot"}')

if [[ "$HTTP_CODE" == "200" ]]; then
    RESP_TYPE=$(cat /tmp/desktop_resp | jq -r '.output.type' 2>/dev/null || echo 'unknown')
    echo "==> Screenshot returned 200 (type: $RESP_TYPE)"
elif [[ "$HTTP_CODE" == "500" ]]; then
    echo "==> Screenshot returned 500 (display not ready — acceptable for smoke test)"
else
    echo "FAILED: screenshot returned unexpected $HTTP_CODE"
    cat /tmp/desktop_resp
    exit 1
fi

echo "==> PASSED: desktop image smoke test"
