FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    tini \
    ca-certificates \
    curl \
    iptables \
    xvfb \
    openbox \
    x11vnc \
    novnc \
    websockify \
    xdotool \
    scrot \
    chromium \
    fonts-noto \
    fonts-noto-cjk \
    fonts-noto-color-emoji \
    fonts-liberation \
    dbus-x11 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash sandbox

ENV DISPLAY=:99
ENV RESOLUTION=1280x800x24
ENV VNC_ENABLED=1
RUN echo 'CHROMIUM_FLAGS="${CHROMIUM_FLAGS} --no-sandbox"' > /etc/chromium.d/no-sandbox \
    && mkdir -p /etc/chromium/policies/managed \
    && printf '{"CommandLineFlagSecurityWarningsEnabled": false}\n' > /etc/chromium/policies/managed/sandbox-policy.json

WORKDIR /workspace

COPY start.sh /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh
COPY agentd /usr/local/bin/agentd
COPY sandbox-init /usr/local/bin/sandbox-init

ENTRYPOINT ["tini", "--", "sandbox-init", "/usr/local/bin/start.sh"]

EXPOSE 8080 6080
