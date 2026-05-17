#!/usr/bin/env bash
# provision/30-kata.sh
#
# Install Kata Containers with Firecracker backend on a KVM-capable host.
#
# Prerequisites:
#   - Linux host with KVM enabled (check: ls /dev/kvm)
#   - Docker installed
#   - Run AFTER 00-common.sh
#
# Verified against:
#   - https://github.com/kata-containers/kata-containers/blob/main/docs/install/container-manager/containerd/containerd-install.md
#   - https://github.com/kata-containers/kata-containers/releases

set -euo pipefail

KATA_VERSION="${KATA_VERSION:-3.13.0}"
DRONE_NUMBER="${DRONE_NUMBER:-unknown}"

log() { echo "[drone-${DRONE_NUMBER} 30-kata] $*"; }

log "Installing Kata Containers ${KATA_VERSION}..."

if [[ ! -e /dev/kvm ]]; then
    log "ERROR: /dev/kvm not found. Kata requires hardware virtualization."
    log "  On EC2: use c8i/m8i/r8i with NestedVirtualization=enabled, or *.metal"
    log "  On bare metal: ensure KVM modules are loaded (kvm, kvm_intel or kvm_amd)"
    exit 1
fi

ARCH=$(uname -m)
case "${ARCH}" in
    x86_64)  KATA_ARCH="amd64" ;;
    aarch64) KATA_ARCH="arm64" ;;
    *)
        log "ERROR: unsupported architecture ${ARCH}"
        exit 1
        ;;
esac

TARBALL="kata-static-${KATA_VERSION}-${KATA_ARCH}.tar.xz"
DOWNLOAD_URL="https://github.com/kata-containers/kata-containers/releases/download/${KATA_VERSION}/${TARBALL}"

log "Downloading ${TARBALL}..."
curl -fsSL -o "/tmp/${TARBALL}" "${DOWNLOAD_URL}"

log "Extracting to /opt/kata..."
mkdir -p /opt/kata
tar -xJf "/tmp/${TARBALL}" -C /
rm -f "/tmp/${TARBALL}"

ln -sf /opt/kata/bin/kata-runtime /usr/local/bin/kata-runtime
ln -sf /opt/kata/bin/containerd-shim-kata-v2 /usr/local/bin/containerd-shim-kata-v2

log "Configuring Kata for Firecracker VMM..."
mkdir -p /opt/kata/share/defaults/kata-containers
cat > /opt/kata/share/defaults/kata-containers/configuration-fc.toml <<'KATA_CONF'
[hypervisor.firecracker]
path = "/opt/kata/bin/firecracker"
kernel = "/opt/kata/share/kata-containers/vmlinux.container"
image = "/opt/kata/share/kata-containers/kata-containers.img"
firmware = ""
default_vcpus = 1
default_memory = 256
enable_annotations = ["default_vcpus", "default_memory"]
valid_hypervisor_paths = ["/opt/kata/bin/firecracker"]

[agent.kata]
enable_tracing = false

[runtime]
enable_debug = false
internetworking_model = "tcfilter"
sandbox_cgroup_only = true
KATA_CONF

ln -sf /opt/kata/share/defaults/kata-containers/configuration-fc.toml \
    /opt/kata/share/defaults/kata-containers/configuration.toml

log "Registering kata-runtime with Docker..."
mkdir -p /etc/docker
DOCKER_DAEMON_JSON="/etc/docker/daemon.json"

if [[ -f "${DOCKER_DAEMON_JSON}" ]]; then
    EXISTING=$(cat "${DOCKER_DAEMON_JSON}")
else
    EXISTING='{}'
fi

echo "${EXISTING}" | python3 -c "
import json, sys
cfg = json.load(sys.stdin)
cfg.setdefault('runtimes', {})
cfg['runtimes']['kata-runtime'] = {'path': '/usr/local/bin/kata-runtime'}
json.dump(cfg, sys.stdout, indent=2)
" > "${DOCKER_DAEMON_JSON}.tmp"
mv "${DOCKER_DAEMON_JSON}.tmp" "${DOCKER_DAEMON_JSON}"

systemctl restart docker

log "Verifying kata-runtime..."
if kata-runtime check --no-network-checks 2>/dev/null; then
    log "Kata runtime check passed."
else
    log "WARN: kata-runtime check reported issues (may still work for basic containers)."
    kata-runtime check --no-network-checks 2>&1 || true
fi

log "Kata Containers ${KATA_VERSION} installed."
