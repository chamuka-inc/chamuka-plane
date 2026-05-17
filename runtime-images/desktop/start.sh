#!/usr/bin/env bash
set -euo pipefail

DISPLAY="${DISPLAY:-:99}"
RESOLUTION="${RESOLUTION:-1280x800x24}"
export DISPLAY

Xvfb "$DISPLAY" -screen 0 "$RESOLUTION" -ac +extension GLX +render -noreset &
XVFB_PID=$!

sleep 0.3

openbox &

if [[ "${VNC_ENABLED:-0}" == "1" ]]; then
    VNC_VIEWONLY="${VNC_VIEWONLY:-1}"
    VNC_FLAGS=(-display "$DISPLAY" -shared -forever -nopw -rfbport 5900)
    if [[ "$VNC_VIEWONLY" == "1" ]]; then
        VNC_FLAGS+=(-viewonly)
    fi
    x11vnc "${VNC_FLAGS[@]}" &

    websockify --web /usr/share/novnc 6080 localhost:5900 &
fi

exec agentd
