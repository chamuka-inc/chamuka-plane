#!/usr/bin/env bash
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

setup_test "node24"
build_image "node24/Dockerfile.multistage"
run_container
wait_for_health 30

assert_exec "node -e 'console.log(42)'" \
    '{"cmd":"node","args":["-e","console.log(42)"],"timeout_ms":10000}' \
    "42"

echo "==> PASSED: node24 image smoke test"
