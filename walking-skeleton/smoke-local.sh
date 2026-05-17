#!/usr/bin/env bash
set -euo pipefail

# Local walking skeleton smoke test.
# Tests agentd directly without Docker/containers.
# Useful for development; the full walking skeleton (run.sh) requires Docker.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Building agentd..."
cd "$REPO_ROOT/crates"
cargo build --release -p agentd 2>/dev/null

BINARY="$REPO_ROOT/crates/target/release/agentd"

echo "==> Starting agentd on ephemeral port..."
PORT=0
$BINARY &
AGENTD_PID=$!
sleep 1

# agentd listens on 8080 by default
ADDR="127.0.0.1:8080"

cleanup() {
    kill $AGENTD_PID 2>/dev/null || true
}
trap cleanup EXIT

echo "==> Testing /health..."
HEALTH=$(curl -sf "http://$ADDR/health")
echo "  $HEALTH"
echo "$HEALTH" | grep -q '"ready"' || { echo "FAIL: health"; exit 1; }

echo "==> Testing /info..."
INFO=$(curl -sf "http://$ADDR/info")
echo "  $INFO"
echo "$INFO" | grep -q '"agentd-headless"' || { echo "FAIL: info"; exit 1; }

echo "==> Testing /v1/exec (echo)..."
EXEC=$(curl -sf -X POST "http://$ADDR/v1/exec" \
    -H 'Content-Type: application/json' \
    -d '{"cmd":"echo","args":["hello-skeleton"],"timeout_ms":5000}')
echo "  $EXEC"
echo "$EXEC" | grep -q '"exit_code":0' || { echo "FAIL: exec exit_code"; exit 1; }
echo "$EXEC" | grep -q 'hello-skeleton' || { echo "FAIL: exec stdout"; exit 1; }

echo "==> Testing /v1/fs/write + /v1/fs/read..."
TMPFILE="/tmp/ws-smoke-$$.txt"
curl -sf -X PUT "http://$ADDR/v1/fs/write" \
    -F "path=$TMPFILE" \
    -F "file=@-;filename=f.txt" <<< "smoke-test-data"
READ=$(curl -sf "http://$ADDR/v1/fs/read?path=$TMPFILE")
echo "  Read: $READ"
[ "$READ" = "smoke-test-data" ] || { echo "FAIL: fs read mismatch"; exit 1; }
rm -f "$TMPFILE"

echo ""
echo "==> Walking skeleton (local smoke): PASS"
