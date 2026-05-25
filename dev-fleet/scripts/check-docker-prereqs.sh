#!/usr/bin/env bash
# scripts/check-docker-prereqs.sh
#
# Verify the host can start the Docker-only fallback fleet without
# conflicting with the canonical Vagrant fleet.

set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

ok()   { echo -e "  ${GREEN}✓${NC} $*"; }
fail() { echo -e "  ${RED}✗${NC} $*"; FAILED=1; }
warn() { echo -e "  ${YELLOW}!${NC} $*"; }

FAILED=0
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FLEET_DIR="$(dirname "$SCRIPT_DIR")"

echo "Checking Docker fallback prerequisites..."

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    ok "Docker daemon is running"
else
    fail "Docker is not running. Start Docker and retry."
fi

if docker compose version >/dev/null 2>&1; then
    ok "Docker Compose installed"
else
    fail "Docker Compose v2 plugin not found."
fi

if docker compose -f "${FLEET_DIR}/host/docker-compose.yml" ps --status running -q 2>/dev/null | grep -q .; then
    fail "Canonical Vagrant host services are running."
    warn "Stop them first: make fleet-nuke (from repo root)"
fi

if command -v vagrant >/dev/null 2>&1; then
    if vagrant status 2>/dev/null | grep -q "running"; then
        fail "Vagrant drone VMs are running."
        warn "Stop them first: make fleet-down (from repo root)"
    else
        ok "No Vagrant drones running"
    fi
fi

if docker compose -f "${FLEET_DIR}/docker-compose.yml" ps --status running -q 2>/dev/null | grep -q .; then
    ok "Docker fallback fleet already running"
else
    for port in 8080 9090 5432 5000; do
        if lsof -i ":${port}" -sTCP:LISTEN -P -n >/dev/null 2>&1; then
            owner=$(lsof -i ":${port}" -sTCP:LISTEN -P -n 2>/dev/null | awk 'NR==2{print $1}')
            fail "Port ${port} is already in use (by ${owner:-unknown})."
            warn "If the canonical fleet is active, run: make fleet-nuke"
            warn "If another process holds the port, stop it or override ports in .env."
        else
            ok "Port ${port} is available"
        fi
    done
fi

if docker image inspect chamuka-plane:latest >/dev/null 2>&1; then
    ok "chamuka-plane:latest image exists"
else
    fail "chamuka-plane:latest not found. Run: make fleet-images (from repo root)"
fi

if docker image inspect sandbox-runtime:node24-latest >/dev/null 2>&1; then
    ok "sandbox-runtime:node24-latest image exists"
else
    fail "sandbox-runtime:node24-latest not found. Run: make fleet-images (from repo root)"
fi

echo ""
if [[ ${FAILED} -eq 1 ]]; then
    echo -e "${RED}Docker fallback prerequisite check failed.${NC}"
    exit 1
else
    echo -e "${GREEN}Docker fallback prerequisites satisfied.${NC}"
fi
