FROM node:24-bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    tini \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash sandbox || true

# Install y-sweet globally — single layer, no devDependencies
RUN npm install -g y-sweet@0.6.3 && npm cache clean --force

RUN mkdir -p /data && chown sandbox:sandbox /data

WORKDIR /workspace

COPY agentd /usr/local/bin/agentd
COPY start.sh /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh
COPY sandbox-entrypoint.sh /usr/local/bin/sandbox-entrypoint.sh
RUN chmod +x /usr/local/bin/sandbox-entrypoint.sh

ENV YSWEET_HOST=0.0.0.0
ENV YSWEET_PORT=3000
ENV YSWEET_DATA_DIR=/data

ENTRYPOINT ["tini", "--", "sandbox-entrypoint.sh"]
CMD ["start.sh"]

EXPOSE 8080 3000
