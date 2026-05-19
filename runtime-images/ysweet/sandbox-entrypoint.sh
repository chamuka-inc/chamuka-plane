#!/bin/sh
# Entrypoint wrapper: configures egress proxy (iptables + CA cert) if
# SANDBOX_EGRESS_PROXY is set, then execs into the original command.
# Runs as part of container startup — no docker exec overhead.

if [ -n "$SANDBOX_EGRESS_PROXY" ]; then
    PROXY_HOST="${SANDBOX_EGRESS_PROXY%:*}"
    PROXY_PORT="${SANDBOX_EGRESS_PROXY##*:}"
    ADMIN_PORT=$((PROXY_PORT - 1))
    HTTPS_PORT=$((PROXY_PORT + 1))

    PROXY_IP=$(getent ahostsv4 "$PROXY_HOST" 2>/dev/null | head -1 | awk '{print $1}')
    [ -z "$PROXY_IP" ] && PROXY_IP="$PROXY_HOST"

    # Fetch CA cert and append to system bundle (fast path — avoids update-ca-certificates)
    if curl -sf "http://$PROXY_IP:$ADMIN_PORT/admin/ca.crt" -o /tmp/egress-proxy-ca.crt 2>/dev/null; then
        cat /tmp/egress-proxy-ca.crt >> /etc/ssl/certs/ca-certificates.crt 2>/dev/null || true
        # Also set env vars for runtimes that use their own cert stores
        export NODE_EXTRA_CA_CERTS=/tmp/egress-proxy-ca.crt
        export REQUESTS_CA_BUNDLE=/etc/ssl/certs/ca-certificates.crt
        export SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
        rm -f /tmp/egress-proxy-ca.crt
    fi

    iptables -t nat -A OUTPUT -p tcp --dport 80 -j DNAT --to-destination "$PROXY_IP:$PROXY_PORT" 2>/dev/null
    iptables -t nat -A OUTPUT -p tcp --dport 443 -j DNAT --to-destination "$PROXY_IP:$HTTPS_PORT" 2>/dev/null
fi

exec "$@"
