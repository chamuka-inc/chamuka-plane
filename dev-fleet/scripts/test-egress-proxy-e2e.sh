#!/usr/bin/env bash
set -euo pipefail

# Full production e2e: console BFF → controller → drone → egress proxy → echo server
#
# Tests the complete credential binding pipeline:
#   1. Spawn sandbox with HTTP_PROXY env var
#   2. Create secret via controller API
#   3. Bind credential to sandbox via controller API
#   4. Controller dispatches PushCredential → drone resolves secret value → pushes to egress proxy
#   5. Sandbox curls echo server through proxy → proxy injects header
#   6. Verify echo server received the injected header
#
# Prerequisites:
#   - Dev fleet running with rebuilt Plane image (make rebuild && docker compose up -d)
#   - Egress proxy and echo server containers running

CONTROLLER="http://localhost:8080"
PROXY_ADMIN="http://localhost:3129"
ECHO_SERVER_HOST="host.docker.internal:8888"
PROXY_URL="http://host.docker.internal:3128"
SECRET_VALUE="sk-prod-e2e-$(date +%s)"
TENANT="default"

fail() { echo "FAIL: $1" >&2; exit 1; }
pass() { echo "PASS: $1"; }

echo "=== Full Production E2E Test ==="
echo ""

# 1. Health checks
echo "--- Step 1: Health checks ---"
curl -sf "$PROXY_ADMIN/admin/health" > /dev/null || fail "egress-proxy admin not reachable"
curl -sf "http://localhost:8888/" > /dev/null || fail "echo-server not reachable"
curl -sf "$CONTROLLER/pub/v1/sandboxes" -H "X-Auth-Tenant: $TENANT" > /dev/null || fail "controller not reachable"
pass "all services healthy"

# 2. Spawn sandbox with HTTP_PROXY env var
echo ""
echo "--- Step 2: Spawn sandbox with HTTP_PROXY ---"
SPAWN=$(curl -sf "$CONTROLLER/pub/v1/sandboxes" -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Tenant: $TENANT" \
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

# 3. Wait for sandbox to be ready
echo ""
echo "--- Step 3: Wait for sandbox ready ---"
for i in $(seq 1 20); do
  STATUS=$(curl -sf "$CONTROLLER/pub/v1/sandboxes/$SANDBOX_ID" \
    -H "X-Auth-Tenant: $TENANT" 2>/dev/null | grep -o '"status":"[^"]*"' | head -1 | cut -d'"' -f4 || echo "unknown")
  if [ "$STATUS" = "ready" ]; then break; fi
  if [ "$STATUS" = "terminated" ]; then fail "sandbox terminated before becoming ready"; fi
  sleep 1
done
[ "$STATUS" = "ready" ] || fail "sandbox did not become ready (status=$STATUS)"
pass "sandbox ready"

# 4. Create secret via controller API
echo ""
echo "--- Step 4: Create secret ---"
SECRET_RESP=$(curl -sf "$CONTROLLER/pub/v1/secrets" -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Tenant: $TENANT" \
  -d "{
    \"value\": \"$SECRET_VALUE\",
    \"name\": \"test-openai-key\"
  }")

SECRET_ID=$(echo "$SECRET_RESP" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "  secret_id=$SECRET_ID"
[ -n "$SECRET_ID" ] || fail "failed to create secret"
pass "secret created"

# 5. Bind credential to sandbox
echo ""
echo "--- Step 5: Bind credential to sandbox ---"
CRED_RESP=$(curl -sf "$CONTROLLER/pub/v1/sandboxes/$SANDBOX_ID/credentials" -X POST \
  -H "Content-Type: application/json" \
  -H "X-Auth-Tenant: $TENANT" \
  -d "{
    \"secret_id\": \"$SECRET_ID\",
    \"destination\": \"$ECHO_SERVER_HOST\",
    \"header_name\": \"Authorization\",
    \"header_format\": \"Bearer {value}\"
  }")

CRED_ID=$(echo "$CRED_RESP" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)
echo "  credential_id=$CRED_ID"
[ -n "$CRED_ID" ] || fail "failed to bind credential"
pass "credential bound"

# 6. Wait for credential to propagate through the pipeline
echo ""
echo "--- Step 6: Wait for credential propagation (controller → drone → proxy) ---"
sleep 3

# Check if proxy received the credential
PROXY_CREDS=$(curl -sf "$PROXY_ADMIN/admin/credentials" 2>/dev/null)
if echo "$PROXY_CREDS" | grep -q "$ECHO_SERVER_HOST"; then
  pass "credential propagated to egress proxy"
else
  echo "  WARNING: credential not yet in proxy (may need more time or drone reconnect)"
  echo "  Proxy state: $PROXY_CREDS"
  sleep 5
  PROXY_CREDS=$(curl -sf "$PROXY_ADMIN/admin/credentials" 2>/dev/null)
  if echo "$PROXY_CREDS" | grep -q "$ECHO_SERVER_HOST"; then
    pass "credential propagated to egress proxy (delayed)"
  else
    fail "credential did not propagate to egress proxy. Proxy state: $PROXY_CREDS"
  fi
fi

# 7. Verify header injection from inside sandbox
echo ""
echo "--- Step 7: Curl echo server from inside sandbox (through proxy) ---"
CONTAINER="plane-$BACKEND_ID"
SANDBOX_ECHO=$(docker exec "$CONTAINER" curl -sf "http://$ECHO_SERVER_HOST/" 2>/dev/null)
if echo "$SANDBOX_ECHO" | grep -q "$SECRET_VALUE"; then
  pass "HEADER INJECTED via full production pipeline!"
  echo "  Echo response shows Authorization header with secret value"
else
  echo "  Response: $SANDBOX_ECHO"
  fail "header not injected from sandbox through production pipeline"
fi

# 8. Verify secret NOT in sandbox env
echo ""
echo "--- Step 8: Verify secret NOT visible in sandbox ---"
SECRET_LEAK=$(docker exec "$CONTAINER" env 2>/dev/null | grep "$SECRET_VALUE" || true)
if [ -n "$SECRET_LEAK" ]; then
  fail "secret value leaked to sandbox environment!"
fi
pass "secret value not visible in sandbox"

# 9. Control test: bypass proxy
echo ""
echo "--- Step 9: Verify no injection when bypassing proxy ---"
DIRECT_ECHO=$(docker exec "$CONTAINER" curl -sf --noproxy '*' "http://$ECHO_SERVER_HOST/" 2>/dev/null)
if echo "$DIRECT_ECHO" | grep -q "$SECRET_VALUE"; then
  fail "secret appeared without proxy — something is wrong"
fi
pass "no injection when proxy bypassed"

echo ""
echo "=== ALL TESTS PASSED ==="
echo ""
echo "Full production pipeline verified:"
echo "  Console API → Controller (secret + credential binding)"
echo "  → Controller dispatch → Drone (PushCredential with resolved secret)"
echo "  → Drone EgressPublisher → Egress Proxy (credential registered)"
echo "  → Sandbox curl → Proxy injects Authorization header → Echo Server"
echo "  → Sandbox NEVER sees the secret value"
