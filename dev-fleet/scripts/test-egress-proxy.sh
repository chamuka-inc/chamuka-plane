#!/usr/bin/env bash
set -euo pipefail

# Egress proxy spike e2e test
# Verifies: credential registration → proxy header injection → sandbox-to-echo-server flow
#
# Prerequisites:
#   - Dev fleet running: cd dev-fleet && docker compose up -d
#   - Echo server and egress-proxy containers running

CONTROLLER="http://localhost:8080"
PROXY_ADMIN="http://localhost:3129"
ECHO_SERVER_HOST="host.docker.internal:8888"
PROXY_URL="http://host.docker.internal:3128"
SECRET_VALUE="sk-e2e-test-$(date +%s)"

fail() { echo "FAIL: $1" >&2; exit 1; }
pass() { echo "PASS: $1"; }

echo "=== Egress Proxy Spike E2E Test ==="
echo ""

# 1. Health checks
echo "--- Step 1: Health checks ---"
curl -sf "$PROXY_ADMIN/admin/health" > /dev/null || fail "egress-proxy admin not reachable"
curl -sf "http://localhost:8888/" > /dev/null || fail "echo-server not reachable"
pass "egress-proxy and echo-server are healthy"

# 2. Register credential binding
echo ""
echo "--- Step 2: Register credential for $ECHO_SERVER_HOST ---"
RESULT=$(curl -sf -X POST "$PROXY_ADMIN/admin/credentials" \
  -H "Content-Type: application/json" \
  -d "{
    \"destination\": \"$ECHO_SERVER_HOST\",
    \"header_name\": \"Authorization\",
    \"header_format\": \"Bearer {value}\",
    \"value\": \"$SECRET_VALUE\"
  }")

echo "$RESULT" | grep -q '"ok":true' || fail "credential registration failed: $RESULT"
pass "credential registered"

# 3. Verify injection from host
echo ""
echo "--- Step 3: Verify header injection (host-side) ---"
ECHO=$(curl -sf --proxy http://localhost:3128 "http://$ECHO_SERVER_HOST/")
if echo "$ECHO" | grep -q "$SECRET_VALUE"; then
  pass "host-side proxy injection works"
else
  fail "header not injected. Echo response: $ECHO"
fi

# 4. Spawn sandbox with HTTP_PROXY
echo ""
echo "--- Step 4: Spawn sandbox with HTTP_PROXY ---"
SPAWN=$(curl -sf "$CONTROLLER/pub/v1/sandboxes" -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Tenant: default" \
  -d "{
    \"runtime\": \"bash\",
    \"image\": \"localhost:5000/sandbox-runtime:node24-latest\",
    \"pool\": \"trusted\",
    \"resources\": { \"vcpus\": 1 },
    \"env\": {
      \"HTTP_PROXY\": \"$PROXY_URL\",
      \"http_proxy\": \"$PROXY_URL\"
    }
  }")

SANDBOX_ID=$(echo "$SPAWN" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
BACKEND_ID=$(echo "$SPAWN" | grep -o '"backend_id":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "  sandbox=$SANDBOX_ID backend=$BACKEND_ID"

# 5. Wait for sandbox to be ready
echo ""
echo "--- Step 5: Wait for sandbox ready ---"
for i in $(seq 1 20); do
  STATUS=$(curl -sf "$CONTROLLER/pub/v1/sandboxes/$SANDBOX_ID" \
    -H "X-Auth-Tenant: default" 2>/dev/null | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "unknown")
  if [ "$STATUS" = "ready" ]; then
    break
  fi
  if [ "$STATUS" = "terminated" ]; then
    fail "sandbox terminated before becoming ready"
  fi
  sleep 1
done
[ "$STATUS" = "ready" ] || fail "sandbox did not become ready (status=$STATUS)"
pass "sandbox ready"

CONTAINER="plane-$BACKEND_ID"

# 6. Verify env vars inside sandbox
echo ""
echo "--- Step 6: Verify HTTP_PROXY in sandbox env ---"
PROXY_ENV=$(docker exec "$CONTAINER" env 2>/dev/null | grep -i "HTTP_PROXY" || true)
echo "$PROXY_ENV" | grep -q "$PROXY_URL" || fail "HTTP_PROXY not set in sandbox"
pass "HTTP_PROXY is set"

# 7. Verify secret is NOT in sandbox env
echo ""
echo "--- Step 7: Verify secret NOT visible in sandbox env ---"
SECRET_LEAK=$(docker exec "$CONTAINER" env 2>/dev/null | grep "$SECRET_VALUE" || true)
if [ -n "$SECRET_LEAK" ]; then
  fail "secret value leaked to sandbox environment!"
fi
pass "secret value not visible in sandbox"

# 8. Test injection from inside sandbox
echo ""
echo "--- Step 8: Curl echo server from inside sandbox (through proxy) ---"
SANDBOX_ECHO=$(docker exec "$CONTAINER" curl -sf "http://$ECHO_SERVER_HOST/" 2>/dev/null)
if echo "$SANDBOX_ECHO" | grep -q "$SECRET_VALUE"; then
  pass "HEADER INJECTED! Sandbox received proxied response with secret"
else
  fail "header not injected from sandbox. Response: $SANDBOX_ECHO"
fi

# 9. Verify no injection when bypassing proxy
echo ""
echo "--- Step 9: Verify no injection when bypassing proxy ---"
DIRECT_ECHO=$(docker exec "$CONTAINER" curl -sf --noproxy '*' "http://$ECHO_SERVER_HOST/" 2>/dev/null)
if echo "$DIRECT_ECHO" | grep -q "$SECRET_VALUE"; then
  fail "secret appeared without proxy — something is wrong"
fi
pass "no injection when proxy bypassed (as expected)"

# 10. Cleanup: delete credential
echo ""
echo "--- Step 10: Cleanup ---"
ENCODED_HOST=$(echo "$ECHO_SERVER_HOST" | sed 's/:/%3A/g')
curl -sf -X DELETE "$PROXY_ADMIN/admin/credentials/$ENCODED_HOST" > /dev/null
pass "credential cleaned up"

echo ""
echo "=== ALL TESTS PASSED ==="
echo ""
echo "Summary:"
echo "  - Egress proxy injects Authorization header based on destination"
echo "  - Sandbox never sees the secret value in its environment"
echo "  - Header only appears when traffic goes through the proxy"
echo "  - Bypassing the proxy produces no injection (control test)"
