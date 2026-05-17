#!/usr/bin/env bash
set -euo pipefail

DISPLAY="${DISPLAY:-:99}"
RESOLUTION="${RESOLUTION:-1280x800x24}"
export DISPLAY

Xvfb "$DISPLAY" -screen 0 "$RESOLUTION" -ac +extension GLX +render -noreset &

DISPLAY_NUM="${DISPLAY#:}"
for _ in $(seq 1 20); do
    [ -e "/tmp/.X11-unix/X${DISPLAY_NUM}" ] && break
    sleep 0.1
done

openbox &

if [[ "${VNC_ENABLED:-0}" == "1" ]]; then
    VNC_FLAGS=(-display "$DISPLAY" -shared -forever -nopw -rfbport 5900)
    if [[ "${VNC_VIEWONLY:-1}" == "1" ]]; then
        VNC_FLAGS+=(-viewonly)
    fi
    x11vnc "${VNC_FLAGS[@]}" &
    websockify --web /usr/share/novnc 6080 localhost:5900 &
fi

exec agentd
