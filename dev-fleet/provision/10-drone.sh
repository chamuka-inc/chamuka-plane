#!/usr/bin/env bash
# provision/10-drone.sh
#
# Install and run the Plane drone in this VM as a systemd-managed
# Docker container.
#
# CLI flags verified against plane/src/drone/command.rs at
# github.com/jamsocket/plane:
#   --name <name>                  drone identifier (auto-generated if omitted)
#   --controller-url <ws url>      REQUIRED, WebSocket scheme
#   --cluster <name>               REQUIRED
#   --ip <ip-or-hostname>          IP that proxies use to reach this drone
#                                  (default 127.0.0.1)
#   --pool <name>                  drone pool tag; spawn requests with a
#                                  matching `pool` go to this drone
#   --docker-runtime <name>        Docker runtime for spawned backends
#                                  (e.g. runsc, runc); used for gVisor

set -euo pipefail

DRONE_NUMBER="${DRONE_NUMBER:-unknown}"
CONTROLLER_HOST="${CONTROLLER_HOST:-192.168.56.1}"
CONTROLLER_PORT="${CONTROLLER_PORT:-8080}"
CLUSTER_NAME="${CLUSTER_NAME:-192.168.56.1:9090}"
TRUST_TIER="${TRUST_TIER:-trusted}"
PLANE_IMAGE="${PLANE_IMAGE:-${CONTROLLER_HOST}:5000/chamuka-plane:latest}"

log() { echo "[drone-${DRONE_NUMBER} 10-drone] $*"; }

log "Installing Plane drone (cluster=${CLUSTER_NAME}, pool=${TRUST_TIER})"

# ---- Determine the VM's host-only network IP -----------------------------
#
# The drone advertises an IP to the controller via --ip. The proxy uses
# this IP to reach sandbox containers spawned on this drone, so it must
# be reachable from the host running the proxy.
#
# In our Vagrant setup, the second NIC (enp0s8 on bento/ubuntu-24.04) is
# attached to the host-only network at 192.168.56.0/24.

HOST_ONLY_IP=$(ip -4 -o addr show \
    | awk '/192\.168\.56\./ {print $4}' \
    | cut -d/ -f1 \
    | head -n1)

if [[ -z "${HOST_ONLY_IP}" ]]; then
    log "ERROR: could not determine host-only network IP."
    log "Available interfaces:"
    ip -4 -o addr show >&2
    exit 1
fi

log "Host-only IP: ${HOST_ONLY_IP}"

# ---- Pre-pull the Plane image -------------------------------------------

docker pull "${PLANE_IMAGE}"

# ---- Pick the Docker runtime --------------------------------------------
#
# Trusted tier:    runc  (Docker default)
# Untrusted tier:  runsc (gVisor; installed by 20-runsc.sh)

case "${TRUST_TIER}" in
    trusted)
        DOCKER_RUNTIME=""
        ;;
    untrusted)
        DOCKER_RUNTIME="runsc"
        ;;
    paranoid)
        if [[ ! -e /dev/kvm ]]; then
            log "ERROR: 'paranoid' tier requires KVM (/dev/kvm not found)."
            log "  Use a bare-metal host or EC2 c8i/m8i/r8i with nested virt."
            exit 1
        fi
        DOCKER_RUNTIME="kata-runtime"
        ;;
    *)
        log "ERROR: unknown trust tier '${TRUST_TIER}' (valid: trusted, untrusted, paranoid)"
        exit 1
        ;;
esac

# ---- systemd unit -------------------------------------------------------

cat > /etc/systemd/system/plane-drone.service <<EOF
[Unit]
Description=Plane drone for sandbox platform
Documentation=https://plane.dev
After=docker.service network-online.target
Requires=docker.service
Wants=network-online.target

[Service]
Type=simple
ExecStartPre=-/usr/bin/docker rm -f plane-drone
ExecStart=/usr/bin/docker run \\
    --rm \\
    --name plane-drone \\
    --label sandbox-platform.role=drone \\
    --label sandbox-platform.pool=${TRUST_TIER} \\
    --label sandbox-platform.drone-number=${DRONE_NUMBER} \\
    --add-host=host.docker.internal:host-gateway \\
    -v /var/run/docker.sock:/var/run/docker.sock \\
    ${PLANE_IMAGE} \\
    drone \\
    --name drone-${DRONE_NUMBER} \\
    --controller-url ws://${CONTROLLER_HOST}:${CONTROLLER_PORT} \\
    --cluster '${CLUSTER_NAME}' \\
    --ip ${HOST_ONLY_IP} \\
    --pool ${TRUST_TIER}${DOCKER_RUNTIME:+ \\
    --docker-runtime ${DOCKER_RUNTIME}}
ExecStop=/usr/bin/docker stop plane-drone
Restart=always
RestartSec=5

StandardOutput=journal
StandardError=journal
SyslogIdentifier=plane-drone

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable plane-drone.service
systemctl restart plane-drone.service

# ---- Verify drone registered with the controller ------------------------
#
# We look up the cluster state via /ctrl/c/{cluster}/state and check
# the `drones` array for our drone name. Verified route from
# plane/src/controller/mod.rs and common/src/lib.rs (cluster_state).

# URL-encode the cluster name (it contains a colon).
encoded_cluster=$(printf '%s' "${CLUSTER_NAME}" | sed 's/:/%3A/g')

log "Waiting for drone to register with controller..."
registered=false
for i in $(seq 1 30); do
    state=$(curl -sf \
        "http://${CONTROLLER_HOST}:${CONTROLLER_PORT}/ctrl/c/${encoded_cluster}/state" \
        2>/dev/null || echo '')
    if echo "${state}" | grep -q "drone-${DRONE_NUMBER}"; then
        log "Drone registered."
        registered=true
        break
    fi
    sleep 2
done

if ! $registered; then
    log "WARN: drone did not appear in cluster state within 60s."
    log "Check logs with: journalctl -u plane-drone -n 100 --no-pager"
fi

log "Drone provisioning complete."
