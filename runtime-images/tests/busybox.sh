#!/usr/bin/env bash
# Smoke test for sandbox-runtime:busybox
# Verifies: agentd, bash (not ash), busybox coreutils, jq
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

setup_test "busybox"
build_image "busybox/Dockerfile.multistage"
run_container
wait_for_health 30

echo "==> Testing bash (associative arrays)..."
RESP=$(curl -sf -X POST "http://localhost:$PORT/v1/exec" \
    -H "Content-Type: application/json" \
    -d '{"cmd":"/bin/bash","args":["-c","declare -A m=([k]=v); echo ${m[k]}"],"timeout_ms":5000}')

EXIT_CODE=$(echo "$RESP" | jq -r '.exit_code')
STDOUT=$(echo "$RESP" | jq -r '.stdout' | tr -d '\n')

if [[ "$EXIT_CODE" != "0" || "$STDOUT" != "v" ]]; then
    echo "FAILED: bash associative array test (exit=$EXIT_CODE, stdout='$STDOUT')"
    echo "$RESP" | jq .
    exit 1
fi
echo "==> bash associative arrays: OK"

echo "==> Testing bash regex..."
RESP=$(curl -sf -X POST "http://localhost:$PORT/v1/exec" \
    -H "Content-Type: application/json" \
    -d '{"cmd":"/bin/bash","args":["-c","[[ \"hello\" =~ ^hel ]] && echo match"],"timeout_ms":5000}')

STDOUT=$(echo "$RESP" | jq -r '.stdout' | tr -d '\n')
if [[ "$STDOUT" != "match" ]]; then
    echo "FAILED: bash regex test (stdout='$STDOUT')"
    exit 1
fi
echo "==> bash regex: OK"

echo "==> Testing busybox applet count..."
RESP=$(curl -sf -X POST "http://localhost:$PORT/v1/exec" \
    -H "Content-Type: application/json" \
    -d '{"cmd":"/bin/sh","args":["-c","busybox --list | wc -l"],"timeout_ms":5000}')

APPLETS=$(echo "$RESP" | jq -r '.stdout' | tr -d '[:space:]')
if [[ "$APPLETS" -lt 400 ]]; then
    echo "FAILED: expected >= 400 busybox applets, got $APPLETS"
    exit 1
fi
echo "==> busybox applets: $APPLETS (>= 400 OK)"

echo "==> Testing jq..."
RESP=$(curl -sf -X POST "http://localhost:$PORT/v1/exec" \
    -H "Content-Type: application/json" \
    -d '{"cmd":"/bin/bash","args":["-c","echo '\''{ \"x\": 42 }'\'' | jq .x"],"timeout_ms":5000}')

STDOUT=$(echo "$RESP" | jq -r '.stdout' | tr -d '[:space:]')
if [[ "$STDOUT" != "42" ]]; then
    echo "FAILED: jq test (stdout='$STDOUT')"
    exit 1
fi
echo "==> jq: OK"

echo "==> Testing wget (via busybox)..."
RESP=$(curl -sf -X POST "http://localhost:$PORT/v1/exec" \
    -H "Content-Type: application/json" \
    -d '{"cmd":"/bin/sh","args":["-c","wget -q -O /dev/null http://localhost:8080/health && echo ok"],"timeout_ms":5000}')

STDOUT=$(echo "$RESP" | jq -r '.stdout' | tr -d '\n')
if [[ "$STDOUT" != "ok" ]]; then
    echo "FAILED: wget test (stdout='$STDOUT')"
    exit 1
fi
echo "==> wget: OK"

echo "==> PASSED: busybox image smoke test"
