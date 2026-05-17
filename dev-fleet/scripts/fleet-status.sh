#!/usr/bin/env bash
# fleet-status.sh — show current state of the dev fleet.
#
# Usage:
#   ./dev-fleet/scripts/fleet-status.sh
#   CONTROLLER=localhost:8080 ./dev-fleet/scripts/fleet-status.sh

set -euo pipefail

CONTROLLER="${CONTROLLER:-localhost:8080}"
CLUSTER="${CLUSTER:-localhost:9090}"
ENCODED_CLUSTER=$(printf '%s' "$CLUSTER" | sed 's/:/%3A/g')

# ─── Docker containers ───────────────────────────────────────

echo "=== Fleet containers ==="
if ! docker compose -f "$(dirname "$0")/../docker-compose.yml" ps --format '  {{.Name}}  {{.Status}}' 2>/dev/null; then
    echo "  (compose not running or not found)"
fi

# ─── Controller health ────────────────────────────────────────

echo ""
echo "=== Controller ($CONTROLLER) ==="
if curl -sf "http://$CONTROLLER/pub/health" >/dev/null 2>&1; then
    echo "  health: ok"
else
    echo "  health: UNREACHABLE"
    exit 1
fi

# ─── Cluster state ────────────────────────────────────────────

echo ""
echo "=== Cluster ($CLUSTER) ==="
cluster_state=$(curl -sf "http://$CONTROLLER/ctrl/c/$ENCODED_CLUSTER/state" 2>/dev/null || echo "")
if [ -z "$cluster_state" ]; then
    echo "  (no cluster state available)"
    exit 0
fi

CLUSTER_STATE="$cluster_state" python3 -c '
import json, os

data = json.loads(os.environ["CLUSTER_STATE"])

drones = data.get("drones", [])
proxies = data.get("proxies", [])

print(f"  drones:  {len(drones)}")
print(f"  proxies: {len(proxies)}")
print()

if drones:
    print("  --- Drones ---")
    for d in drones:
        name = d.get("node", {}).get("name", {}).get("Drone", "?")
        ready = d.get("ready", False)
        draining = d.get("draining", False)
        backends = d.get("backend_count", 0)
        hb = d.get("last_heartbeat_age", "?")
        version = d["node"].get("plane_version", "?")
        status = "ready" if ready else ("draining" if draining else "not-ready")
        print(f"  {name:24s}  {status:10s}  backends={backends}  heartbeat={hb}s  v{version}")
    print()

if proxies:
    print("  --- Proxies ---")
    for p in proxies:
        name = p.get("name", {}).get("Proxy", "?")
        version = p.get("plane_version", "?")
        hb = p.get("controller_heartbeat_age", "?")
        print(f"  {name:24s}  heartbeat={hb}s  v{version}")
    print()
'

# ─── Active backends ──────────────────────────────────────────

echo "=== Active backends ==="
backend_containers=$(docker ps --filter "name=plane-" --format '{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}' 2>/dev/null || echo "")
if [ -n "$backend_containers" ]; then
    echo "$backend_containers" | while IFS=$'\t' read -r name image status ports; do
        backend_id="${name#plane-}"
        port_map=""
        if [ -n "$ports" ]; then
            port_map=$(echo "$ports" | grep -o ':[0-9]*->8080' | head -1 | tr -d ':->' | sed 's/8080//')
            [ -n "$port_map" ] && port_map=" port=$port_map"
        fi
        echo "  $backend_id  ($image)  $status$port_map"
    done
else
    echo "  (none)"
fi
