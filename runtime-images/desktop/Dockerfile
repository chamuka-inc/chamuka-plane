FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    tini \
    ca-certificates \
    curl \
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
ENV VNC_ENABLED=0

WORKDIR /workspace

COPY agentd /usr/local/bin/agentd
COPY start.sh /usr/local/bin/start.sh

ENTRYPOINT ["tini", "--"]
CMD ["/usr/local/bin/start.sh"]

EXPOSE 8080 6080
