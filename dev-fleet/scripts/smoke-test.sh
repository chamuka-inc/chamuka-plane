#!/usr/bin/env bash
# smoke-test.sh — end-to-end fleet verification.
#
# Spawns a sandbox on each pool, verifies proxy routing,
# and exercises agentd endpoints (health, exec, fs).
#
# Requires: curl, python3, and a running dev fleet.
#
# Prefer invoking via Make so cluster env is set correctly:
#   make smoke          # canonical Vagrant fleet
#   make smoke-docker   # Docker-only fallback fleet
#
# Direct usage (Docker fallback defaults):
#   ./dev-fleet/scripts/smoke-test.sh
#   CONTROLLER=localhost:8080 PROXY=localhost:9090 CLUSTER=localhost:9090 ./dev-fleet/scripts/smoke-test.sh

set -euo pipefail

CONTROLLER="${CONTROLLER:-localhost:8080}"
PROXY="${PROXY:-localhost:9090}"
CLUSTER="${CLUSTER:-localhost:9090}"
IMAGE="${IMAGE:-sandbox-runtime:node24-latest}"
ENCODED_CLUSTER=$(printf '%s' "$CLUSTER" | sed 's/:/%3A/g')

PASS=0
FAIL=0
TOTAL=0

pass() { PASS=$((PASS + 1)); TOTAL=$((TOTAL + 1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL + 1)); TOTAL=$((TOTAL + 1)); echo "  FAIL: $1"; }

json_field() {
    python3 -c "import sys,json; print(json.load(sys.stdin).get('$1',''))" 2>/dev/null <<< "$2"
}

check_json_field() {
    local json="$1" field="$2" expected="$3" label="$4"
    local actual
    actual=$(json_field "$field" "$json")
    if [ "$actual" = "$expected" ]; then
        pass "$label"
    else
        fail "$label (expected '$expected', got '$actual')"
    fi
}

spawn_backend() {
    local pool="$1"
    local ts
    ts="$(date +%s)-$$-$RANDOM"
    local resp
    resp=$(curl -sf -X POST "http://$CONTROLLER/ctrl/connect" \
        -H 'Content-Type: application/json' \
        -d "{
            \"key\": {\"name\": \"smoke-$pool-$ts\", \"namespace\": \"smoke\", \"tag\": \"\"},
            \"spawn_config\": {
                \"id\": \"smoke-$pool-$ts\",
                \"cluster\": \"$CLUSTER\",
                \"pool\": \"$pool\",
                \"executable\": {
                    \"image\": \"$IMAGE\",
                    \"pull_policy\": \"IfNotPresent\",
                    \"env\": {}
                },
                \"lifetime_limit_seconds\": 120
            }
        }") || { fail "spawn on pool=$pool"; return 1; }

    local token backend_id drone
    token=$(json_field token "$resp")
    backend_id=$(json_field backend_id "$resp")
    drone=$(json_field drone "$resp")

    echo "  spawned $backend_id on $drone (pool=$pool)"
    printf '%s|%s|%s' "$token" "$backend_id" "$drone"
}

wait_ready() {
    local backend_id="$1" label="$2"
    for i in $(seq 1 30); do
        local status
        status=$(curl -sf "http://$CONTROLLER/pub/b/$backend_id/status" \
            | python3 -c "import sys,json; print(json.load(sys.stdin).get('status',''))" 2>/dev/null || echo "")
        if [ "$status" = "ready" ]; then
            pass "$label (${i}s)"
            return 0
        fi
        sleep 1
    done
    fail "$label (timeout after 30s)"
    return 1
}

proxy_get() {
    local token="$1" path="$2"
    curl -sf "http://$PROXY/$token$path" -H "Host: $CLUSTER" 2>/dev/null || echo ""
}

proxy_post() {
    local token="$1" path="$2" body="$3"
    curl -sf "http://$PROXY/$token$path" \
        -H "Host: $CLUSTER" \
        -H "Content-Type: application/json" \
        -d "$body" 2>/dev/null || echo ""
}

terminate_backend() {
    local backend_id="$1"
    curl -sf -X POST "http://$CONTROLLER/ctrl/b/$backend_id/soft-terminate" >/dev/null 2>&1 || true
}

# ─── Pre-flight ───────────────────────────────────────────────

echo "=== Pre-flight ==="
echo "  controller: $CONTROLLER"
echo "  proxy:      $PROXY"
echo "  cluster:    $CLUSTER"
echo "  image:      $IMAGE"
echo ""

cluster_state=$(curl -sf "http://$CONTROLLER/ctrl/c/$ENCODED_CLUSTER/state" 2>/dev/null || echo "")
if [ -z "$cluster_state" ]; then
    echo "FATAL: cannot reach controller at $CONTROLLER"
    exit 1
fi

drone_count=$(python3 -c "import sys,json; print(len(json.load(sys.stdin).get('drones',[])))" <<< "$cluster_state")
proxy_count=$(python3 -c "import sys,json; print(len(json.load(sys.stdin).get('proxies',[])))" <<< "$cluster_state")
echo "  cluster: $CLUSTER"
echo "  drones:  $drone_count"
echo "  proxies: $proxy_count"

if [ "$drone_count" -lt 1 ]; then
    echo "FATAL: no drones registered"
    exit 1
