#!/usr/bin/env bash
# scripts/smoke-test.sh
#
# End-to-end test using the real Plane API.
#
# Routes verified against plane/src/controller/mod.rs and
# common/src/lib.rs:
#   GET  /pub/health                       — public health
#   GET  /ctrl/status                      — controller status
#   GET  /ctrl/c/{cluster}/state           — cluster state (drones, proxies)
#   POST /ctrl/connect                     — connect request
#   GET  /pub/b/{backend}/status           — backend status (no auth)
#   POST /ctrl/b/{backend}/soft-terminate  — terminate

set -euo pipefail

CONTROLLER_URL="${CONTROLLER_URL:-http://127.0.0.1:8080}"
CLUSTER_NAME="${CLUSTER_NAME:-192.168.56.1:9090}"
DEMO_IMAGE="${DEMO_IMAGE:-ghcr.io/jamsocket/demo-image-drop-four}"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

ok()   { echo -e "${GREEN}✓${NC} $*"; }
fail() { echo -e "${RED}✗${NC} $*"; exit 1; }

# URL-encode the colon in the cluster name for path segments.
encoded_cluster=$(printf '%s' "${CLUSTER_NAME}" | sed 's/:/%3A/g')

echo "Smoke test against ${CONTROLLER_URL}"
echo "Cluster: ${CLUSTER_NAME}"
echo ""

# ---- 1. Public health check ----------------------------------------------

if curl -sf "${CONTROLLER_URL}/pub/health" >/dev/null; then
    ok "Controller /pub/health responds"
else
    fail "Controller at ${CONTROLLER_URL} is not responding to /pub/health"
fi

# ---- 2. Cluster state (drone registration) -------------------------------

state=$(curl -sf "${CONTROLLER_URL}/ctrl/c/${encoded_cluster}/state" || echo '{}')
# ClusterState shape: { drones: [...], proxies: [...] }
drone_count=$(echo "${state}" | jq '.drones | length' 2>/dev/null || echo 0)
proxy_count=$(echo "${state}" | jq '.proxies | length' 2>/dev/null || echo 0)

if [[ "${drone_count}" -ge 1 ]]; then
    ok "${drone_count} drone(s) and ${proxy_count} proxy(ies) in cluster"
    if command -v jq >/dev/null 2>&1; then
        echo "${state}" | jq -r '.drones[] | "    - drone: \(.name // .id // "?")"' 2>/dev/null || true
    fi
else
    fail "No drones in cluster state. Check 'make logs' or 'journalctl -u plane-drone' inside a VM."
fi

# ---- 3. Connect request ---------------------------------------------------
#
# ConnectRequest body shape (from common/src/types/mod.rs):
#   {
#     key?: { name, namespace?, tag? },
#     spawn_config?: {
#       cluster?, pool?, executable: <JSON>, lifetime_limit_seconds?,
#       max_idle_seconds?, use_static_token?, subdomain?, id?
#     },
#     user?, auth?
#   }
#
# executable is deserialized to DockerExecutorConfig:
#   { image, pull_policy?, credentials?, env?, resource_limits?, mount?,
#     network_name? }

echo ""
echo "Issuing connect request..."

key_name="smoke-$(date +%s)-$$"

connect_body=$(cat <<JSON
{
  "key": {
    "name": "${key_name}",
    "namespace": "",
    "tag": ""
  },
  "spawn_config": {
    "executable": {
      "image": "${DEMO_IMAGE}"
    },
    "cluster": "${CLUSTER_NAME}",
    "lifetime_limit_seconds": 300,
    "max_idle_seconds": 60
  }
}
JSON
)

# /ctrl/connect is on the control side; in production it'd require auth
# behind a reverse proxy. In local dev with no --forward-auth set on the
# controller, it's directly callable.
connect_response=$(curl -sf -X POST \
    "${CONTROLLER_URL}/ctrl/connect" \
    -H "Content-Type: application/json" \
    -d "${connect_body}")

if [[ -z "${connect_response}" ]]; then
    fail "Empty response from /ctrl/connect"
fi

# Response shape (from common/src/types/mod.rs ConnectResponse):
#   { backend_id, spawned, status, token, url, subdomain?, secret_token?,
#     status_url, drone? }

backend_id=$(echo "${connect_response}" | jq -r '.backend_id // empty')
backend_url=$(echo "${connect_response}" | jq -r '.url // empty')
status_url=$(echo "${connect_response}" | jq -r '.status_url // empty')
spawned=$(echo "${connect_response}" | jq -r '.spawned // empty')
assigned_drone=$(echo "${connect_response}" | jq -r '.drone // empty')

if [[ -z "${backend_id}" || -z "${backend_url}" ]]; then
    echo "Raw response: ${connect_response}"
    fail "Missing backend_id or url in /ctrl/connect response"
fi

ok "Backend ${backend_id} (spawned=${spawned}, drone=${assigned_drone:-?})"
ok "URL:        ${backend_url}"
ok "Status URL: ${status_url}"

# ---- 4. Wait for backend ready -------------------------------------------
#
# BackendStatus enum includes: scheduled, loading, starting, waiting,
# ready, terminating, hard-terminating, terminated.

echo ""
echo "Waiting for backend to reach 'ready'..."

ready=false
final_status=""
for i in $(seq 1 60); do
    body=$(curl -sf "${CONTROLLER_URL}/pub/b/${backend_id}/status" 2>/dev/null || echo '{}')
    final_status=$(echo "${body}" | jq -r '.status // empty' 2>/dev/null || echo "")
    case "${final_status}" in
        ready)
            ok "Backend reached 'ready' (after ~${i}s)"
            ready=true
            break
            ;;
        terminating|hard-terminating|terminated)
            fail "Backend terminated during startup (status=${final_status})"
            ;;
    esac
    sleep 1
done

if ! $ready; then
    fail "Backend did not reach 'ready' in 60s (last status: ${final_status:-unknown})"
fi

# ---- 5. Route through proxy ----------------------------------------------

echo ""
echo "Verifying proxy routes to backend..."

if curl -sf -m 10 "${backend_url}" -o /tmp/smoke-body >/dev/null; then
    bytes=$(wc -c < /tmp/smoke-body)
    ok "Proxy returned ${bytes} bytes from backend"
else
    fail "Proxy did not route to backend (url=${backend_url})"
fi

# ---- 6. Soft-terminate ---------------------------------------------------

echo ""
echo "Terminating backend ${backend_id}..."
if curl -sf -X POST "${CONTROLLER_URL}/ctrl/b/${backend_id}/soft-terminate" >/dev/null; then
    ok "Soft-terminate accepted"
else
    fail "Soft-terminate failed"
fi

echo ""
echo -e "${GREEN}Smoke test PASSED.${NC}"
