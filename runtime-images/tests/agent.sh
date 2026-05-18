#!/usr/bin/env bash
# Smoke test for sandbox-runtime:agent
# Verifies: agentd, bash, git, jq, curl, node
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

setup_test "agent"
build_image "agent/Dockerfile.multistage"
run_container
wait_for_health 30

assert_exec "bash" \
    '{"cmd":"/bin/bash","args":["-c","declare -A m=([k]=v); echo ${m[k]}"],"timeout_ms":5000}' \
    "v"

assert_exec "node" \
    '{"cmd":"node","args":["-e","console.log(42)"],"timeout_ms":10000}' \
    "42"

assert_exec "git" \
    '{"cmd":"git","args":["--version"],"timeout_ms":5000}' \
    ""

assert_exec "jq" \
    '{"cmd":"/bin/bash","args":["-c","echo '\''{ \"x\": 42 }'\'' | jq .x"],"timeout_ms":5000}' \
    "42"

assert_exec "curl" \
    '{"cmd":"curl","args":["-sf","http://localhost:8080/health"],"timeout_ms":5000}' \
    ""

assert_exec "wget" \
    '{"cmd":"/bin/bash","args":["-c","wget -q -O /dev/null http://localhost:8080/health && echo ok"],"timeout_ms":5000}' \
    "ok"

echo "==> PASSED: agent image smoke test"