fi
pass "cluster online ($drone_count drones, $proxy_count proxies)"

# ─── Test: trusted pool ───────────────────────────────────────

echo ""
echo "=== Trusted pool ==="

TRUSTED_LINE=$(spawn_backend trusted) || exit 1
TRUSTED_TOKEN=$(echo "$TRUSTED_LINE" | tail -1 | cut -d'|' -f1)
TRUSTED_BID=$(echo "$TRUSTED_LINE" | tail -1 | cut -d'|' -f2)
TRUSTED_DRONE=$(echo "$TRUSTED_LINE" | tail -1 | cut -d'|' -f3)

[ -n "$TRUSTED_DRONE" ] \
    && pass "routed to drone ($TRUSTED_DRONE) on trusted pool" \
    || fail "no drone assigned for trusted pool"

wait_ready "$TRUSTED_BID" "trusted backend ready"

health=$(proxy_get "$TRUSTED_TOKEN" "/health")
check_json_field "$health" "status" "ready" "trusted: /health"

exec_resp=$(proxy_post "$TRUSTED_TOKEN" "/v1/exec" '{"cmd":"/bin/sh","args":["-c","node --version"]}')
node_ver=$(json_field stdout "$exec_resp" | tr -d '[:space:]')
exit_code=$(json_field exit_code "$exec_resp")
if [ "$exit_code" = "0" ] && echo "$node_ver" | grep -q "^v"; then
    pass "trusted: exec node --version ($node_ver)"
else
    fail "trusted: exec node --version (exit=$exit_code, out=$node_ver)"
fi

fs_resp=$(proxy_get "$TRUSTED_TOKEN" "/v1/fs/read?path=/etc/hostname")
[ -n "$fs_resp" ] && pass "trusted: fs/read /etc/hostname" || fail "trusted: fs/read /etc/hostname"

list_resp=$(proxy_get "$TRUSTED_TOKEN" "/v1/fs/list?path=/")
entry_count=$(python3 -c "import sys,json; print(len(json.load(sys.stdin).get('entries',[])))" <<< "$list_resp" 2>/dev/null || echo "0")
[ "$entry_count" -gt 5 ] \
    && pass "trusted: fs/list / ($entry_count entries)" \
    || fail "trusted: fs/list / ($entry_count entries)"

# ─── Test: untrusted pool ─────────────────────────────────────

echo ""
echo "=== Untrusted pool ==="

UNTRUSTED_LINE=$(spawn_backend untrusted) || exit 1
UNTRUSTED_TOKEN=$(echo "$UNTRUSTED_LINE" | tail -1 | cut -d'|' -f1)
UNTRUSTED_BID=$(echo "$UNTRUSTED_LINE" | tail -1 | cut -d'|' -f2)
UNTRUSTED_DRONE=$(echo "$UNTRUSTED_LINE" | tail -1 | cut -d'|' -f3)

[ -n "$UNTRUSTED_DRONE" ] \
    && pass "routed to drone ($UNTRUSTED_DRONE) on untrusted pool" \
    || fail "no drone assigned for untrusted pool"

wait_ready "$UNTRUSTED_BID" "untrusted backend ready"

health=$(proxy_get "$UNTRUSTED_TOKEN" "/health")
check_json_field "$health" "status" "ready" "untrusted: /health"

exec_resp=$(proxy_post "$UNTRUSTED_TOKEN" "/v1/exec" '{"cmd":"/bin/sh","args":["-c","echo hello from untrusted"]}')
stdout=$(json_field stdout "$exec_resp")
echo "$stdout" | grep -q "untrusted" \
    && pass "untrusted: exec echo" \
    || fail "untrusted: exec echo (got: $stdout)"

# ─── Test: cross-pool isolation ───────────────────────────────

echo ""
echo "=== Cross-pool isolation ==="

trusted_host=$(json_field stdout "$(proxy_post "$TRUSTED_TOKEN" "/v1/exec" '{"cmd":"/bin/sh","args":["-c","cat /etc/hostname"]}')" | tr -d '[:space:]')
untrusted_host=$(json_field stdout "$(proxy_post "$UNTRUSTED_TOKEN" "/v1/exec" '{"cmd":"/bin/sh","args":["-c","cat /etc/hostname"]}')" | tr -d '[:space:]')

if [ -n "$trusted_host" ] && [ -n "$untrusted_host" ] && [ "$trusted_host" != "$untrusted_host" ]; then
    pass "different containers: trusted=$trusted_host, untrusted=$untrusted_host"
else
    fail "pool isolation (trusted=$trusted_host, untrusted=$untrusted_host)"
fi

if [ "$TRUSTED_DRONE" != "$UNTRUSTED_DRONE" ]; then
    pass "different drones: trusted=$TRUSTED_DRONE, untrusted=$UNTRUSTED_DRONE"
else
    fail "expected different drones (both on $TRUSTED_DRONE)"
fi

# ─── Cleanup ──────────────────────────────────────────────────

echo ""
echo "=== Cleanup ==="
terminate_backend "$TRUSTED_BID"
terminate_backend "$UNTRUSTED_BID"
echo "  terminated $TRUSTED_BID and $UNTRUSTED_BID"

# ─── Summary ──────────────────────────────────────────────────

echo ""
echo "=== Results ==="
echo "  $PASS/$TOTAL passed, $FAIL failed"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
