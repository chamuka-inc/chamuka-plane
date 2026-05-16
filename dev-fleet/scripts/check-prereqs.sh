#!/usr/bin/env bash
# scripts/check-prereqs.sh
#
# Verify the host has everything needed to bring up the local fleet.

set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

ok()   { echo -e "  ${GREEN}✓${NC} $*"; }
fail() { echo -e "  ${RED}✗${NC} $*"; FAILED=1; }
warn() { echo -e "  ${YELLOW}!${NC} $*"; }

FAILED=0

echo "Checking host prerequisites..."

# ---- VirtualBox ----------------------------------------------------------

if command -v VBoxManage >/dev/null 2>&1; then
    vbox_version=$(VBoxManage --version | awk -F'r' '{print $1}')
    ok "VirtualBox installed (version ${vbox_version})"
    # 7.0+ is required for nested virt support.
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
    if VBoxManage list hostonlyifs | grep -q "192.168.56"; then
        ok "Host-only network 192.168.56.0/24 exists"
    else
        warn "Host-only network 192.168.56.0/24 does not exist. Vagrant will create it on 'vagrant up'."
        warn "If creation fails, manually run: VBoxManage hostonlyif create"
    fi
fi

# ---- Port availability ---------------------------------------------------

for port in 8080 9090 5432 5000; do
    if ss -ltn 2>/dev/null | awk '{print $4}' | grep -qE ":${port}\$"; then
        fail "Port ${port} is already in use on the host. Free it or override in .env."
    else
        ok "Port ${port} is available"
    fi
done

# ---- IP forwarding -------------------------------------------------------

if [[ "$(cat /proc/sys/net/ipv4/ip_forward 2>/dev/null || echo 0)" == "1" ]]; then
    ok "IPv4 forwarding is enabled"
else
    warn "IPv4 forwarding is disabled (not strictly required, but useful for cross-VM testing)"
fi

# ---- Result --------------------------------------------------------------

echo ""
if [[ ${FAILED} -eq 1 ]]; then
    echo -e "${RED}Prerequisite check failed.${NC} Fix the issues above and retry."
    exit 1
else
    echo -e "${GREEN}All prerequisites satisfied.${NC}"
fi
