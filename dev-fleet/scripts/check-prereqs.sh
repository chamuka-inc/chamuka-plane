#!/usr/bin/env bash
# scripts/check-prereqs.sh
#
# Verify the host has everything needed to bring up the canonical
# Vagrant-backed local fleet.

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

echo "Checking canonical Vagrant fleet prerequisites..."

# ---- VirtualBox ----------------------------------------------------------

if command -v VBoxManage >/dev/null 2>&1; then
    vbox_version=$(VBoxManage --version | awk -F'r' '{print $1}')
    ok "VirtualBox installed (version ${vbox_version})"
    major=$(echo "$vbox_version" | cut -d. -f1)
    if [[ "${major}" -lt 7 ]]; then
        warn "VirtualBox 7.0+ is recommended for nested virtualization support."
    fi
else
    fail "VirtualBox is not installed. See https://www.virtualbox.org/wiki/Downloads"
fi

# ---- Vagrant -------------------------------------------------------------

if command -v vagrant >/dev/null 2>&1; then
    vagrant_version=$(vagrant --version | awk '{print $2}')
    ok "Vagrant installed (version ${vagrant_version})"
else
    fail "Vagrant is not installed. See https://developer.hashicorp.com/vagrant/install"
fi

# ---- Docker --------------------------------------------------------------

if command -v docker >/dev/null 2>&1; then
    docker_version=$(docker --version | awk '{print $3}' | tr -d ',')
    ok "Docker installed (version ${docker_version})"
    if docker info >/dev/null 2>&1; then
        ok "Docker daemon is running"
    else
        fail "Docker is installed but the daemon is not running."
    fi
else
    fail "Docker is not installed. See https://docs.docker.com/get-docker/"
fi

# Docker Compose v2
if docker compose version >/dev/null 2>&1; then
    compose_version=$(docker compose version --short)
    ok "Docker Compose installed (version ${compose_version})"
else
    fail "Docker Compose v2 plugin not found. Install via 'docker-compose-plugin'."
fi

# ---- jq ------------------------------------------------------------------

if command -v jq >/dev/null 2>&1; then
    ok "jq installed"
else
    warn "jq not installed (smoke tests will still work, output less readable)"
fi

# ---- Host-only network ---------------------------------------------------

if command -v VBoxManage >/dev/null 2>&1; then
    vbox_major=$(VBoxManage --version | cut -d. -f1)
    if [[ "${vbox_major}" -ge 7 ]]; then
        if VBoxManage list hostonlynets 2>/dev/null | grep -q "vboxnet0"; then
            ok "Host-only network vboxnet0 defined (VBox 7.x)"
        else
            warn "Host-only network vboxnet0 not defined. 'make ensure-network' will create it."
        fi
    else
        if VBoxManage list hostonlyifs 2>/dev/null | grep -q "192.168.56"; then
            ok "Host-only network 192.168.56.0/24 exists"
        else
            warn "Host-only network 192.168.56.0/24 does not exist. Vagrant will create it."
        fi
    fi
fi

# ---- Conflicting fleet containers ----------------------------------------

if docker compose -f "${FLEET_DIR}/docker-compose.yml" ps --status running -q 2>/dev/null | grep -q .; then
    fail "The Docker-only fallback fleet has running containers."
    warn "Stop it first: make fleet-down-docker (from repo root)"
fi

if docker compose -f "${FLEET_DIR}/host/docker-compose.yml" ps --status running -q 2>/dev/null | grep -q .; then
    ok "Canonical Vagrant host services already running"
    HOST_SERVICES_UP=1
else
    HOST_SERVICES_UP=0
fi

# ---- Port availability ---------------------------------------------------

if [[ "${HOST_SERVICES_UP}" -eq 0 ]]; then
    for port in 8080 9090 5432 5000; do
        if lsof -i ":${port}" -sTCP:LISTEN -P -n >/dev/null 2>&1; then
            owner=$(lsof -i ":${port}" -sTCP:LISTEN -P -n 2>/dev/null | awk 'NR==2{print $1}')
            fail "Port ${port} is already in use (by ${owner:-unknown})."
            warn "If the Docker fallback fleet is active, run: make fleet-down-docker"
            warn "If another process holds the port, stop it or override ports in .env."
        else
            ok "Port ${port} is available"
        fi
    done
else
    ok "Skipping port checks (host services already running)"
fi

# ---- Required images -----------------------------------------------------

if docker image inspect chamuka-plane:latest >/dev/null 2>&1; then
    ok "chamuka-plane:latest image exists"
    if docker run --rm --entrypoint /bin/wget chamuka-plane:latest --help >/dev/null 2>&1; then
        ok "chamuka-plane:latest has /bin/wget (health check will work)"
    else
        fail "chamuka-plane:latest lacks /bin/wget. Rebuild with: make image-plane (from repo root)"
    fi
else
    fail "chamuka-plane:latest image not found. Build it first: make fleet-images (from repo root)"
fi

if docker image inspect sandbox-runtime:node24-latest >/dev/null 2>&1; then
    ok "sandbox-runtime:node24-latest image exists"
else
    fail "sandbox-runtime:node24-latest not found. Run: make fleet-images (from repo root)"
fi

# ---- IP forwarding -------------------------------------------------------

if [[ "$(uname)" == "Darwin" ]]; then
    if sysctl -n net.inet.ip.forwarding 2>/dev/null | grep -q "1"; then
        ok "IPv4 forwarding is enabled"
    else
        warn "IPv4 forwarding is disabled (not strictly required)"
    fi
else
    if [[ "$(cat /proc/sys/net/ipv4/ip_forward 2>/dev/null || echo 0)" == "1" ]]; then
        ok "IPv4 forwarding is enabled"
    else
        warn "IPv4 forwarding is disabled (not strictly required)"
    fi
fi

# ---- Result --------------------------------------------------------------

echo ""
if [[ ${FAILED} -eq 1 ]]; then
    echo -e "${RED}Prerequisite check failed.${NC} Fix the issues above and retry."
    exit 1
else
    echo -e "${GREEN}All prerequisites satisfied.${NC}"
fi
