#!/usr/bin/env bash
# provision/00-common.sh
#
# Common setup for every drone VM, regardless of trust tier.
# Run as root by Vagrant's shell provisioner.

set -euo pipefail

DRONE_NUMBER="${DRONE_NUMBER:-unknown}"
CONTROLLER_HOST="${CONTROLLER_HOST:-192.168.56.1}"
CONTROLLER_PORT="${CONTROLLER_PORT:-8080}"
TRUST_TIER="${TRUST_TIER:-trusted}"
REGISTRY_MIRROR="${REGISTRY_MIRROR:-http://${CONTROLLER_HOST}:5000}"

log() { echo "[drone-${DRONE_NUMBER} 00-common] $*"; }

log "Beginning common provisioning. Trust tier: ${TRUST_TIER}"

# ---- System hygiene -------------------------------------------------------

export DEBIAN_FRONTEND=noninteractive

# Avoid surprise prompts and grub interactions.
apt-get update -qq
apt-get install -y -qq \
    ca-certificates \
    curl \
    gnupg \
    jq \
    net-tools \
    iproute2 \
    iptables \
    bridge-utils

# ---- Docker ---------------------------------------------------------------

if ! command -v docker >/dev/null 2>&1; then
    log "Installing Docker..."

    install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg \
        -o /etc/apt/keyrings/docker.asc
    chmod a+r /etc/apt/keyrings/docker.asc

    # shellcheck disable=SC1091
    . /etc/os-release
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] \
https://download.docker.com/linux/ubuntu $VERSION_CODENAME stable" \
        > /etc/apt/sources.list.d/docker.list

    apt-get update -qq
    apt-get install -y -qq \
        docker-ce \
        docker-ce-cli \
        containerd.io \
        docker-buildx-plugin \
        docker-compose-plugin
fi

# ---- Docker daemon config -------------------------------------------------

log "Configuring Docker daemon (registry mirror, log limits, cgroup driver)..."

mkdir -p /etc/docker
cat > /etc/docker/daemon.json <<EOF
{
  "registry-mirrors": ["${REGISTRY_MIRROR}"],
  "insecure-registries": ["${CONTROLLER_HOST}:5000"],
  "log-driver": "json-file",
  "log-opts": {
    "max-size": "10m",
    "max-file": "3"
  },
  "exec-opts": ["native.cgroupdriver=systemd"],
  "live-restore": true,
  "features": {
    "buildkit": true
  }
}
EOF

systemctl enable --now docker
systemctl restart docker

# ---- Drone identity -------------------------------------------------------

mkdir -p /etc/sandbox-platform
cat > /etc/sandbox-platform/drone.env <<EOF
DRONE_NUMBER=${DRONE_NUMBER}
TRUST_TIER=${TRUST_TIER}
CONTROLLER_HOST=${CONTROLLER_HOST}
CONTROLLER_PORT=${CONTROLLER_PORT}
REGISTRY_MIRROR=${REGISTRY_MIRROR}
EOF

# ---- Resource limits ------------------------------------------------------

# Allow many open files and processes; container workloads expect this.
cat > /etc/security/limits.d/99-sandbox.conf <<EOF
*       soft    nofile  1048576
*       hard    nofile  1048576
*       soft    nproc   unlimited
*       hard    nproc   unlimited
EOF

# Enable IP forwarding for container networking.
cat > /etc/sysctl.d/99-sandbox.conf <<EOF
net.ipv4.ip_forward = 1
net.ipv6.conf.all.forwarding = 1
net.bridge.bridge-nf-call-iptables = 1
net.bridge.bridge-nf-call-ip6tables = 1
fs.inotify.max_user_watches = 524288
fs.inotify.max_user_instances = 512
EOF
modprobe br_netfilter || true
sysctl --system >/dev/null

# ---- Wait for controller to be reachable ---------------------------------

log "Waiting for controller at ${CONTROLLER_HOST}:${CONTROLLER_PORT}..."
for i in $(seq 1 60); do
    if curl -sf "http://${CONTROLLER_HOST}:${CONTROLLER_PORT}/ctrl/status" >/dev/null 2>&1; then
        log "Controller reachable."
        break
    fi
    if [[ $i -eq 60 ]]; then
        log "WARN: controller did not become reachable in 60s; drone will retry on its own."
    fi
    sleep 1
done

log "Common provisioning complete."
