#!/usr/bin/env bash
# Smoke test for sandbox-runtime:busybox
# Verifies: agentd, bash (not ash), busybox coreutils, jq
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

setup_test "busybox"
build_image "busybox/Dockerfile.multistage"
run_container
wait_for_health 30

assert_exec "bash associative arrays" \
    '{"cmd":"/bin/bash","args":["-c","declare -A m=([k]=v); echo ${m[k]}"],"timeout_ms":5000}' \
    "v"

assert_exec "bash regex" \
    '{"cmd":"/bin/bash","args":["-c","[[ \"hello\" =~ ^hel ]] && echo match"],"timeout_ms":5000}' \
    "match"

assert_exec_ge "busybox applets >= 400" \
    '{"cmd":"/bin/sh","args":["-c","busybox --list | wc -l"],"timeout_ms":5000}' \
    400

assert_exec "jq" \
    '{"cmd":"/bin/bash","args":["-c","echo '\''{ \"x\": 42 }'\'' | jq .x"],"timeout_ms":5000}' \
    "42"

assert_exec "wget" \
    '{"cmd":"/bin/sh","args":["-c","wget -q -O /dev/null http://localhost:8080/health && echo ok"],"timeout_ms":5000}' \
    "ok"

echo "==> PASSED: busybox image smoke test"
