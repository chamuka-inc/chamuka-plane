#!/usr/bin/env bash
# provision/20-runsc.sh
#
# Install gVisor (runsc) and register it as a Docker runtime.
# Only runs on drones tagged 'untrusted' tier.
#
# Verified install procedure from https://gvisor.dev/docs/user_guide/install/
#
# Critical detail: the gVisor release URLs use the kernel-arch name
# from `uname -m` (x86_64 / aarch64), NOT the Debian/Ubuntu arch name
# from `dpkg --print-architecture` (amd64 / arm64). Using the wrong
# variable would 404.

set -euo pipefail

log() { echo "[20-runsc] $*"; }

if command -v runsc >/dev/null 2>&1; then
    log "runsc already installed; skipping."
    runsc --version
    exit 0
fi

log "Installing gVisor via apt repository (recommended method)..."

# Use the apt repo so we get the signed package and automatic updates.
# Verified procedure from https://gvisor.dev/docs/user_guide/install/

export DEBIAN_FRONTEND=noninteractive

apt-get update -qq
apt-get install -y -qq apt-transport-https ca-certificates curl gnupg

# Add the gVisor signing key.
curl -fsSL https://gvisor.dev/archive.key \
    | gpg --dearmor -o /usr/share/keyrings/gvisor-archive-keyring.gpg

# Add the gVisor repository (uses Debian-style arch: amd64/arm64).
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/gvisor-archive-keyring.gpg] https://storage.googleapis.com/gvisor/releases release main" \
    > /etc/apt/sources.list.d/gvisor.list

apt-get update -qq
apt-get install -y -qq runsc

# ---- Register runsc as a Docker runtime ----------------------------------
#
# `runsc install` is the canonical command — it edits /etc/docker/daemon.json
# to add a "runtimes.runsc" entry pointing at the runsc binary. See
# https://gvisor.dev/docs/user_guide/install/ — "If you have Docker
# installed, it will be automatically configured."

log "Registering runsc with Docker via 'runsc install'..."
runsc install

systemctl restart docker

# ---- Verify --------------------------------------------------------------

log "Verifying runsc runtime..."
if docker run --rm --runtime=runsc hello-world >/dev/null 2>&1; then
    log "runsc runtime works."
    docker info --format '{{json .Runtimes}}' 2>/dev/null | head -c 200; echo
else
    log "ERROR: runsc smoke test failed."
    log "docker info:"
    docker info | grep -i runtime || true
    exit 1
fi

log "gVisor installation complete."
