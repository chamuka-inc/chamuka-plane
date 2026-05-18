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
    local extra_ports=("$@")
    local port_args=(-p 0:8080)
    for p in "${extra_ports[@]}"; do
        port_args+=(-p "0:$p")
    done
    echo "==> Running container..."
    docker run -d --name "$CONTAINER_NAME" "${port_args[@]}" "$IMAGE"
    PORT=$(docker port "$CONTAINER_NAME" 8080 | head -1 | cut -d: -f2)
    echo "==> Container running on port $PORT"
}

assert_exec() {
    local label="$1" body="$2" expected="$3" expected_exit="${4:-0}"
    echo "==> Testing $label..."
    local resp
    resp=$(curl -sf -X POST "http://localhost:$PORT/v1/exec" \
        -H "Content-Type: application/json" \
        -d "$body")
    local exit_code stdout
    exit_code=$(echo "$resp" | jq -r '.exit_code')
    stdout=$(echo "$resp" | jq -r '.stdout' | tr -d '\n')
    if [[ "$exit_code" != "$expected_exit" ]]; then
        echo "FAILED: $label exit_code=$exit_code (expected $expected_exit)"
        echo "$resp" | jq .
        exit 1
    fi
    if [[ -n "$expected" && "$stdout" != "$expected" ]]; then
        echo "FAILED: $label stdout='$stdout' (expected '$expected')"
        exit 1
    fi
    echo "==> $label: OK"
}

assert_exec_ge() {
    local label="$1" body="$2" minimum="$3"
    echo "==> Testing $label..."
    local resp
    resp=$(curl -sf -X POST "http://localhost:$PORT/v1/exec" \
        -H "Content-Type: application/json" \
        -d "$body")
    local exit_code stdout
    exit_code=$(echo "$resp" | jq -r '.exit_code')
    stdout=$(echo "$resp" | jq -r '.stdout' | tr -d '[:space:]')
    if [[ "$exit_code" != "0" ]]; then
        echo "FAILED: $label exit_code=$exit_code"
        echo "$resp" | jq .
        exit 1
    fi
    if [[ "$stdout" -lt "$minimum" ]]; then
        echo "FAILED: $label got $stdout (expected >= $minimum)"
        exit 1
    fi
    echo "==> $label: $stdout (>= $minimum OK)"
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
