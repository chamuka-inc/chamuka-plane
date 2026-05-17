#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

setup_test "node24"
build_image "node24/Dockerfile.multistage"
run_container
wait_for_health 30

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
