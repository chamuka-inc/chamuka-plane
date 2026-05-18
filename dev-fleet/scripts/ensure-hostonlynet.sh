#!/usr/bin/env bash
# scripts/ensure-hostonlynet.sh
#
# Ensure the VirtualBox host-only network for the dev fleet exists.
# VBox 7.x (macOS) uses the hostonlynet API; VBox 6.x uses hostonlyif.
#
# The kernel interface only materializes when a VM attaches. This
# script ensures the *definition* exists so vagrant up doesn't fail.

set -euo pipefail

NETWORK_NAME="${NETWORK_NAME:-vboxnet0}"
NETWORK_LOWER_IP="${NETWORK_LOWER_IP:-192.168.56.1}"
NETWORK_UPPER_IP="${NETWORK_UPPER_IP:-192.168.56.254}"
NETWORK_MASK="${NETWORK_MASK:-255.255.255.0}"

if ! command -v VBoxManage >/dev/null 2>&1; then
    echo "VBoxManage not found; skipping network check."
    exit 0
fi

vbox_major=$(VBoxManage --version | cut -d. -f1)

if [[ "${vbox_major}" -ge 7 ]]; then
    if VBoxManage list hostonlynets 2>/dev/null | grep -q "${NETWORK_NAME}"; then
        echo "Host-only network '${NETWORK_NAME}' already defined."
    else
        echo "Creating host-only network '${NETWORK_NAME}'..."
        VBoxManage hostonlynet add \
            --name="${NETWORK_NAME}" \
            --netmask="${NETWORK_MASK}" \
            --lower-ip="${NETWORK_LOWER_IP}" \
            --upper-ip="${NETWORK_UPPER_IP}" \
            --enable
        echo "Created."
    fi
else
    if VBoxManage list hostonlyifs 2>/dev/null | grep -q "192.168.56"; then
        echo "Host-only interface with 192.168.56.x already exists."
    else
        echo "Creating host-only interface..."
        VBoxManage hostonlyif create
        iface=$(VBoxManage list hostonlyifs \
            | awk '/^Name:/{name=$2} /IPAddress:.*0\.0\.0\.0/{print name}' \
            | tail -1)
        if [[ -n "${iface}" ]]; then
            VBoxManage hostonlyif ipconfig "${iface}" \
                --ip 192.168.56.1 --netmask 255.255.255.0
            echo "Configured ${iface} with 192.168.56.1/24."
        fi
    fi
fi
